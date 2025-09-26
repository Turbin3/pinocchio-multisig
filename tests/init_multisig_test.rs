use pinocchio_multisig::helper::account_init::StateDefinition;
use pinocchio_multisig::state::{MemberState, MultisigState};
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
