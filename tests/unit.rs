use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::{self, Pubkey},
    signature::Keypair,
    signer::Signer,
    system_program,
    sysvar::rent,
};

use pinocchio_multisig::state::{MultisigState, MemberState};
use pinocchio_multisig::helper::account_init::StateDefinition;
use bytemuck::Pod;

use solana_sdk::bs58;

mod common;

// #[test]
// fn test_init_multisig_no_members() {
//     let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
//     let fee_payer_pubkey = fee_payer.pubkey();

//     let min_threshold: u8 = 2;
//     let max_expiry: u64 = 1_000_000;
//     let num_members: u8 = 0; // No initial members
//     let primary_seed: u16 = 0;

//     let data = [
//         vec![0], // discriminator for InitMultisig instruction
//         max_expiry.to_le_bytes().to_vec(),
//         primary_seed.to_le_bytes().to_vec(),
//         min_threshold.to_le_bytes().to_vec(),
//         num_members.to_le_bytes().to_vec(),
//         0u8.to_le_bytes().to_vec(),
//         vec![0; 3], // 4 bytes of padding for 8-byte alignment
//     ]
//     .concat();

//     // Multisig Config PDA
//     let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
//     let seeds = &seed[..];
//     let (pda_multisig, multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

//     println!("pda_multisig acc : {:?}", pda_multisig);

//     // Treasury PDA
//     let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
//     let treasury_seeds = &treasury_seed[..];
//     let (pda_treasury, treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

//     println!("pda_treasury acc : {:?}", pda_treasury);

//     let instruction = vec![Instruction {
//         program_id: program_id,
//         accounts: vec![
//             AccountMeta::new(fee_payer.pubkey(), true),
//             AccountMeta::new(pda_multisig, false),
//             AccountMeta::new(pda_treasury, false),
//             AccountMeta::new(rent::ID, false),
//             AccountMeta::new(system_program::ID, false),
//         ],
//         data,
//     }];
//     let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

//     println!("result: {:?}", result);

//     assert!(result.is_ok());

//     // Verify the multisig state
//     let multisig_account = svm.get_account(&pda_multisig).unwrap();
//     let multisig_data = &multisig_account.data;
//     let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_data);

//     // Check counters
//     assert_eq!(multisig_state.num_members, 0);
//     assert_eq!(multisig_state.admin_counter, 0);
//     assert_eq!(multisig_state.min_threshold, min_threshold);
//     assert_eq!(multisig_state.max_expiry, max_expiry);

//     println!("✅ Success: Multisig initialized with 0 members and 0 admins!");
// }

// #[test]
// fn test_init_multisig_invalid_member_data() {
//     let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
//     let fee_payer_pubkey = fee_payer.pubkey();

//     let min_threshold: u8 = 2;
//     let max_expiry: u64 = 1_000_000;
//     let num_members: u8 = 1; // 1 member expected
//     let primary_seed: u16 = 2;

//     // Create invalid member data - 33 bytes total but first 32 bytes are not a valid pubkey
//     let mut invalid_member_bytes = [0u8; 32]; // Correct size

//     let data = [
//         vec![0],
//         max_expiry.to_le_bytes().to_vec(),
//         primary_seed.to_le_bytes().to_vec(),
//         min_threshold.to_le_bytes().to_vec(),
//         num_members.to_le_bytes().to_vec(),
//         1u8.to_le_bytes().to_vec(),
//         vec![0; 3],
//     ]
//     .concat();

//     let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
//     let seeds = &seed[..];
//     let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

//     let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
//     let treasury_seeds = &treasury_seed[..];
//     let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

//     let invalid_member_pubkey = Pubkey::new_from_array(invalid_member_bytes);

//     let instruction = vec![Instruction {
//         program_id: program_id,
//         accounts: vec![
//             AccountMeta::new(fee_payer.pubkey(), true),
//             AccountMeta::new(pda_multisig, false),
//             AccountMeta::new(pda_treasury, false),
//             AccountMeta::new(rent::ID, false),
//             AccountMeta::new(system_program::ID, false),
//             AccountMeta::new(invalid_member_pubkey, false),
//         ],
//         data,
//     }];

//     let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
//     println!("init multisig with invalid member data result: {:?}", result);

//     assert!(result.is_err(), "Expected error due to invalid member data format");

//     println!("✅ Success: Multisig correctly rejected invalid member data!");
// }

// #[test]
// fn test_init_multisig_with_members() {
//     let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
//     let fee_payer_pubkey = fee_payer.pubkey();
//     let second_admin_pubkey = second_admin.pubkey();

//     let min_threshold: u8 = 2;
//     let max_expiry: u64 = 1_000_000;
//     let num_members: u8 = 3; // 3 initial members
//     let primary_seed: u16 = 1;
//     let num_admins: u8 = 2;

//     let third_member = Keypair::new();

//     let data = [
//         vec![0], // discriminator for InitMultisig instruction
//         max_expiry.to_le_bytes().to_vec(),
//         primary_seed.to_le_bytes().to_vec(),
//         min_threshold.to_le_bytes().to_vec(),
//         num_members.to_le_bytes().to_vec(),
//         num_admins.to_le_bytes().to_vec(),
//         vec![0; 3], // 3 bytes of padding for 8-byte alignment
//     ]
//     .concat();

//     // Multisig Config PDA
//     let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
//     let seeds = &seed[..];
//     let (pda_multisig, _multisig_bump) = Pubkey::find_program_address(seeds, &program_id);

//     // Treasury PDA
//     let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
//     let treasury_seeds = &treasury_seed[..];
//     let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);

//     let instruction = vec![Instruction {
//         program_id: program_id,
//         accounts: vec![
//             AccountMeta::new(fee_payer.pubkey(), true),
//             AccountMeta::new(pda_multisig, false),
//             AccountMeta::new(pda_treasury, false),
//             AccountMeta::new(rent::ID, false),
//             AccountMeta::new(system_program::ID, false),
//             AccountMeta::new(fee_payer_pubkey, false),
//             AccountMeta::new(second_admin_pubkey, false),
//             AccountMeta::new(third_member.pubkey(), false),
//         ],
//         data,
//     }];

//     let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);
//     println!("init multisig with members result: {:?}", result);
//     assert!(result.is_ok());

//     // Verify the multisig state
//     let multisig_account = svm.get_account(&pda_multisig).unwrap();
//     let multisig_data = &multisig_account.data;

//     // Only deserialize the first MultisigState::LEN bytes
//     if multisig_data.len() < MultisigState::LEN {
//         panic!("Multisig account data too small");
//     }
//     let multisig_state_bytes = &multisig_data[..MultisigState::LEN];
//     let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_state_bytes);

//     // Check basic state
//     assert_eq!(multisig_state.num_members, num_members);
//     assert_eq!(multisig_state.admin_counter, num_admins); // 2 admins
//     assert_eq!(multisig_state.min_threshold, min_threshold);
//     assert_eq!(multisig_state.max_expiry, max_expiry);

//     let multisig_account = svm.get_account(&pda_multisig).unwrap();
//     println!("multisig account len: {}", multisig_account.data.len());
//     println!("MultisigState::LEN = {}", MultisigState::LEN);
//     println!("member bytes (hex): {}", hex::encode(&multisig_account.data[MultisigState::LEN..MultisigState::LEN+32]));
//     println!("header bytes (hex): {}", hex::encode(&multisig_account.data[..MultisigState::LEN]));

//     // Verify member organization: admins first, then normal members
//     let member_data_start = MultisigState::LEN;
//     let member_data_slice = &multisig_data[member_data_start..];

//     // First member should be admin (fee_payer)
//     let first_member_bytes = &member_data_slice[0..MemberState::LEN];
//     let first_member: &MemberState = bytemuck::from_bytes(first_member_bytes);

//     let lhs_str = bs58::encode(first_member.pubkey).into_string();
//     let rhs_str = fee_payer_pubkey.to_string(); // Pubkey already implements Display as base58

//     println!("first member pubkey lhs = {}", lhs_str);
//     println!("expected pubkey rhs = {}", rhs_str);

//     assert_eq!(first_member.pubkey, fee_payer_pubkey.to_bytes());

//     // // Second member should be admin (third_member)
//     // let second_member_bytes = &member_data_slice[MemberState::LEN..2 * MemberState::LEN];
//     // let second_member: &MemberState = bytemuck::from_bytes(second_member_bytes);
//     // assert_eq!(second_member.pubkey, third_member.pubkey().to_bytes());

//     // // Third member should be normal member (second_admin)
//     // let third_member_bytes = &member_data_slice[2 * MemberState::LEN..3 * MemberState::LEN];
//     // let third_member_state: &MemberState = bytemuck::from_bytes(third_member_bytes);
//     // assert_eq!(third_member_state.pubkey, second_admin_pubkey.to_bytes());

//     println!("✅ Success: Multisig initialized with 2 admins and 1 normal member!");
// }

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
    let proposal_seed = [b"proposal".as_ref(), pda_multisig.as_ref(), &proposal_primary_seed.to_le_bytes()];
    let (pda_proposal, proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    println!("pda_proposal acc : {:?}", pda_proposal);
    println!("proposal_bump: {}", proposal_bump);

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        proposal_primary_seed.to_le_bytes().to_vec(),
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // creator (signer)
            AccountMeta::new(pda_proposal, false),      // proposal_account (will be created)
            AccountMeta::new_readonly(pda_multisig, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    // Send proposal creation transaction
    let result =
        common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);

    println!("create proposal result: {:?}", result);

    assert!(result.is_ok());
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
    let proposal_seed = [b"proposal".as_ref(), pda_multisig.as_ref(), &proposal_primary_seed.to_le_bytes()];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary seed
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!("create proposal with uninitialized multisig result: {:?}", result);
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
    let proposal_seed = [b"proposal".as_ref(), pda_multisig.as_ref(), &proposal_primary_seed.to_le_bytes()];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary seed
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_proposal, false),
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_proposal_data,
    }];

    // Try to create the same proposal that already exists from the first test
    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!("create proposal with existing account result: {:?}", result);
    assert!(result.is_err(), "Expected error for existing proposal account");
}

#[test]
fn test_create_proposal_invalid_multisig_owner() {
    let (mut svm, fee_payer, _second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    // Create a regular account (not owned by our program) to use as "multisig"
    let fake_multisig = Keypair::new();
    svm.airdrop(&fake_multisig.pubkey(), 1000000).unwrap();

    // Use a primary seed for the proposal
    let proposal_primary_seed: u16 = 0;
    let binding = fake_multisig.pubkey();
    let proposal_seed = [b"proposal".as_ref(), binding.as_ref(), &proposal_primary_seed.to_le_bytes()];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary seed
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_proposal, false),
            AccountMeta::new_readonly(fake_multisig.pubkey(), false), // wrong owner
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
    println!("create proposal with invalid multisig owner result: {:?}", result);
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
    ].concat();

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
    let proposal_seed = [b"proposal".as_ref(), pda_multisig.as_ref(), &proposal_primary_seed.to_le_bytes()];
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
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
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
    ].concat();

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
    let (wrong_pda_proposal, _wrong_proposal_bump) = Pubkey::find_program_address(&wrong_proposal_seed, &program_id);

    // Use correct instruction data with primary seed
    let proposal_primary_seed: u16 = 0;
    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary seed
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(wrong_pda_proposal, false), // wrong PDA
            AccountMeta::new_readonly(pda_multisig, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, create_proposal_instruction);
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
    let proposal_seed = [b"proposal".as_ref(), pda_multisig.as_ref(), &proposal_primary_seed.to_le_bytes()];
    let (pda_proposal, _proposal_bump) = Pubkey::find_program_address(&proposal_seed, &program_id);

    let create_proposal_data = [
        vec![2], // discriminator (CreateProposal)
        proposal_primary_seed.to_le_bytes().to_vec(), // primary seed
    ]
    .concat();

    let create_proposal_instruction = vec![Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(second_admin.pubkey(), true), // normal member as creator (signer)
            AccountMeta::new(pda_proposal, false),         // proposal account
            AccountMeta::new_readonly(pda_multisig, false), // multisig account
            AccountMeta::new_readonly(rent::ID, false),    // rent sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: create_proposal_data,
    }];

    let result = common::build_and_send_transaction(&mut svm, &second_admin, create_proposal_instruction);
    println!("create proposal with normal member result: {:?}", result);
    assert!(result.is_err(), "Expected error for non-admin member creating proposal");
}

// #[test]
// fn test_init_transaction() {
//     let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
//     let fee_payer_pubkey = fee_payer.pubkey();

//     let data = [
//         vec![5],                     // Discriminator (1 byte) - CreateTransaction
//         0u64.to_le_bytes().to_vec(), // transaction_index: u64 (8 bytes)
//         vec![0; 512],                // tx_buffer: [u8; 512] (512 bytes)
//         0u16.to_le_bytes().to_vec(), // buffer_size: u16 (2 bytes)
//         vec![0; 6],                  // 6 bytes of padding for 8-byte alignment
//     ]
//     .concat();

//     // Transaction Config PDA
//     let seed = [(b"transaction"), fee_payer_pubkey.as_ref()];
//     let seeds = &seed[..];
//     let (pda_transaction, transaction_bump) = Pubkey::find_program_address(seeds, &program_id);

//     println!("pda_transaction acc : {:?}", pda_transaction);

//     let instruction = vec![Instruction {
//         program_id: program_id,
//         accounts: vec![
//             AccountMeta::new(fee_payer.pubkey(), true),
//             AccountMeta::new(pda_transaction, false),
//             AccountMeta::new(rent::ID, false),
//             AccountMeta::new(system_program::ID, false),
//         ],
//         data,
//     }];

//     let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

//     println!("result: {:?}", result);

//     assert!(result.is_ok());
// }

// #[test]
// fn test_init_and_update_multisig() {
//     let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
//     let fee_payer_pubkey = fee_payer.pubkey();

//     let min_threshold: u8 = 2;
//     let max_expiry: u64 = 1_000_000;
//     let num_members: u8 = 3;
//     let primary_seed: u16 = 0;

//     let data = [
//         vec![0], // discriminator for InitMultisig instruction
//         max_expiry.to_le_bytes().to_vec(),
//         primary_seed.to_le_bytes().to_vec(),
//         min_threshold.to_le_bytes().to_vec(),
//         num_members.to_le_bytes().to_vec(),
//         vec![0; 4], // 4 bytes of padding for 8-byte alignment
//     ]
//     .concat();

//     // Multisig Config PDA
//     let seed = [(b"multisig"), &primary_seed.to_le_bytes() as &[u8]];
//     let seeds = &seed[..];
//     let (pda_multisig, _) = Pubkey::find_program_address(seeds, &program_id);

//     // Treasury PDA
//     let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
//     let treasury_seeds = &treasury_seed[..];
//     let (pda_treasury, _) = Pubkey::find_program_address(treasury_seeds, &program_id);

//     let instruction = vec![Instruction {
//         program_id: program_id,
//         accounts: vec![
//             AccountMeta::new(fee_payer.pubkey(), true),
//             AccountMeta::new(pda_multisig, false),
//             AccountMeta::new(pda_treasury, false),
//             AccountMeta::new(rent::ID, false),
//             AccountMeta::new(system_program::ID, false),
//         ],
//         data,
//     }];

//     let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

//     assert!(result.is_ok());

//     let update_multisig_data = [
//         vec![1], // discriminator for UpdateMultisig instruction
//         100u64.to_le_bytes().to_vec(), // value: u64 (8 bytes)
//         1u8.to_le_bytes().to_vec(), // update_type: u8 (1 for threshold, 2 for spending limit, 3 for stale transaction index)
//         3u8.to_le_bytes().to_vec(), // threshold: u8 (1 byte)
//         vec![0; 6],
//     ]
//     .concat();

//     let update_multisig_instruction = vec![Instruction {
//         program_id: program_id,
//         accounts: vec![
//             AccountMeta::new(fee_payer.pubkey(), true),
//             AccountMeta::new(pda_multisig, false),
//         ],
//         data: update_multisig_data,
//     }];

//     let result = common::build_and_send_transaction(&mut svm, &fee_payer, update_multisig_instruction);

//     println!("result: {:?}", result);

//     assert!(result.is_ok());

//     let multisig_account = svm.get_account(&pda_multisig).unwrap();
//     let multisig_data = &multisig_account.data;

//     let multisig_state: &MultisigState = bytemuck::from_bytes(multisig_data);

//     assert_eq!(multisig_state.min_threshold, 3);
//     assert_eq!(multisig_state.max_expiry, max_expiry);

//     println!("✅ Success: Multisig threshold correctly updated to 3!");
// }
