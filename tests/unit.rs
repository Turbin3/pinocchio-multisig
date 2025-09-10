use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::{self, Pubkey},
    signer::Signer, system_program, sysvar::rent,
};

mod common;

#[test]
fn test_init_transaction() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    let data = [
        vec![5],                                // Discriminator (1 byte) - CreateTransaction
        0u64.to_le_bytes().to_vec(),            // transaction_index: u64 (8 bytes)
        vec![0; 512],                           // tx_buffer: [u8; 512] (512 bytes)
        0u16.to_le_bytes().to_vec(),            // buffer_size: u16 (2 bytes)
        vec![0; 6],                             // 6 bytes of padding for 8-byte alignment
    ]
    .concat();

    // Transaction Config PDA
    let seed = [(b"transaction"), fee_payer_pubkey.as_ref()];
    let seeds = &seed[..];
    let (pda_transaction, transaction_bump) = Pubkey::find_program_address(seeds, &program_id);

    println!("pda_transaction acc : {:?}", pda_transaction);

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_transaction, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),      
        ],
        data
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

    println!("result: {:?}", result);

    assert!(result.is_ok());
}

#[test]
fn test_init_multisig() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    let min_threshold: u8 = 2;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 3;
    let primary_seed: u16 = 0;


    let data = [
        vec![0], // discriminator for InitMultisig instruction
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        vec![0; 4],                             // 4 bytes of padding for 8-byte alignment
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
        data
    }];
    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

    println!("result: {:?}", result);

    assert!(result.is_ok());
}


    #[test]
    fn test_add_member() {
        let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

        // Step 1: Init Multisig first
        let min_threshold: u8 = 2;
        let max_expiry: u64 = 1_000_000;
        let num_members: u8 = 3;
        let primary_seed: u16 = 1;

        let init_data = [
            vec![0],
            max_expiry.to_le_bytes().to_vec(),
            primary_seed.to_le_bytes().to_vec(),
            min_threshold.to_le_bytes().to_vec(),
            num_members.to_le_bytes().to_vec(),
            vec![0; 4], // padding
        ].concat();

        let (pda_multisig, _) = Pubkey::find_program_address(
            &[b"multisig", &primary_seed.to_le_bytes()],
            &program_id,
        );

        let (pda_treasury, _) = Pubkey::find_program_address(
            &[b"treasury", pda_multisig.as_ref()],
            &program_id,
        );

        println!("pda_multisig acc : {:?}", pda_multisig);
        println!("pda_treasury acc : {:?}", pda_treasury);

        let init_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(fee_payer.pubkey(), true),
                AccountMeta::new(pda_multisig, false),
                AccountMeta::new(pda_treasury, false),
                AccountMeta::new(rent::ID, false),
                AccountMeta::new(system_program::ID, false),
            ],
            data: init_data,
        };

        let init_result = common::build_and_send_transaction(&mut svm, &fee_payer, vec![init_ix]);
        println!("Init result: {:?}", init_result);
        assert!(init_result.is_ok(), "Multisig initialization should succeed");

        // Step 2: Add Member
        let member_index: u8 = 0;
        let (pda_member, _) = Pubkey::find_program_address(
            &[b"member", pda_multisig.as_ref(), &[member_index]],
            &program_id,
        );

        println!("pda_member acc : {:?}", pda_member);

        let new_member_pubkey = second_admin.pubkey();

        let add_member_data = [
            vec![6], // AddMember discriminator - VERIFY THIS IS CORRECT
            new_member_pubkey.to_bytes().to_vec(),
        ].concat();

        let add_member_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(fee_payer.pubkey(), true),  // admin signer
                AccountMeta::new(pda_multisig, false),       // multisig account (mutable)
                AccountMeta::new(pda_member, false),         // member account (will be created)
                AccountMeta::new_readonly(rent::ID, false),  // rent sysvar
                AccountMeta::new_readonly(system_program::ID, false), // system program
            ],
            data: add_member_data,
        };

        let add_member_result = common::build_and_send_transaction(&mut svm, &fee_payer, vec![add_member_ix]);

        println!("Add member result: {:?}", add_member_result);




        assert!(add_member_result.is_ok())

    }

#[test]
fn test_remove_member() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    // Step 1: Init Multisig first
    let min_threshold: u8 = 2;
    let max_expiry: u64 = 1_000_000;
    let num_members: u8 = 3;
    let primary_seed: u16 = 1;

    let init_data = [
        vec![0],
        max_expiry.to_le_bytes().to_vec(),
        primary_seed.to_le_bytes().to_vec(),
        min_threshold.to_le_bytes().to_vec(),
        num_members.to_le_bytes().to_vec(),
        vec![0; 4], // padding
    ].concat();

    let (pda_multisig, _) = Pubkey::find_program_address(
        &[b"multisig", &primary_seed.to_le_bytes()],
        &program_id,
    );

    let (pda_treasury, _) = Pubkey::find_program_address(
        &[b"treasury", pda_multisig.as_ref()],
        &program_id,
    );

    println!("pda_multisig acc : {:?}", pda_multisig);
    println!("pda_treasury acc : {:?}", pda_treasury);

    let init_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(pda_multisig, false),
            AccountMeta::new(pda_treasury, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: init_data,
    };

    let init_result = common::build_and_send_transaction(&mut svm, &fee_payer, vec![init_ix]);
    println!("Init result: {:?}", init_result);
    assert!(init_result.is_ok(), "Multisig initialization should succeed");

    // Step 2: Add Member first (needed before we can remove)
    let member_index: u8 = 0;
    let (pda_member, _) = Pubkey::find_program_address(
        &[b"member", pda_multisig.as_ref(), &[member_index]],
        &program_id,
    );

    println!("pda_member acc : {:?}", pda_member);

    let new_member_pubkey = second_admin.pubkey();

    let add_member_data = [
        vec![6], // AddMember discriminator
        new_member_pubkey.to_bytes().to_vec(),
    ].concat();

    let add_member_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),  // admin (non-signer for now)
            AccountMeta::new(pda_multisig, false),       // multisig account (mutable)
            AccountMeta::new(pda_member, false),         // member account (will be created)
            AccountMeta::new_readonly(rent::ID, false),  // rent sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: add_member_data,
    };

    let add_member_result = common::build_and_send_transaction(&mut svm, &fee_payer, vec![add_member_ix]);
    println!("Add member result: {:?}", add_member_result);
    assert!(add_member_result.is_ok(), "Add member should succeed");

    // Step 3: Remove Member
    let remove_member_data = [
        vec![7], // RemoveMember discriminator
        new_member_pubkey.to_bytes().to_vec(), // Same member we just added
    ].concat();

    let remove_member_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),  // admin (also a signer)
            AccountMeta::new(pda_multisig, false),       // multisig account (mutable)
            AccountMeta::new(pda_member, false),         // member account (will be modified)
        ],
        data: remove_member_data,
    };

    let remove_member_result = common::build_and_send_transaction(&mut svm, &fee_payer, vec![remove_member_ix]);

    println!("Remove member result: {:?}", remove_member_result);

    match &remove_member_result {
        Ok(metadata) => {
            println!("✅ Remove member transaction SUCCEEDED!");
            println!("Signature: {}", metadata.signature);
            println!("Compute units: {}", metadata.compute_units_consumed);
            for (i, log) in metadata.logs.iter().enumerate() {
                println!("Log {}: {}", i, log);
            }
        }
        Err(e) => {
            println!("❌ Remove member transaction FAILED: {:?}", e);
        }
    }

    assert!(remove_member_result.is_ok(), "Remove member transaction should succeed");

    println!("Member removal test completed successfully!");
}



