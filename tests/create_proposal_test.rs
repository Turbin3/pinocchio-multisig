use {
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_system_program as system_program,
    solana_sysvar::{clock, rent},
};

mod common;

#[test]
fn test_create_proposal() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    println!("pda_multisig acc : {:?}", pda_multisig);
    println!("multisig_bump: {}", multisig_bump);

    let min_threshold: u8 = 2;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 0; // No initial members for this test
    let primary_seed: u16 = 0;

    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        0u8.to_le_bytes().to_vec(),
        vec![0; 3], // 3 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Multisig Config PDA
    let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    println!("pda_multisig acc : {:?}", pda_multisig);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    println!("pda_treasury acc : {:?}", pda_treasury);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::id(), false),
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
    let (pda_proposal, proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    println!("pda_proposal acc : {:?}", pda_proposal);
    println!("proposal_bump: {}", proposal_bump);

    let expiry: u64 = 1_000_000;
    let tx_type: u8 = 0;

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
            AccountMeta::new(fee_payer.pubkey(), true), // creator (signer)
            AccountMeta::new(pda_proposal, false),      // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
            AccountMeta::new_readonly(clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::id(), false), // system program
        ],
        data: create_proposal_data,
    }];

    // Send proposal creation transaction
    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);

    println!("create proposal result: {:?}", result);

    assert!(result.is_ok());

    // Verify the proposal account was created and has correct data
    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    assert!(!proposal_account.data.is_empty());

    // Read proposal state directly from bytes
    let proposal_data = &proposal_account.data;
    let proposal_id = u16::from_le_bytes([proposal_data[0], proposal_data[1]]);
    let expiry = u64::from_le_bytes([
        proposal_data[2],
        proposal_data[3],
        proposal_data[4],
        proposal_data[5],
        proposal_data[6],
        proposal_data[7],
        proposal_data[8],
        proposal_data[9],
    ]);
    let created_time = u64::from_le_bytes([
        proposal_data[10],
        proposal_data[11],
        proposal_data[12],
        proposal_data[13],
        proposal_data[14],
        proposal_data[15],
        proposal_data[16],
        proposal_data[17],
    ]);
    let status = proposal_data[18]; // ProposalStatus as u8
    let tx_type = proposal_data[19]; // ProposalType as u8
    let bump = proposal_data[20];

    // Verify proposal state fields
    assert_eq!(proposal_id, proposal_primary_seed);
    assert_eq!(expiry, expiry);
    assert_eq!(status, 0); // ProposalStatus::Draft = 0
    assert_eq!(tx_type, 0); // ProposalType::Cpi = 0
    println!("✅ Success: Proposal created with correct state data!");
}

#[test]
fn test_create_proposal_multisig_not_initialized() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    // Create a multisig PDA but don't initialize it
    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    // Use a primary seed for the proposal
    let proposal_primary_seed: u16 = 0;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let tx_type: u8 = 0;

    let create_proposal_data = [
        vec![2],                                      // discriminator (CreateProposal)
        0u64.to_le_bytes().to_vec(),                  // expiry: u64 (8 bytes)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary_seed: u16 (2 bytes)
        tx_type.to_le_bytes().to_vec(),               // tx_type: u8 (1 byte)
        vec![0; 5],
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!(
        "create proposal with uninitialized multisig result: {:?}",
        result
    );
    assert!(result.is_err(), "Expected error for uninitialized multisig");
}

#[test]
fn test_create_proposal_account_already_exists() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();

    // Use the same multisig and proposal from the first test
    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    // Use the same proposal primary seed as the first test
    let proposal_primary_seed: u16 = 1;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let tx_type: u8 = 0;

    let create_proposal_data = [
        vec![2],                                      // discriminator (CreateProposal)
        0u64.to_le_bytes().to_vec(),                  // expiry: u64 (8 bytes)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary_seed: u16 (2 bytes)
        tx_type.to_le_bytes().to_vec(),               // tx_type: u8 (1 byte)
        vec![0; 5],
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(clock::ID, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: create_proposal_data,
    }];

    // Try to create the same proposal that already exists from the first test
    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!("create proposal with existing account result: {:?}", result);
    assert!(
        result.is_err(),
        "Expected error for existing proposal account"
    );
}

#[test]
fn test_create_proposal_invalid_multisig_owner() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    // Create a regular account (not owned by program) to use as "multisig"
    let fake_multisig = Keypair::new();
    svm.airdrop(&fake_multisig.pubkey(), 1000000).unwrap();

    // Use a primary seed for the proposal
    let proposal_primary_seed: u16 = 0;
    let binding = fake_multisig.pubkey();
    let proposal_seed = [
        b"proposal".as_ref(),
        binding.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let tx_type: u8 = 0;

    let create_proposal_data = [
        vec![2],                                      // discriminator (CreateProposal)
        0u64.to_le_bytes().to_vec(),                  // expiry: u64 (8 bytes)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary_seed: u16 (2 bytes)
        tx_type.to_le_bytes().to_vec(),               // tx_type: u8 (1 byte)
        vec![0; 5],
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_proposal, false),
            AccountMeta::new_readonly(fake_multisig.pubkey(), false), // wrong owner
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(clock::ID, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!(
        "create proposal with invalid multisig owner result: {:?}",
        result
    );
    assert!(result.is_err(), "Expected error for invalid multisig owner");
}

#[test]
fn test_create_proposal_invalid_instruction_data() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    let multisig_seed = [(b"multisig"), &3u16.to_le_bytes() as &[u8]];
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(&treasury_seed, &program_id);

    let init_data = [
        vec![0], // discriminator
        1_000_000u64.to_le_bytes().to_vec(),
        3u16.to_le_bytes().to_vec(),
        1u8.to_le_bytes().to_vec(),
        0u8.to_le_bytes().to_vec(),
        0u8.to_le_bytes().to_vec(),
        vec![0; 3],
    ]
    .concat();

    let init_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::id(), false),
        ],
        data: init_data,
    }];

    let init_result = common::build_and_send_transaction(&mut svm, &fee_payer, init_instruction);
    assert!(init_result.is_ok(), "Failed to initialize multisig");

    // Use a primary seed for the proposal
    let proposal_primary_seed: u16 = 0;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    // Test with malformed data (too short - only discriminator, missing primary seed)
    let create_proposal_data = vec![2]; // Only discriminator, missing primary seed

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(clock::ID, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!("create proposal with empty data result: {:?}", result);
    assert!(result.is_err(), "Expected error for empty instruction data");
}

#[test]
fn test_create_proposal_wrong_proposal_pda() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    let multisig_seed = [(b"multisig"), &4u16.to_le_bytes() as &[u8]];
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(&treasury_seed, &program_id);

    let init_data = [
        vec![0], // discriminator
        1_000_000u64.to_le_bytes().to_vec(),
        4u16.to_le_bytes().to_vec(),
        1u8.to_le_bytes().to_vec(),
        0u8.to_le_bytes().to_vec(),
        0u8.to_le_bytes().to_vec(),
        vec![0; 3],
    ]
    .concat();

    let init_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::id(), false),
        ],
        data: init_data,
    }];

    let init_result = common::build_and_send_transaction(&mut svm, &fee_payer, init_instruction);
    assert!(init_result.is_ok(), "Failed to initialize multisig");

    // Use wrong seeds for proposal PDA (missing primary seed)
    let wrong_proposal_seed = [b"proposal".as_ref(), pda_multisig.as_ref()]; // Missing primary seed
    let (wrong_pda_proposal, _wrong_proposal_bump) =
        Pubkey::find_program_address(&wrong_proposal_seed, &program_id);

    // Use correct instruction data with primary seed
    let proposal_primary_seed: u16 = 0;
    let tx_type: u8 = 0;

    let create_proposal_data = [
        vec![2],                                      // discriminator (CreateProposal)
        0u64.to_le_bytes().to_vec(),                  // expiry: u64 (8 bytes)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary_seed: u16 (2 bytes)
        tx_type.to_le_bytes().to_vec(),               // tx_type: u8 (1 byte)
        vec![0; 5],
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(wrong_pda_proposal, false), // wrong PDA
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(clock::ID, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!("create proposal with wrong PDA result: {:?}", result);
    assert!(result.is_err(), "Expected error for wrong proposal PDA");
}

#[test]
fn test_create_proposal_non_admin_member() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let multisig_seed = [(b"multisig"), &5u16.to_le_bytes() as &[u8]];
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);

    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(&treasury_seed, &program_id);

    // Initialize multisig with 1 admin and 1 normal member
    let min_threshold: u8 = 1;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 2; // 2 initial members
    let primary_seed: u16 = 5;
    let num_admins: u8 = 1; // 1 admin

    let admin_member = Keypair::new();

    let init_data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        num_admins.to_le_bytes().to_vec(),
        vec![0; 3], // 3 bytes of padding for 8-byte alignment
    ]
    .concat();

    let init_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::id(), false),
            AccountMeta::new(admin_member.pubkey(), false), // admin member
            AccountMeta::new(second_admin.pubkey(), false), // normal member
        ],
        data: init_data,
    }];

    let init_result = common::build_and_send_transaction(&mut svm, &fee_payer, init_instruction);
    assert!(init_result.is_ok(), "Failed to initialize multisig");

    // Try to create proposal with the normal member (second_admin) - should fail
    let proposal_primary_seed: u16 = 0;
    let proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let tx_type: u8 = 0;

    let create_proposal_data = [
        vec![2],                                      // discriminator (CreateProposal)
        0u64.to_le_bytes().to_vec(),                  // expiry: u64 (8 bytes)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary_seed: u16 (2 bytes)
        tx_type.to_le_bytes().to_vec(),               // tx_type: u8 (1 byte)
        vec![0; 5],
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin.pubkey(), true), // normal member as creator (signer)
            AccountMeta::new(pda_proposal, false),         // proposal account
            AccountMeta::new_readonly(pda_multisig, false), // multisig account
            AccountMeta::new_readonly(rent::ID, false),    // rent sysvar
            AccountMeta::new_readonly(clock::ID, false),
            AccountMeta::new_readonly(system_program::id(), false), // system program
        ],
        data: create_proposal_data,
    }];

    let result =
        common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    println!("create proposal with normal member result: {:?}", result);
    assert!(
        result.is_err(),
        "Expected error for non-admin member creating proposal"
    );
}
