use {
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_system_program as system_program,
    solana_sysvar::rent,
};

mod common;

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
            AccountMeta::new(system_program::id(), false),
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
            AccountMeta::new(system_program::id(), false),
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
            AccountMeta::new(system_program::id(), false),
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
            AccountMeta::new(system_program::id(), false),
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
            AccountMeta::new(system_program::id(), false),
        ],
        data,
    }];

    let result2 = common::build_and_send_transaction(&mut svm, &fee_payer, instruction2);
    println!("create transaction duplicate result: {:?}", result2);
    assert!(result2.is_err()); // Should fail because account is already initialized

    println!("✅ Success: Transaction creation correctly rejected already initialized account!");
}
