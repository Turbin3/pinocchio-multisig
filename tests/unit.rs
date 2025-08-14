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
        vec![2],                                // Discriminator (1 byte)
        0u64.to_le_bytes().to_vec(),            // transaction_index: u64 (8 bytes)
        fee_payer_pubkey.to_bytes().to_vec(),   // tx_maker: Pubkey (32 bytes)
        vec![0; 512],                           // tx_buffer: [u8; 512] (512 bytes)
        0u16.to_le_bytes().to_vec(),            // buffer_size: u16 (2 bytes)
        vec![0; 6],                             // 6 bytes of padding at the end
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
mod common;

#[test]
fn test_init_multisig() {
    let (svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
}
