use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use pinocchio_multisig::{
    helper::StateDefinition,
    instructions::{CreateProposalIxData, VoteIxData},
    state::{ProposalState, ProposalType},
    ID,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    sysvar::rent,
    transaction::VersionedTransaction,
};

use pinocchio_multisig::helper::utils::to_bytes;
use pinocchio_multisig::instructions::InitMultisigIxData;

pub fn setup_svm_and_program() -> (LiteSVM, Keypair, Keypair, Pubkey) {
    let mut svm = LiteSVM::new();
    let fee_payer = Keypair::new();

    svm.airdrop(&fee_payer.pubkey(), 100000000).unwrap();

    let program_id = Pubkey::from(ID);
    svm.add_program_from_file(program_id, "./target/deploy/pinocchio_multisig.so")
        .unwrap();

    let second_keypair = Keypair::new();
    svm.airdrop(&second_keypair.pubkey(), 1000000000).unwrap();

    (svm, fee_payer, second_keypair, program_id)
}

pub fn build_and_send_transaction(
    svm: &mut LiteSVM,
    fee_payer: &Keypair,
    instruction: Vec<Instruction>,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let msg = v0::Message::try_compile(
        &fee_payer.pubkey(),
        &instruction,
        &[],
        svm.latest_blockhash(),
    )
    .unwrap();

    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();

    svm.send_transaction(tx)
}

pub fn build_and_send_transaction_multisig(
    svm: &mut LiteSVM,
    fee_payer: &Keypair,
    instructions: Vec<Instruction>,
    additional_signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let msg = v0::Message::try_compile(
        &fee_payer.pubkey(),
        &instructions,
        &[],
        svm.latest_blockhash(),
    )
    .unwrap();

    let mut all_signers = Vec::with_capacity(1 + additional_signers.len());
    all_signers.push(fee_payer);
    all_signers.extend_from_slice(additional_signers);

    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &all_signers).unwrap();

    svm.send_transaction(tx)
}


pub fn create_multisig(
    svm: &mut LiteSVM,
    fee_payer: &Keypair,
    program_id: Pubkey,
    admins: Vec<Pubkey>,
) -> (Pubkey, u8) {
    let multisig_seed = [(b"multisig"), &0u16.to_le_bytes() as &[u8]];
    let (pda_multisig, multisig_bump) = Pubkey::find_program_address(&multisig_seed, &program_id);
    let treasury_seed = [(b"treasury"), pda_multisig.as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, _treasury_bump) = Pubkey::find_program_address(treasury_seeds, &program_id);
    let init_multisig = InitMultisigIxData {
        max_expiry: 1_000_000,
        primary_seed: 0,
        min_threshold: 2,
        num_members: admins.len() as u8,
        num_admins: admins.len() as u8,
    };

    let mut ix_data = vec![0u8];

    ix_data.extend_from_slice(unsafe { to_bytes(&init_multisig) });

    let mut accounts = vec![
        AccountMeta::new(fee_payer.pubkey(), true),
        AccountMeta::new(pda_multisig, false),
        AccountMeta::new(pda_treasury, false),
        AccountMeta::new(rent::ID, false),
        AccountMeta::new(system_program::ID, false),
    ];
    let admins_accounts = admins
        .iter()
        .map(|admin| AccountMeta::new(admin.clone(), false))
        .collect::<Vec<AccountMeta>>();
    accounts.extend(admins_accounts);

    let init_ix = Instruction {
        program_id: Pubkey::from(ID),
        accounts: accounts,
        data: ix_data,
    };

    let result = build_and_send_transaction(svm, fee_payer, vec![init_ix]);
    assert!(result.is_ok());

    (pda_multisig, multisig_bump)
}

pub fn create_proposal(
    svm: &mut LiteSVM,
    fee_payer: &Keypair,
    program_id: Pubkey,
    multisig_pda: Pubkey,
    tx_type: ProposalType,
) -> (Pubkey, u8) {
    let proposal_seed = &[
        ProposalState::SEED.as_bytes(),
        multisig_pda.as_ref(),
        &0u16.to_le_bytes(),
    ];
    let (pda_proposal, proposal_bump) = Pubkey::find_program_address(proposal_seed, &program_id);

    let create_proposal_data = CreateProposalIxData {
        expiry: 1_000_000,
        primary_seed: 0,
        tx_type,
    };

    let mut ix_data = vec![2u8];
    ix_data.extend_from_slice(unsafe { to_bytes(&create_proposal_data) });

    let create_proposal_ix = Instruction {
        program_id: Pubkey::from(ID),
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true), // creator (signer)
            AccountMeta::new(pda_proposal, false),      // proposal_account (will be created)
            AccountMeta::new_readonly(multisig_pda, false), // multisig_account (readonly)
            AccountMeta::new_readonly(rent::ID, false), // rent sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::ID, false), // clock sysvar
            AccountMeta::new_readonly(system_program::ID, false), // system program
        ],
        data: ix_data,
    };

    let result = build_and_send_transaction(svm, fee_payer, vec![create_proposal_ix]);
    assert!(result.is_ok());
    // println!("Created Proposal PDA: {:?}", pda_proposal);

    (pda_proposal, proposal_bump)
}

pub fn vote(
    svm: &mut LiteSVM,
    fee_payer: &Keypair,
    program_id: Pubkey,
    multisig_pda: Pubkey,
    multisig_bump: u8,
    proposal_pda: Pubkey,
    proposal_bump: u8,
    vote: u8,
) -> (Pubkey, u8) {
    let vote_ix = VoteIxData {
        multisig_bump: multisig_bump,
        proposal_bump: proposal_bump,
        vote,
    };

    let mut ix_data = vec![3u8];
    ix_data.extend_from_slice(unsafe { to_bytes(&vote_ix) });

    let vote_ix = vec![Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new(rent::ID, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: ix_data,
    }];

    let result = build_and_send_transaction(svm, fee_payer, vote_ix);
    println!("Vote result: {:?}", result);
    assert!(result.is_ok());
    // println!("Voted on Proposal PDA: {:?}", proposal_pda);

    (proposal_pda, proposal_bump)
}
