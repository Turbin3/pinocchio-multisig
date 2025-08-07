use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::{self, Pubkey},
    signer::Signer,
    system_instruction, system_program, sysvar,
};

mod common;

#[test]
fn test_execute_transaction() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();
    let fee_payer_pubkey = fee_payer.pubkey();

    let (multisig_pda, multisig_bump) =
        Pubkey::find_program_address(&[b"multisig", fee_payer_pubkey.as_ref()], &program_id);
    let (transaction_pda, _transaction_bump) =
        Pubkey::find_program_address(&[b"transaction", fee_payer_pubkey.as_ref()], &program_id);
    let (proposal_pda, _proposal_bump) =
        Pubkey::find_program_address(&[b"proposal", transaction_pda.as_ref()], &program_id);

    let lamports_to_transfer = 1_000_000_000;
    let inner_ix =
        system_instruction::transfer(&multisig_pda, &second_admin.pubkey(), lamports_to_transfer);

    let mut tx_buffer = [0u8; 512];
    let program_id_bytes = inner_ix.program_id.to_bytes();
    let data_bytes = &inner_ix.data;
    let buffer_size = program_id_bytes.len() + data_bytes.len();

    tx_buffer[..32].copy_from_slice(&program_id_bytes);
    tx_buffer[32..buffer_size].copy_from_slice(data_bytes);

    svm.airdrop(&multisig_pda, lamports_to_transfer * 2).unwrap();

    let data = [
        vec![5],                                // Discriminator (1 byte)        
    ]
    .concat();

    let instruction = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta { pubkey: fee_payer.pubkey(), is_signer: true, is_writable: true },    
            AccountMeta { pubkey: proposal_pda, is_signer: false, is_writable: false },    
            AccountMeta { pubkey: multisig_pda, is_signer: false, is_writable: true },            
            AccountMeta { pubkey: transaction_pda, is_signer: false, is_writable: false },    
            AccountMeta { pubkey: second_admin.pubkey(), is_signer: false, is_writable: true },            
            AccountMeta { pubkey: system_program::ID, is_signer: false, is_writable: false },     
        ],
        data
    }];

    let result = common::build_and_send_transaction(&mut svm, &fee_payer, instruction);

    println!("result: {:?}", result);

    assert!(result.is_ok());
}