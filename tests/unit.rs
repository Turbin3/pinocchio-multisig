use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::{self, Pubkey},
    signature::Keypair,
    signer::Signer,
    system_program,
    sysvar::rent,
};

use bytemuck::Pod;
use pinocchio::account_info::AccountInfo;
use pinocchio_multisig::{helper::account_init::StateDefinition, instructions::VoteIxData};
use pinocchio_multisig::{
    helper::to_bytes,
    state::{
        proposal::{ProposalState, ProposalStatus, ProposalType},
        MemberState, MultisigState,
    },
    instructions::{UpdateMemberIxData, UpdateMultisigIxData},
};

use solana_sdk::bs58;

mod common;

#[test]
fn test_init_multisig_no_members() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    let min_threshold: u8 = 2;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 0; // No initial members
    let primary_seed: u16 = 0;

    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        0u8.to_le_bytes().to_vec(),
        vec![0; 3], // 4 bytes of padding for 8-byte alignment
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
            AccountMeta::new(system_program::ID, false),
        ],
        data,
    }];
    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

    println!("result: {:?}", result);

    assert!(result.is_ok());

    // Verify the multisig state
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_data);

    // Check counters
    assert_eq!(multisig_state.num_members, 0);
    assert_eq!(multisig_state.admin_counter, 0);
    assert_eq!(multisig_state.min_threshold, min_threshold);
    assert_eq!(multisig_state.max_expiry, max_expiry);

    println!("✅ Success: Multisig initialized with 0 members and 0 admins!");
}

#[test]
fn test_init_multisig_with_members() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();
    let second_admin_pubkey = second_admin.pubkey();

    let min_threshold: u8 = 2;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 3; // 3 initial members
    let primary_seed: u16 = 1;
    let num_admins: u8 = 2;

    let third_member = Keypair::new();
    let fourth_member = Keypair::new();

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
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
            AccountMeta::new(second_admin_pubkey, false),
            AccountMeta::new(third_member.pubkey(), false),
            AccountMeta::new(fourth_member.pubkey(), false),
        ],
        data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    println!("init multisig with members result: {:?}", result);
    assert!(result.is_ok());

    // Verify the multisig state
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;

    // Only deserialize the first MultisigState::LEN bytes
    if multisig_data.len() < MultisigState::LEN {
        panic!("Multisig account data too small");
    }
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    // Check basic state
    assert_eq!(multisig_state.num_members, num_members);
    assert_eq!(multisig_state.admin_counter, num_admins); // 2 admins
    assert_eq!(multisig_state.min_threshold, min_threshold);
    assert_eq!(multisig_state.max_expiry, max_expiry);

    // Verify member organization: admins first, then normal members
    let member_data_start = MultisigState::LEN;
    let member_data_slice = multisig_data.split_at(member_data_start);

    let first_member_bytes = &member_data_slice.1[..MemberState::LEN];
    let first_member: &MemberState = bytemuck::from_bytes(first_member_bytes);
    assert_eq!(first_member.pubkey, second_admin_pubkey.to_bytes());

    let second_member_bytes = &member_data_slice.1[MemberState::LEN..2 * MemberState::LEN];
    let second_member: &MemberState = bytemuck::from_bytes(second_member_bytes);
    assert_eq!(second_member.pubkey, third_member.pubkey().to_bytes());

    let third_member_bytes = &member_data_slice.1[2 * MemberState::LEN..3 * MemberState::LEN];
    let third_member_state: &MemberState = bytemuck::from_bytes(third_member_bytes);
    assert_eq!(third_member_state.pubkey, fourth_member.pubkey().to_bytes());

    println!("✅ Success: Multisig initialized with 2 admins and 1 normal member!");
}

#[test]
fn test_init_multisig_all_admins() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();
    let second_admin_pubkey = second_admin.pubkey();

    let min_threshold: u8 = 2;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 2; // 2 members
    let primary_seed: u16 = 2;
    let num_admins: u8 = 2; // All members are admins

    let third_member = Keypair::new();

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
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
            AccountMeta::new(second_admin_pubkey, false),
            AccountMeta::new(third_member.pubkey(), false),
        ],
        data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    assert!(result.is_ok());

    // Verify the multisig state
    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.num_members, num_members);
    assert_eq!(multisig_state.admin_counter, num_admins);
    assert_eq!(multisig_state.min_threshold, min_threshold);
    assert_eq!(multisig_state.max_expiry, max_expiry);

    println!("✅ Success: Multisig initialized with all members as admins!");
}

#[test]
fn test_init_multisig_invalid_data() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();

    let min_threshold: u8 = 2;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 1; // 1 member
    let primary_seed: u16 = 4;
    let num_admins: u8 = 2; // 2 admins - INVALID: more admins than members

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
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    assert!(result.is_err()); // Should fail because num_admins > num_members

    println!("✅ Success: Multisig initialization correctly rejected invalid data!");
}

#[test]
fn test_init_multisig_account_already_initialized() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();

    let min_threshold: u8 = 1;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 0;
    let primary_seed: u16 = 5;

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
    let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

    // Treasury PDA
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

    // First initialization - should succeed
    let instruction1 = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: data.clone(),
    }];

    let result1 = common::build_and_send_transaction(&mut svm, &fee_payer, instruction1);
    assert!(result1.is_ok());

    // Second initialization - should fail
    let instruction2 = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data,
    }];

    let result2 = common::build_and_send_transaction(&mut svm, &fee_payer, instruction2);
    assert!(result2.is_err()); // Should fail because account is already initialized

    println!("✅ Success: Multisig initialization correctly rejected already initialized account!");
}

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
            AccountMeta::new(system_program::ID, false),
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
        tx_type.to_le_bytes().to_vec(), // tx_type: u8 (1 byte)
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
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
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
        tx_type.to_le_bytes().to_vec(), // tx_type: u8 (1 byte)
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
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false),
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
        tx_type.to_le_bytes().to_vec(), // tx_type: u8 (1 byte)
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
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
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
        tx_type.to_le_bytes().to_vec(), // tx_type: u8 (1 byte)
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
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
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
            AccountMeta::new(system_program::ID, false),
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
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
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
            AccountMeta::new(system_program::ID, false),
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
        tx_type.to_le_bytes().to_vec(), // tx_type: u8 (1 byte)
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
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
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
            AccountMeta::new(system_program::ID, false),
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
        tx_type.to_le_bytes().to_vec(), // tx_type: u8 (1 byte)
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
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false),
            AccountMeta::new_readonly(system_program::ID, false), // system program
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

#[test]
fn test_create_transaction() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();

    let transaction_index: u64 = 1;
    let primary_seed: u16 = 10;
    let buffer_size: u16 = 100;
    let mut tx_buffer = [0u8; 512];
    tx_buffer[..100].copy_from_slice(&[1u8; 100]); // Fill first 100 bytes with 1s

    let data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let seed = [(b"transaction"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    println!("create transaction result: {:?}", result);
    assert!(result.is_ok());

    // Verify the transaction account was created
    let transaction_account = svm.get_account(&pda_transaction).unwrap();
    assert!(!transaction_account.data.is_empty());

    // Read transaction state directly from bytes
    let transaction_data = &transaction_account.data;
    let tx_index = u64::from_le_bytes([
        transaction_data[0],
        transaction_data[1],
        transaction_data[2],
        transaction_data[3],
        transaction_data[4],
        transaction_data[5],
        transaction_data[6],
        transaction_data[7],
    ]);
    let buf_size = u16::from_le_bytes([transaction_data[8], transaction_data[9]]);
    let bump = transaction_data[522]; // bump is at offset 8 + 2 + 512 = 522

    // Verify transaction state fields
    assert_eq!(tx_index, transaction_index);
    assert_eq!(buf_size, buffer_size);
    // Verify first 100 bytes of buffer are 1s
    for i in 0..100 {
        assert_eq!(transaction_data[10 + i], 1u8);
    }
    // Verify remaining buffer bytes are 0s
    for i in 100..512 {
        assert_eq!(transaction_data[10 + i], 0u8);
    }

    println!("✅ Success: Transaction created with correct state data!");
}

#[test]
fn test_create_transaction_max_buffer() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();

    let transaction_index: u64 = 2;
    let primary_seed: u16 = 20;
    let buffer_size: u16 = 512; // Full buffer size
    let mut tx_buffer = [0u8; 512];
    tx_buffer.fill(0xFF); // Fill entire buffer with 0xFF

    let data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let seed = [(b"transaction"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    println!("create transaction max buffer result: {:?}", result);
    assert!(result.is_ok());

    // Verify the transaction account was created
    let transaction_account = svm.get_account(&pda_transaction).unwrap();
    assert!(!transaction_account.data.is_empty());

    println!("✅ Success: Transaction with max buffer created successfully!");
}

#[test]
fn test_create_transaction_empty_buffer() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();

    let transaction_index: u64 = 3;
    let primary_seed: u16 = 30;
    let buffer_size: u16 = 0; // Empty buffer
    let tx_buffer = [0u8; 512]; // All zeros

    let data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let seed = [(b"transaction"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(seeds, &program_id);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
    println!("create transaction empty buffer result: {:?}", result);
    assert!(result.is_ok());

    // Verify the transaction account was created
    let transaction_account = svm.get_account(&pda_transaction).unwrap();
    assert!(!transaction_account.data.is_empty());

    println!("✅ Success: Transaction with empty buffer created successfully!");
}

#[test]
fn test_create_transaction_account_already_initialized() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();

    let transaction_index: u64 = 4;
    let primary_seed: u16 = 40;
    let buffer_size: u16 = 50;
    let mut tx_buffer = [0u8; 512];
    tx_buffer[..50].copy_from_slice(&[0xABu8; 50]); // Fill first 50 bytes with 0xAB

    let data = [
        vec![4], // discriminator for CreateTransaction instruction
        transaction_index.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        tx_buffer.to_vec(),
        buffer_size.to_le_bytes().to_vec(),
        vec![0; 4], // 4 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction PDA
    let seed = [(b"transaction"), &primary_seed.to_le_bytes() as &[u8]];
    let seeds = &seed[..];
    let (pda_transaction, _) = Pubkey::find_program_address(seeds, &program_id);

    // First creation - should succeed
    let instruction1 = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: data.clone(),
    }];

    let result1 = common::build_and_send_transaction(&mut svm, &fee_payer, instruction1);
    assert!(result1.is_ok());

    // Second creation - should fail
    let instruction2 = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data,
    }];

    let result2 = common::build_and_send_transaction(&mut svm, &fee_payer, instruction2);
    println!("create transaction duplicate result: {:?}", result2);
    assert!(result2.is_err()); // Should fail because account is already initialized

    println!("✅ Success: Transaction creation correctly rejected already initialized account!");
}

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
        tx_type.to_le_bytes().to_vec(), // tx_type: u8 (1 byte)
        vec![0; 5], // 5 bytes of padding for 8-byte alignment (total 16 bytes)
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin_pubkey, true), // creator (signer)
            AccountMeta::new(pda_proposal, false),      // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
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
    let transaction_seed = [(b"transaction"), &transaction_primary_seed.to_le_bytes() as &[u8]];
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMember instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)            
            AccountMeta::new(pda_multisig, false), // multisig
            AccountMeta::new(pda_proposal, false), // proposal
            AccountMeta::new(pda_transaction, false), // transaction
            AccountMeta::new(rent::ID, false), // rent for add_member
            AccountMeta::new(system_program::ID, false), // system program for add_member
        ],
        data: execute_transaction_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
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
    let (pda_remove_proposal, _remove_proposal_bump) = Pubkey::find_program_address(&remove_proposal_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_remove_proposal_instruction);
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

    let remove_transaction_seed = [(b"transaction"), &remove_transaction_primary_seed.to_le_bytes() as &[u8]];
    let (pda_remove_transaction, _) = Pubkey::find_program_address(&remove_transaction_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_remove_transaction_instruction);
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_remove_transaction_instruction);
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
    let (pda_add_admin_proposal, _add_admin_proposal_bump) = Pubkey::find_program_address(&add_admin_proposal_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_add_admin_proposal_instruction);
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

    let add_admin_transaction_seed = [(b"transaction"), &add_admin_transaction_primary_seed.to_le_bytes() as &[u8]];
    let (pda_add_admin_transaction, _) = Pubkey::find_program_address(&add_admin_transaction_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_add_admin_transaction_instruction);
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_add_admin_transaction_instruction);
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
    let (pda_add_normal_proposal, _add_normal_proposal_bump) = Pubkey::find_program_address(&add_normal_proposal_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_add_normal_proposal_instruction);
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

    let add_normal_transaction_seed = [(b"transaction"), &add_normal_transaction_primary_seed.to_le_bytes() as &[u8]];
    let (pda_add_normal_transaction, _) = Pubkey::find_program_address(&add_normal_transaction_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_add_normal_transaction_instruction);
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_add_normal_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Normal member added before admin removal test");

    // Test removing the first admin to verify admin removal logic
    let remove_first_admin_proposal_primary_seed: u16 = 6;
    let remove_first_admin_proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &remove_first_admin_proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_remove_first_admin_proposal, _remove_first_admin_proposal_bump) = Pubkey::find_program_address(&remove_first_admin_proposal_seed, &program_id);

    let remove_first_admin_expiry: u64 = 3758794966; // Feb 09 2089
    let remove_first_admin_tx_type: u8 = 1; // UpdateMember

    let create_remove_first_admin_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        remove_first_admin_expiry.to_le_bytes().to_vec(),
        remove_first_admin_proposal_primary_seed.to_le_bytes().to_vec(),
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

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_remove_first_admin_proposal_instruction);
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
    remove_first_admin_tx_buffer[32..32 + remove_first_admin_bytes.len()].copy_from_slice(&remove_first_admin_bytes);
    let remove_first_admin_buffer_size = (32 + remove_first_admin_bytes.len()) as u16;

    let create_remove_first_admin_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        remove_first_admin_transaction_index.to_le_bytes().to_vec(),
        remove_first_admin_transaction_primary_seed.to_le_bytes().to_vec(),
        remove_first_admin_tx_buffer.to_vec(),
        remove_first_admin_buffer_size.to_le_bytes().to_vec(),
        vec![0; 4],
    ]
    .concat();

    let remove_first_admin_transaction_seed = [(b"transaction"), &remove_first_admin_transaction_primary_seed.to_le_bytes() as &[u8]];
    let (pda_remove_first_admin_transaction, _) = Pubkey::find_program_address(&remove_first_admin_transaction_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_remove_first_admin_transaction_instruction);
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_remove_first_admin_transaction_instruction);
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
    println!("Normal member at position 2: {:?}", normal_member_state.pubkey);
    println!("✅ Normal member correctly left-shifted from position 3 to position 2");
    
    println!("✅ Admin removal logic verified: last admin swapped to first position, normal members left-shifted once");

    // Test removing another admin to verify the logic works consistently
    let remove_fifth_member_proposal_primary_seed: u16 = 18;
    let remove_fifth_member_proposal_seed = [
        b"proposal".as_ref(),
        pda_multisig.as_ref(),
        &remove_fifth_member_proposal_primary_seed.to_le_bytes(),
    ];
    let (pda_remove_fifth_member_proposal, _remove_fifth_member_proposal_bump) = Pubkey::find_program_address(&remove_fifth_member_proposal_seed, &program_id);

    let remove_fifth_member_expiry: u64 = 3758794966; // Feb 09 2089
    let remove_fifth_member_tx_type: u8 = 1; // UpdateMember

    let create_remove_fifth_member_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        remove_fifth_member_expiry.to_le_bytes().to_vec(),
        remove_fifth_member_proposal_primary_seed.to_le_bytes().to_vec(),
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

    let result = common::build_and_send_transaction(&mut svm, &fourth_member, create_remove_fifth_member_proposal_instruction);
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
    remove_fifth_member_tx_buffer[32..32 + remove_fifth_member_bytes.len()].copy_from_slice(&remove_fifth_member_bytes);
    let remove_fifth_member_buffer_size = (32 + remove_fifth_member_bytes.len()) as u16;

    let create_remove_fifth_member_transaction_data = [
        vec![4], // discriminator for CreateTransaction instruction
        remove_fifth_member_transaction_index.to_le_bytes().to_vec(),
        remove_fifth_member_transaction_primary_seed.to_le_bytes().to_vec(),
        remove_fifth_member_tx_buffer.to_vec(),
        remove_fifth_member_buffer_size.to_le_bytes().to_vec(),
        vec![0; 4],
    ]
    .concat();

    let remove_fifth_member_transaction_seed = [(b"transaction"), &remove_fifth_member_transaction_primary_seed.to_le_bytes() as &[u8]];
    let (pda_remove_fifth_member_transaction, _) = Pubkey::find_program_address(&remove_fifth_member_transaction_seed, &program_id);

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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_remove_fifth_member_transaction_instruction);
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_remove_fifth_member_transaction_instruction);
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
    println!("Final Normal Member: {:?}", final_normal_member_state.pubkey);

    // Verify that the remaining admin (fourth_member) is in position 0
    assert_eq!(final_admin_state.pubkey, fourth_member.pubkey().to_bytes());
    
    // Verify that the normal member (sixth_member) is in position 1 (left-shifted again)
    assert_eq!(final_normal_member_state.pubkey, sixth_member.pubkey().to_bytes());
    
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
            AccountMeta::new(pda_proposal, false), // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    assert!(result.is_ok());

    // Create transaction with UpdateMultisig instruction data for threshold update
    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 10;
    
    // Prepare UpdateMultisig instruction data for threshold update
    let update_multisig_data = UpdateMultisigIxData {
        value: 0, // not used for threshold update
        update_type: 1, // 1 for update threshold
        threshold: 3, // new threshold value
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
    let transaction_seed = [(b"transaction"), &transaction_primary_seed.to_le_bytes() as &[u8]];
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMultisig threshold instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)            
            AccountMeta::new(pda_multisig, false), // multisig
            AccountMeta::new(pda_proposal, false), // proposal
            AccountMeta::new(pda_transaction, false), // transaction
            AccountMeta::new(rent::ID, false), // rent
            AccountMeta::new(system_program::ID, false), // system program
        ],
        data: execute_transaction_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
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
            AccountMeta::new(pda_proposal, false), // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    assert!(result.is_ok());

    // Create transaction with UpdateMultisig instruction data for spending limit update
    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 20;
    
    // Prepare UpdateMultisig instruction data for spending limit update
    let update_multisig_data = UpdateMultisigIxData {
        value: 1_000_000_000, // 1 SOL spending limit
        update_type: 2, // 2 for update spending limit
        threshold: 0, // not used for spending limit update
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
    let transaction_seed = [(b"transaction"), &transaction_primary_seed.to_le_bytes() as &[u8]];
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMultisig spending limit instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)            
            AccountMeta::new(pda_multisig, false), // multisig
            AccountMeta::new(pda_proposal, false), // proposal
            AccountMeta::new(pda_transaction, false), // transaction
            AccountMeta::new(rent::ID, false), // rent
            AccountMeta::new(system_program::ID, false), // system program
        ],
        data: execute_transaction_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
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
            AccountMeta::new(pda_proposal, false), // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    assert!(result.is_ok());

    // Create transaction with UpdateMultisig instruction data for stale transaction index update
    let transaction_index: u64 = 0;
    let transaction_primary_seed: u16 = 30;
    
    // Prepare UpdateMultisig instruction data for stale transaction index update
    let update_multisig_data = UpdateMultisigIxData {
        value: 100, // new stale transaction index
        update_type: 3, // 3 for update stale transaction index
        threshold: 0, // not used for stale transaction index update
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
    let transaction_seed = [(b"transaction"), &transaction_primary_seed.to_le_bytes() as &[u8]];
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with UpdateMultisig stale transaction index instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)            
            AccountMeta::new(pda_multisig, false), // multisig
            AccountMeta::new(pda_proposal, false), // proposal
            AccountMeta::new(pda_transaction, false), // transaction
            AccountMeta::new(rent::ID, false), // rent
            AccountMeta::new(system_program::ID, false), // system program
        ],
        data: execute_transaction_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, execute_transaction_instruction);
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
    
    svm.airdrop(&source_account.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&destination_account.pubkey(), 100_000_000).unwrap();

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
            AccountMeta::new(pda_proposal, false), // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
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
    let transaction_seed = [(b"transaction"), &transaction_primary_seed.to_le_bytes() as &[u8]];
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

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_transaction_instruction);
    assert!(result.is_ok());
    println!("✅ Transaction created with CPI instruction");

    // Execute the transaction
    let execute_transaction_data = vec![5]; // discriminator for ExecuteTransaction instruction

    let execute_transaction_instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // payer (signer)            
            AccountMeta::new(pda_multisig, false), // multisig
            AccountMeta::new(pda_proposal, false), // proposal
            AccountMeta::new(pda_transaction, false), // transaction
            AccountMeta::new(rent::ID, false), // rent
            AccountMeta::new(system_program::ID, false), // system program
            AccountMeta::new(source_account.pubkey(), true), // source account (signer)
            AccountMeta::new(destination_account.pubkey(), false), // destination account
        ],
        data: execute_transaction_data,
    }];

    // Get initial balances for verification
    let initial_source_balance = svm.get_account(&source_account.pubkey()).unwrap().lamports;
    let initial_dest_balance = svm.get_account(&destination_account.pubkey()).unwrap().lamports;
    
    let result = common::build_and_send_transaction_multisig(
        &mut svm,
        &fee_payer,
        execute_transaction_instruction,
        &[&source_account], // additional signer for CPI
    );
    println!("Execute CPI transaction result: {:?}", result);
    
    assert!(result.is_ok(), "CPI transaction should succeed with System Program");
    println!("✅ CPI transaction executed successfully!");
    
    let final_source_balance = svm.get_account(&source_account.pubkey()).unwrap().lamports;
    let final_dest_balance = svm.get_account(&destination_account.pubkey()).unwrap().lamports;
    
    assert!(final_source_balance < initial_source_balance, "Source balance should have decreased");
    assert!(final_dest_balance > initial_dest_balance, "Destination balance should have increased");
    
    let transferred_amount = initial_source_balance - final_source_balance;
    assert_eq!(transferred_amount, 50_000_000, "Should have transferred exactly 0.05 SOL");
    
    println!("✅ System Program transfer via CPI verified: {} lamports transferred", transferred_amount);

    let multisig_account = svm.get_account(&pda_multisig).unwrap();
    let multisig_data = &multisig_account.data;
    let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
    let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

    assert_eq!(multisig_state.transaction_index, 1);
    println!("✅ Success: CPI transaction processed and transaction index updated!");
}
