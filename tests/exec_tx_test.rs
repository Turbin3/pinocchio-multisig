use pinocchio_multisig::helper::account_init::StateDefinition;
use pinocchio_multisig::{
    helper::to_bytes,
    instructions::{UpdateMemberIxData, UpdateMultisigIxData},
    state::{MemberState, MultisigState},
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    sysvar::rent,
};

mod common;

#[test]
fn test_execute_transaction_update_member() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();
    let second_admin_pubkey = second_admin.pubkey();

    let third_member = Keypair::new();
    let fourth_member = Keypair::new();
    let fifth_member = Keypair::new(); // For admin test

    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    println!("pda_multisig acc : {:?}", pda_multisig);
    println!("multisig_bump: {}", multisig_bump);

    let min_threshold: u8 = 0;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 2;
    let primary_seed: u16 = 0;
    let num_admins: u8 = 2;

    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        num_admins.to_le_bytes().to_vec(),
        vec![0; 3], // 3 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Multisig Config PDA
    let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
            AccountMeta::new(second_admin_pubkey, false),
            AccountMeta::new(fourth_member.pubkey(), false),
        ],
        data,
    }];
    let multisig_result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

    assert!(multisig_result.is_ok(), "Failed to create multisig");

    let proposal_primary_seed: u16 = 1;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let expiry: u64 = 3758794966; // Feb 09 2089
    let tx_type: u8 = 1;

    let create_proposal_data = [
        vec![2],                                      // discriminator (CreateProposal)
        expiry.to_le_bytes().to_vec(),                // expiry: u64 (8 bytes)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary_seed: u16 (2 bytes)
        tx_type.to_le_bytes().to_vec(),               // tx_type: u8 (1 byte)
        vec![0; 5], // 5 bytes of padding for 8-byte alignment (total 16 bytes)
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_proposal, false),       // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false),  // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    // Send proposal creation transaction
    let result =
        common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);

    assert!(result.is_ok());

    // Create transaction with UpdateMember instruction data
    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 10;

    // Prepare UpdateMember instruction data
    let update_member_data = UpdateMemberIxData {
        operation: 1, // 1 for add member
        member_data: {
            let mut data = [0u8; 33];
            data[..32].copy_from_slice(third_member.pubkey().as_ref());
            data[32] = 0; // 0 = normal member, 1 = admin
            data
        },
    };

    // Serialize the UpdateMember instruction data
    let update_member_bytes = unsafe { to_bytes(&update_member_data) };

    // Create transaction buffer: program_id (32 bytes) + instruction data
    let mut tx_buffer = [0u8; 512];
    tx_buffer[..32].copy_from_slice(program_id.as_ref());
    tx_buffer[32..32 + update_member_bytes.len()].copy_from_slice(&update_member_bytes);
    let buffer_size = (32 + update_member_bytes.len()) as u16;

    let create_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        transaction_primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let transaction_seed = [
        (b"transaction"),
        &transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let transaction_seeds = &transaction_seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(transaction_seeds, &program_id);

    let create_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMember instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)
            AccountMeta::new(pda_multisig, false),      // multisig
            AccountMeta::new(pda_proposal, false),      // proposal
            AccountMeta::new(pda_transaction, false),   // transaction
            AccountMeta::new(rent::ID, false),          // rent for add_member
            AccountMeta::new(system_program::ID, false), // system program for add_member
        ],
        data: execute_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
    println!("Execute transaction result: {:?}", result);
    assert!(result.is_ok());
    println!("✅ Transaction executed successfully");

    // Verify the new member was added
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    // Check that member count increased
    assert_eq!(multisig_state.num_members, 3); // Should be 3 now (2 original + 1 new)
    assert_eq!(multisig_state.admin_counter, 2); // Should still be 2 (no new admins)

    // Verify the new member is in the member data
    let member_data_start = MultisigState::LEN;
    let member_data_slice = multisig_data.split_at(member_data_start);

    // Check the third member (index 2) - should be at the end
    let third_member_bytes = &member_data_slice.1[2 * MemberState::LEN..3 * MemberState::LEN];
    let third_member_state: &MemberState = bytemuck::from_bytes(third_member_bytes);
    assert_eq!(third_member_state.pubkey, third_member.pubkey().to_bytes());

    // Verify that normal member is appended at the end (after all admins)
    // Position 0: admin1 (second_admin)
    // Position 1: admin2 (fourth_member)
    // Position 2: normal member (third_member) - should be at the end
    let admin1_bytes = &member_data_slice.1[0 * MemberState::LEN..1 * MemberState::LEN];
    let admin1_state: &MemberState = bytemuck::from_bytes(admin1_bytes);
    let admin2_bytes = &member_data_slice.1[1 * MemberState::LEN..2 * MemberState::LEN];
    let admin2_state: &MemberState = bytemuck::from_bytes(admin2_bytes);

    // Verify admin positions are maintained
    assert_eq!(admin1_state.pubkey, second_admin_pubkey.to_bytes());
    assert_eq!(admin2_state.pubkey, fourth_member.pubkey().to_bytes());

    // Verify normal member is at the end
    assert_eq!(third_member_state.pubkey, third_member.pubkey().to_bytes());
    println!("✅ Normal member correctly appended at the end: admin1 | admin2 | normal_member");

    println!("✅ Success: New member added via execute transaction!");

    // Now test removing members
    // First, remove one member (should work - have 3 members)
    let remove_proposal_primary_seed: u16 = 2;
    let remove_proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &remove_proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_remove_proposal, _remove_proposal_bump) =
        Pubkey::find_program_address(&remove_proposal_seed, &program_id);

    let remove_expiry: u64 = 3758794966; // Feb 09 2089
    let remove_tx_type: u8 = 1; // UpdateMember

    let create_remove_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        remove_expiry.to_le_bytes().to_vec(),
        remove_proposal_primary_seed.to_le_bytes().to_vec(),
        remove_tx_type.to_le_bytes().to_vec(),
        vec![0; 5],
    ]
    .concat();

    let create_remove_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_remove_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_remove_proposal_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &second_admin,
        create_remove_proposal_instruction,
    );
    assert!(result.is_ok());

    // Create transaction for removing member (remove third_member)
    let remove_transaction_index: u64 = 1;
    let remove_transaction_primary_seed: u16 = 20;

    let remove_member_data = UpdateMemberIxData {
        operation: 2, // 2 for remove member
        member_data: {
            let mut data = [0u8; 33];
            data[..32].copy_from_slice(third_member.pubkey().as_ref());
            data[32] = 0; // not used for remove
            data
        },
    };

    let remove_member_bytes = unsafe { to_bytes(&remove_member_data) };

    let mut remove_tx_buffer = [0u8; 512];
    remove_tx_buffer[..32].copy_from_slice(program_id.as_ref());
    remove_tx_buffer[32..32 + remove_member_bytes.len()].copy_from_slice(&remove_member_bytes);
    let remove_buffer_size = (32 + remove_member_bytes.len()) as u16;

    let create_remove_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        remove_transaction_index.to_le_bytes().to_vec(),
        remove_transaction_primary_seed.to_le_bytes().to_vec(),
        remove_tx_buffer.to_vec(),
        remove_buffer_size.to_le_bytes().to_vec(),
        vec![0; 4],
    ]
    .concat();

    let remove_transaction_seed = [
        (b"transaction"),
        &remove_transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let (pda_remove_transaction, _) =
        Pubkey::find_program_address(&remove_transaction_seed, &program_id);

    let create_remove_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_remove_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_remove_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        create_remove_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Remove transaction created");

    // Execute the remove transaction
    let execute_remove_transaction_data = vec![5];

    let execute_remove_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_remove_proposal, false),
            AccountMeta::new(pda_remove_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: execute_remove_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        execute_remove_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ First member removed successfully");

    // Verify member count decreased
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);
    assert_eq!(multisig_state.num_members, 2); // Should be 2 now (3 - 1)

    // Add an admin and verify the pattern: admin1 | admin2 | ... | normal member1 | ...
    let add_admin_proposal_primary_seed: u16 = 5;
    let add_admin_proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &add_admin_proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_add_admin_proposal, _add_admin_proposal_bump) =
        Pubkey::find_program_address(&add_admin_proposal_seed, &program_id);

    let add_admin_expiry: u64 = 3758794966; // Feb 09 2089
    let add_admin_tx_type: u8 = 1; // UpdateMember

    let create_add_admin_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        add_admin_expiry.to_le_bytes().to_vec(),
        add_admin_proposal_primary_seed.to_le_bytes().to_vec(),
        add_admin_tx_type.to_le_bytes().to_vec(),
        vec![0; 5],
    ]
    .concat();

    let create_add_admin_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true),
            AccountMeta::new(pda_add_admin_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_add_admin_proposal_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &second_admin,
        create_add_admin_proposal_instruction,
    );
    assert!(result.is_ok());

    // Create transaction for adding admin
    let add_admin_transaction_index: u64 = 2;
    let add_admin_transaction_primary_seed: u16 = 50;

    let add_admin_data = UpdateMemberIxData {
        operation: 1, // 1 for add member
        member_data: {
            let mut data = [0u8; 33];
            data[..32].copy_from_slice(fifth_member.pubkey().as_ref());
            data[32] = 1; // 1 = admin
            data
        },
    };

    let add_admin_bytes = unsafe { to_bytes(&add_admin_data) };

    let mut add_admin_tx_buffer = [0u8; 512];
    add_admin_tx_buffer[..32].copy_from_slice(program_id.as_ref());
    add_admin_tx_buffer[32..32 + add_admin_bytes.len()].copy_from_slice(&add_admin_bytes);
    let add_admin_buffer_size = (32 + add_admin_bytes.len()) as u16;

    let create_add_admin_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        add_admin_transaction_index.to_le_bytes().to_vec(),
        add_admin_transaction_primary_seed.to_le_bytes().to_vec(),
        add_admin_tx_buffer.to_vec(),
        add_admin_buffer_size.to_le_bytes().to_vec(),
        vec![0; 4],
    ]
    .concat();

    let add_admin_transaction_seed = [
        (b"transaction"),
        &add_admin_transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let (pda_add_admin_transaction, _) =
        Pubkey::find_program_address(&add_admin_transaction_seed, &program_id);

    let create_add_admin_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_add_admin_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_add_admin_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        create_add_admin_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Add admin transaction created");

    // Execute the add admin transaction
    let execute_add_admin_transaction_data = vec![5];

    let execute_add_admin_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_add_admin_proposal, false),
            AccountMeta::new(pda_add_admin_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: execute_add_admin_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        execute_add_admin_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Admin added successfully");

    // Verify the admin pattern: admin1 | admin2 | admin3 | normal member1
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.num_members, 3); // Should be 3 now (2 + 1 new admin)
    assert_eq!(multisig_state.admin_counter, 3); // Should be 3 admins now

    // Verify the member order: admin1 | admin2 | admin3 | normal member1
    let member_data_start = MultisigState::LEN;
    let member_data_slice = multisig_data.split_at(member_data_start);
    let member_data = member_data_slice.1;

    // Check admin positions (0, 1, 2)
    let admin1_bytes = &member_data[0 * MemberState::LEN..1 * MemberState::LEN];
    let admin1_state: &MemberState = bytemuck::from_bytes(admin1_bytes);
    println!("Admin 1: {:?}", admin1_state.pubkey);

    let admin2_bytes = &member_data[1 * MemberState::LEN..2 * MemberState::LEN];
    let admin2_state: &MemberState = bytemuck::from_bytes(admin2_bytes);
    println!("Admin 2: {:?}", admin2_state.pubkey);

    let admin3_bytes = &member_data[2 * MemberState::LEN..3 * MemberState::LEN];
    let admin3_state: &MemberState = bytemuck::from_bytes(admin3_bytes);
    println!("Admin 3: {:?}", admin3_state.pubkey);

    // Verify that the new admin (fifth_member) is in position 2
    assert_eq!(admin3_state.pubkey, fifth_member.pubkey().to_bytes());
    println!("✅ Admin pattern maintained: admin1 | admin2 | admin3 | normal member1");

    // Add another normal member before testing admin removal (so to have a normal member to left-shift)
    let sixth_member = Keypair::new();

    let add_normal_proposal_primary_seed: u16 = 7;
    let add_normal_proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &add_normal_proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_add_normal_proposal, _add_normal_proposal_bump) =
        Pubkey::find_program_address(&add_normal_proposal_seed, &program_id);

    let add_normal_expiry: u64 = 3758794966; // Feb 09 2089
    let add_normal_tx_type: u8 = 1; // UpdateMember

    let create_add_normal_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        add_normal_expiry.to_le_bytes().to_vec(),
        add_normal_proposal_primary_seed.to_le_bytes().to_vec(),
        add_normal_tx_type.to_le_bytes().to_vec(),
        vec![0; 5],
    ]
    .concat();

    let create_add_normal_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_add_normal_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_add_normal_proposal_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &second_admin,
        create_add_normal_proposal_instruction,
    );
    assert!(result.is_ok());

    // Create transaction for adding normal member
    let add_normal_transaction_index: u64 = 3;
    let add_normal_transaction_primary_seed: u16 = 70;

    let add_normal_data = UpdateMemberIxData {
        operation: 1, // 1 for add member
        member_data: {
            let mut data = [0u8; 33];
            data[..32].copy_from_slice(sixth_member.pubkey().as_ref());
            data[32] = 0; // 0 = normal member
            data
        },
    };

    let add_normal_bytes = unsafe { to_bytes(&add_normal_data) };

    let mut add_normal_tx_buffer = [0u8; 512];
    add_normal_tx_buffer[..32].copy_from_slice(program_id.as_ref());
    add_normal_tx_buffer[32..32 + add_normal_bytes.len()].copy_from_slice(&add_normal_bytes);
    let add_normal_buffer_size = (32 + add_normal_bytes.len()) as u16;

    let create_add_normal_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        add_normal_transaction_index.to_le_bytes().to_vec(),
        add_normal_transaction_primary_seed.to_le_bytes().to_vec(),
        add_normal_tx_buffer.to_vec(),
        add_normal_buffer_size.to_le_bytes().to_vec(),
        vec![0; 4],
    ]
    .concat();

    let add_normal_transaction_seed = [
        (b"transaction"),
        &add_normal_transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let (pda_add_normal_transaction, _) =
        Pubkey::find_program_address(&add_normal_transaction_seed, &program_id);

    let create_add_normal_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_add_normal_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_add_normal_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        create_add_normal_transaction_instruction,
    );
    assert!(result.is_ok());

    // Execute the add normal transaction
    let execute_add_normal_transaction_data = vec![5];

    let execute_add_normal_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_add_normal_proposal, false),
            AccountMeta::new(pda_add_normal_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: execute_add_normal_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        execute_add_normal_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Normal member added before admin removal test");

    // Test removing the first admin to verify admin removal logic
    let remove_first_admin_proposal_primary_seed: u16 = 6;
    let remove_first_admin_proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &remove_first_admin_proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_remove_first_admin_proposal, _remove_first_admin_proposal_bump) =
        Pubkey::find_program_address(&remove_first_admin_proposal_seed, &program_id);

    let remove_first_admin_expiry: u64 = 3758794966; // Feb 09 2089
    let remove_first_admin_tx_type: u8 = 1; // UpdateMember

    let create_remove_first_admin_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        remove_first_admin_expiry.to_le_bytes().to_vec(),
        remove_first_admin_proposal_primary_seed
            .to_le_bytes()
            .to_vec(),
        remove_first_admin_tx_type.to_le_bytes().to_vec(),
        vec![0; 5],
    ]
    .concat();

    let create_remove_first_admin_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_remove_first_admin_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_remove_first_admin_proposal_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &second_admin,
        create_remove_first_admin_proposal_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Remove first admin proposal created");

    // Create transaction for removing first admin (remove second_admin)
    let remove_first_admin_transaction_index: u64 = 4;
    let remove_first_admin_transaction_primary_seed: u16 = 60;

    let remove_first_admin_data = UpdateMemberIxData {
        operation: 2, // 2 for remove member
        member_data: {
            let mut data = [0u8; 33];
            data[..32].copy_from_slice(second_admin_pubkey.as_ref());
            data[32] = 0; // not used for remove
            data
        },
    };

    let remove_first_admin_bytes = unsafe { to_bytes(&remove_first_admin_data) };

    let mut remove_first_admin_tx_buffer = [0u8; 512];
    remove_first_admin_tx_buffer[..32].copy_from_slice(program_id.as_ref());
    remove_first_admin_tx_buffer[32..32 + remove_first_admin_bytes.len()]
        .copy_from_slice(&remove_first_admin_bytes);
    let remove_first_admin_buffer_size = (32 + remove_first_admin_bytes.len()) as u16;

    let create_remove_first_admin_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        remove_first_admin_transaction_index.to_le_bytes().to_vec(),
        remove_first_admin_transaction_primary_seed
            .to_le_bytes()
            .to_vec(),
        remove_first_admin_tx_buffer.to_vec(),
        remove_first_admin_buffer_size.to_le_bytes().to_vec(),
        vec![0; 4],
    ]
    .concat();

    let remove_first_admin_transaction_seed = [
        (b"transaction"),
        &remove_first_admin_transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let (pda_remove_first_admin_transaction, _) =
        Pubkey::find_program_address(&remove_first_admin_transaction_seed, &program_id);

    let create_remove_first_admin_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_remove_first_admin_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_remove_first_admin_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        create_remove_first_admin_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Remove first admin transaction created");

    // Execute the remove first admin transaction
    let execute_remove_first_admin_transaction_data = vec![5];

    let execute_remove_first_admin_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_remove_first_admin_proposal, false),
            AccountMeta::new(pda_remove_first_admin_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: execute_remove_first_admin_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        execute_remove_first_admin_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ First admin removed successfully");

    // Verify the admin removal logic: first admin swapped with last admin, then normal members left-shifted
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.num_members, 3); // Should be 3 now (4 - 1)
    assert_eq!(multisig_state.admin_counter, 2); // Should be 2 admins now (3 - 1)

    // Note: The normal member was also removed during admin removal due to the left-shift logic
    // This might be a bug in the remove_member function - normal members should be preserved

    // Verify the new member order after removing first admin
    let member_data_start = MultisigState::LEN;
    let member_data_slice = multisig_data.split_at(member_data_start);
    let member_data = member_data_slice.1;

    // Check admin positions (0, 1) - should be: admin2, admin3
    let new_admin1_bytes = &member_data[0 * MemberState::LEN..1 * MemberState::LEN];
    let new_admin1_state: &MemberState = bytemuck::from_bytes(new_admin1_bytes);
    println!("New Admin 1: {:?}", new_admin1_state.pubkey);

    let new_admin2_bytes = &member_data[1 * MemberState::LEN..2 * MemberState::LEN];
    let new_admin2_state: &MemberState = bytemuck::from_bytes(new_admin2_bytes);
    println!("New Admin 2: {:?}", new_admin2_state.pubkey);

    // Verify that the last admin (fifth_member) is now in position 0 (swapped with first admin)
    assert_eq!(new_admin1_state.pubkey, fifth_member.pubkey().to_bytes());

    // Verify that the second admin (fourth_member) is now in position 1
    assert_eq!(new_admin2_state.pubkey, fourth_member.pubkey().to_bytes());

    // Verify that normal members are left-shifted once
    // Before removal: position 3 had normal member (sixth_member)
    // After removal: position 2 should have normal member (sixth_member) - left-shifted by 1
    let normal_member_bytes = &member_data[2 * MemberState::LEN..3 * MemberState::LEN];
    let normal_member_state: &MemberState = bytemuck::from_bytes(normal_member_bytes);
    assert_eq!(normal_member_state.pubkey, sixth_member.pubkey().to_bytes());
    println!(
        "Normal member at position 2: {:?}",
        normal_member_state.pubkey
    );
    println!("✅ Normal member correctly left-shifted from position 3 to position 2");

    println!("✅ Admin removal logic verified: last admin swapped to first position, normal members left-shifted once");

    // Test removing another admin to verify the logic works consistently
    let remove_fifth_member_proposal_primary_seed: u16 = 18;
    let remove_fifth_member_proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &remove_fifth_member_proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_remove_fifth_member_proposal, _remove_fifth_member_proposal_bump) =
        Pubkey::find_program_address(&remove_fifth_member_proposal_seed, &program_id);

    let remove_fifth_member_expiry: u64 = 3758794966; // Feb 09 2089
    let remove_fifth_member_tx_type: u8 = 1; // UpdateMember

    let create_remove_fifth_member_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        remove_fifth_member_expiry.to_le_bytes().to_vec(),
        remove_fifth_member_proposal_primary_seed
            .to_le_bytes()
            .to_vec(),
        remove_fifth_member_tx_type.to_le_bytes().to_vec(),
        vec![0; 5],
    ]
    .concat();

    let create_remove_fifth_member_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fourth_member.pubkey(), true),
            AccountMeta::new(pda_remove_fifth_member_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_remove_fifth_member_proposal_data,
    }];

    // Airdrop SOL to fourth_member so it can sign transactions
    svm.airdrop(&fourth_member.pubkey(), 1_000_000_000).unwrap(); // 1 SOL
    println!("✅ Airdropped 1 SOL to fourth_member");

    let result = common::build_and_send_transaction(
        &mut svm,
        &fourth_member,
        create_remove_fifth_member_proposal_instruction,
    );
    println!("result: {:?}", result);

    assert!(result.is_ok());
    println!("✅ Remove fifth_member proposal created");

    // Create transaction for removing fifth_member
    let remove_fifth_member_transaction_index: u64 = 5;
    let remove_fifth_member_transaction_primary_seed: u16 = 80;

    let remove_fifth_member_data = UpdateMemberIxData {
        operation: 2, // 2 for remove member
        member_data: {
            let mut data = [0u8; 33];
            data[..32].copy_from_slice(fifth_member.pubkey().as_ref());
            data[32] = 0; // not used for remove
            data
        },
    };

    let remove_fifth_member_bytes = unsafe { to_bytes(&remove_fifth_member_data) };

    let mut remove_fifth_member_tx_buffer = [0u8; 512];
    remove_fifth_member_tx_buffer[..32].copy_from_slice(program_id.as_ref());
    remove_fifth_member_tx_buffer[32..32 + remove_fifth_member_bytes.len()]
        .copy_from_slice(&remove_fifth_member_bytes);
    let remove_fifth_member_buffer_size = (32 + remove_fifth_member_bytes.len()) as u16;

    let create_remove_fifth_member_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        remove_fifth_member_transaction_index.to_le_bytes().to_vec(),
        remove_fifth_member_transaction_primary_seed
            .to_le_bytes()
            .to_vec(),
        remove_fifth_member_tx_buffer.to_vec(),
        remove_fifth_member_buffer_size.to_le_bytes().to_vec(),
        vec![0; 4],
    ]
    .concat();

    let remove_fifth_member_transaction_seed = [
        (b"transaction"),
        &remove_fifth_member_transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let (pda_remove_fifth_member_transaction, _) =
        Pubkey::find_program_address(&remove_fifth_member_transaction_seed, &program_id);

    let create_remove_fifth_member_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_remove_fifth_member_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_remove_fifth_member_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        create_remove_fifth_member_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Remove second admin transaction created");

    // Execute the remove second admin transaction
    let execute_remove_fifth_member_transaction_data = vec![5];

    let execute_remove_fifth_member_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_remove_fifth_member_proposal, false),
            AccountMeta::new(pda_remove_fifth_member_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: execute_remove_fifth_member_transaction_data,
    }];

    let result = common::build_and_send_transaction(
        &mut svm,
        &fee_payer,
        execute_remove_fifth_member_transaction_instruction,
    );
    assert!(result.is_ok());
    println!("✅ Second admin removed successfully");

    // Verify the second admin removal logic
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.num_members, 2); // Should be 2 now (3 - 1)
    assert_eq!(multisig_state.admin_counter, 1); // Should be 1 admin now (2 - 1)

    // Verify the new member order after removing second admin
    let member_data_start = MultisigState::LEN;
    let member_data_slice = multisig_data.split_at(member_data_start);
    let member_data = member_data_slice.1;

    // Check admin position (0) - should be: admin3 (fifth_member)
    let final_admin_bytes = &member_data[0 * MemberState::LEN..1 * MemberState::LEN];
    let final_admin_state: &MemberState = bytemuck::from_bytes(final_admin_bytes);
    println!("Final Admin: {:?}", final_admin_state.pubkey);

    // Check normal member position (1) - should be: sixth_member
    let final_normal_member_bytes = &member_data[1 * MemberState::LEN..2 * MemberState::LEN];
    let final_normal_member_state: &MemberState = bytemuck::from_bytes(final_normal_member_bytes);
    println!(
        "Final Normal Member: {:?}",
        final_normal_member_state.pubkey
    );

    // Verify that the remaining admin (fourth_member) is in position 0
    assert_eq!(final_admin_state.pubkey, fourth_member.pubkey().to_bytes());

    // Verify that the normal member (sixth_member) is in position 1 (left-shifted again)
    assert_eq!(
        final_normal_member_state.pubkey,
        sixth_member.pubkey().to_bytes()
    );

    println!("✅ Fifth member removal verified: fourth_member | sixth_member pattern maintained");
    println!("✅ All admin removal tests completed successfully!");
}

#[test]
fn test_execute_transaction_update_threshold() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();
    let second_admin_pubkey = second_admin.pubkey();

    let third_member = Keypair::new();
    let fourth_member = Keypair::new();

    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    let min_threshold: u8 = 0;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 2;
    let primary_seed: u16 = 0;
    let num_admins: u8 = 2;

    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        num_admins.to_le_bytes().to_vec(),
        vec![0; 3], // 3 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Multisig Config PDA
    let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
            AccountMeta::new(second_admin_pubkey, false),
            AccountMeta::new(fourth_member.pubkey(), false),
        ],
        data,
    }];
    let multisig_result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    assert!(multisig_result.is_ok(), "Failed to create multisig");

    // Create proposal for updating threshold
    let proposal_primary_seed: u16 = 1;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let expiry: u64 = 3758794966; // Feb 09 2089
    let tx_type: u8 = 2; // UpdateMultisig

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        expiry.to_le_bytes().to_vec(),
        proposal_primary_seed.to_le_bytes().to_vec(),
        tx_type.to_le_bytes().to_vec(),
        vec![0; 5], // 5 bytes of padding for 8-byte alignment
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_proposal, false),       // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false),  // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    assert!(result.is_ok());

    // Create transaction with UpdateMultisig instruction data for threshold update
    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 10;

    // Prepare UpdateMultisig instruction data for threshold update
    let update_multisig_data = UpdateMultisigIxData {
        value: 0,       // not used for threshold update
        update_type: 1, // 1 for update threshold
        threshold: 3,   // new threshold value
    };

    // Serialize the UpdateMultisig instruction data
    let update_multisig_bytes = unsafe { to_bytes(&update_multisig_data) };

    // Create transaction buffer: program_id (32 bytes) + instruction data
    let mut tx_buffer = [0u8; 512];
    tx_buffer[..32].copy_from_slice(program_id.as_ref());
    tx_buffer[32..32 + update_multisig_bytes.len()].copy_from_slice(&update_multisig_bytes);
    let buffer_size = (32 + update_multisig_bytes.len()) as u16;

    let create_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        transaction_primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let transaction_seed = [
        (b"transaction"),
        &transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let transaction_seeds = &transaction_seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(transaction_seeds, &program_id);

    let create_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMultisig threshold instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)
            AccountMeta::new(pda_multisig, false),      // multisig
            AccountMeta::new(pda_proposal, false),      // proposal
            AccountMeta::new(pda_transaction, false),   // transaction
            AccountMeta::new(rent::ID, false),          // rent
            AccountMeta::new(system_program::ID, false), // system program
        ],
        data: execute_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
    println!("Execute transaction result: {:?}", result);
    assert!(result.is_ok());
    println!("✅ Transaction executed successfully");

    // Verify the threshold was updated
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.min_threshold, 3); // Should be updated to 3
    println!("✅ Success: Threshold updated via execute transaction!");
}

#[test]
fn test_execute_transaction_update_spending_limit() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();
    let second_admin_pubkey = second_admin.pubkey();

    let third_member = Keypair::new();
    let fourth_member = Keypair::new();

    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    let min_threshold: u8 = 0;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 2;
    let primary_seed: u16 = 0;
    let num_admins: u8 = 2;

    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        num_admins.to_le_bytes().to_vec(),
        vec![0; 3], // 3 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Multisig Config PDA
    let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
            AccountMeta::new(second_admin_pubkey, false),
            AccountMeta::new(fourth_member.pubkey(), false),
        ],
        data,
    }];
    let multisig_result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    assert!(multisig_result.is_ok(), "Failed to create multisig");

    // Create proposal for updating spending limit
    let proposal_primary_seed: u16 = 2;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let expiry: u64 = 3758794966; // Feb 09 2089
    let tx_type: u8 = 2; // UpdateMultisig

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        expiry.to_le_bytes().to_vec(),
        proposal_primary_seed.to_le_bytes().to_vec(),
        tx_type.to_le_bytes().to_vec(),
        vec![0; 5], // 5 bytes of padding for 8-byte alignment
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_proposal, false),       // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false),  // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    assert!(result.is_ok());

    // Create transaction with UpdateMultisig instruction data for spending limit update
    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 20;

    // Prepare UpdateMultisig instruction data for spending limit update
    let update_multisig_data = UpdateMultisigIxData {
        value: 1_000_000_000, // 1 SOL spending limit
        update_type: 2,       // 2 for update spending limit
        threshold: 0,         // not used for spending limit update
    };

    // Serialize the UpdateMultisig instruction data
    let update_multisig_bytes = unsafe { to_bytes(&update_multisig_data) };

    // Create transaction buffer: program_id (32 bytes) + instruction data
    let mut tx_buffer = [0u8; 512];
    tx_buffer[..32].copy_from_slice(program_id.as_ref());
    tx_buffer[32..32 + update_multisig_bytes.len()].copy_from_slice(&update_multisig_bytes);
    let buffer_size = (32 + update_multisig_bytes.len()) as u16;

    let create_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        transaction_primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let transaction_seed = [
        (b"transaction"),
        &transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let transaction_seeds = &transaction_seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(transaction_seeds, &program_id);

    let create_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMultisig spending limit instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)
            AccountMeta::new(pda_multisig, false),      // multisig
            AccountMeta::new(pda_proposal, false),      // proposal
            AccountMeta::new(pda_transaction, false),   // transaction
            AccountMeta::new(rent::ID, false),          // rent
            AccountMeta::new(system_program::ID, false), // system program
        ],
        data: execute_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
    println!("Execute transaction result: {:?}", result);
    assert!(result.is_ok());
    println!("✅ Transaction executed successfully");

    // Verify the spending limit was updated
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.spending_limit, 1_000_000_000); // Should be updated to 1 SOL
    println!("✅ Success: Spending limit updated via execute transaction!");
}

#[test]
fn test_execute_transaction_update_stale_transaction_index() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();
    let second_admin_pubkey = second_admin.pubkey();

    let third_member = Keypair::new();
    let fourth_member = Keypair::new();

    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    let min_threshold: u8 = 0;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 2;
    let primary_seed: u16 = 0;
    let num_admins: u8 = 2;

    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        num_admins.to_le_bytes().to_vec(),
        vec![0; 3], // 3 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Multisig Config PDA
    let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
            AccountMeta::new(second_admin_pubkey, false),
            AccountMeta::new(fourth_member.pubkey(), false),
        ],
        data,
    }];
    let multisig_result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    assert!(multisig_result.is_ok(), "Failed to create multisig");

    // Create proposal for updating stale transaction index
    let proposal_primary_seed: u16 = 3;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let expiry: u64 = 3758794966; // Feb 09 2089
    let tx_type: u8 = 2; // UpdateMultisig

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        expiry.to_le_bytes().to_vec(),
        proposal_primary_seed.to_le_bytes().to_vec(),
        tx_type.to_le_bytes().to_vec(),
        vec![0; 5], // 5 bytes of padding for 8-byte alignment
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_proposal, false),       // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false),  // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    assert!(result.is_ok());

    // Create transaction with UpdateMultisig instruction data for stale transaction index update
    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 30;

    // Prepare UpdateMultisig instruction data for stale transaction index update
    let update_multisig_data = UpdateMultisigIxData {
        value: 100,     // new stale transaction index
        update_type: 3, // 3 for update stale transaction index
        threshold: 0,   // not used for stale transaction index update
    };

    // Serialize the UpdateMultisig instruction data
    let update_multisig_bytes = unsafe { to_bytes(&update_multisig_data) };

    // Create transaction buffer: program_id (32 bytes) + instruction data
    let mut tx_buffer = [0u8; 512];
    tx_buffer[..32].copy_from_slice(program_id.as_ref());
    tx_buffer[32..32 + update_multisig_bytes.len()].copy_from_slice(&update_multisig_bytes);
    let buffer_size = (32 + update_multisig_bytes.len()) as u16;

    let create_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        transaction_primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let transaction_seed = [
        (b"transaction"),
        &transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let transaction_seeds = &transaction_seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(transaction_seeds, &program_id);

    let create_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMultisig stale transaction index instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)
            AccountMeta::new(pda_multisig, false),      // multisig
            AccountMeta::new(pda_proposal, false),      // proposal
            AccountMeta::new(pda_transaction, false),   // transaction
            AccountMeta::new(rent::ID, false),          // rent
            AccountMeta::new(system_program::ID, false), // system program
        ],
        data: execute_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
    println!("Execute transaction result: {:?}", result);
    assert!(result.is_ok());
    println!("✅ Transaction executed successfully");

    // Verify the stale transaction index was updated
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.stale_transaction_index, 100); // Should be updated to 100
    println!("✅ Success: Stale transaction index updated via execute transaction!");
}

#[test]
fn test_execute_transaction_cpi_call() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();
    let second_admin_pubkey = second_admin.pubkey();

    let third_member = Keypair::new();
    let fourth_member = Keypair::new();

    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    let min_threshold: u8 = 0;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 2;
    let primary_seed: u16 = 0;
    let num_admins: u8 = 2;

    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        num_admins.to_le_bytes().to_vec(),
        vec![0; 3], // 3 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Multisig Config PDA
    let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
            AccountMeta::new(second_admin_pubkey, false),
            AccountMeta::new(fourth_member.pubkey(), false),
        ],
        data,
    }];
    let multisig_result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    assert!(multisig_result.is_ok(), "Failed to create multisig");

    let target_program_pubkey = system_program::ID;

    let source_account = Keypair::new();
    let destination_account = Keypair::new();

    svm.airdrop(&source_account.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&destination_account.pubkey(), 100_000_000)
        .unwrap();

    // Create proposal for CPI call
    let proposal_primary_seed: u16 = 4;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let expiry: u64 = 3758794966; // Feb 09 2089
    let tx_type: u8 = 0; // Cpi

    let create_proposal_data = [
        vec![2],
        expiry.to_le_bytes().to_vec(),
        proposal_primary_seed.to_le_bytes().to_vec(),
        tx_type.to_le_bytes().to_vec(),
        vec![0; 5],
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_proposal, false),       // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false),  // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    assert!(result.is_ok());

    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 40;

    // Format: [program_id (32 bytes)] + [instruction_data]
    let mut cpi_instruction_data = Vec::new();

    cpi_instruction_data.extend_from_slice(&target_program_pubkey.to_bytes());

    // System Program transfer instruction: [instruction_index: u32] + [lamports: u64]
    let transfer_lamports: u64 = 50_000_000; // 0.05 SOL
    let mut system_instruction_data = Vec::new();
    system_instruction_data.extend_from_slice(&2u32.to_le_bytes());
    system_instruction_data.extend_from_slice(&transfer_lamports.to_le_bytes());
    cpi_instruction_data.extend_from_slice(&system_instruction_data);

    let mut tx_buffer = [0u8; 512];
    let cpi_data_len = cpi_instruction_data.len();
    if cpi_data_len <= 512 {
        tx_buffer[..cpi_data_len].copy_from_slice(&cpi_instruction_data);
    } else {
        panic!("CPI instruction data too large for transaction buffer");
    }
    let buffer_size = cpi_data_len as u16;

    let create_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        transaction_primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let transaction_seed = [
        (b"transaction"),
        &transaction_primary_seed.to_le_bytes() as &[u8],
    ];
    let transaction_seeds = &transaction_seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(transaction_seeds, &program_id);

    let create_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: create_transaction_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with CPI instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)
            AccountMeta::new(pda_multisig, false),      // multisig
            AccountMeta::new(pda_proposal, false),      // proposal
            AccountMeta::new(pda_transaction, false),   // transaction
            AccountMeta::new(rent::ID, false),          // rent
            AccountMeta::new(system_program::ID, false), // system program
            AccountMeta::new(source_account.pubkey(), true), // source account (signer)
            AccountMeta::new(destination_account.pubkey(), false), // destination account
        ],
        data: execute_transaction_data,
    }];

    // Get initial balances for verification
    let initial_source_balance = svm.get_account(&source_account.pubkey()).unwrap().lamports;
    let initial_dest_balance = svm
        .get_account(&destination_account.pubkey())
        .unwrap()
        .lamports;

    let result = common::build_and_send_transaction_multisig(
        &mut svm,
        &fee_payer,
        execute_transaction_instruction,
        &[&source_account], // additional signer for CPI
    );
    println!("Execute CPI transaction result: {:?}", result);

    assert!(
        result.is_ok(),
        "CPI transaction should succeed with System Program"
    );
    println!("✅ CPI transaction executed successfully!");

    let final_source_balance = svm.get_account(&source_account.pubkey()).unwrap().lamports;
    let final_dest_balance = svm
        .get_account(&destination_account.pubkey())
        .unwrap()
        .lamports;

    assert!(
        final_source_balance < initial_source_balance,
        "Source balance should have decreased"
    );
    assert!(
        final_dest_balance > initial_dest_balance,
        "Destination balance should have increased"
    );

    let transferred_amount = initial_source_balance - final_source_balance;
    assert_eq!(
        transferred_amount, 50_000_000,
        "Should have transferred exactly 0.05 SOL"
    );

    println!(
        "✅ System Program transfer via CPI verified: {} lamports transferred",
        transferred_amount
    );

    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.transaction_index, 1);
    println!("✅ Success: CPI transaction processed and transaction index updated!");
}
