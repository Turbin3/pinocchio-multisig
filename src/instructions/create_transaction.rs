use pinocchio::{
    account_info::AccountInfo, instruction::Seed, program_error::ProgramError, pubkey,
    sysvars::rent::Rent, ProgramResult,
};

use crate::helper::account_init::StateDefinition;
use crate::{
    helper::{
        account_checks::{check_signer, is_authorized_multisig_creator},
        account_init::create_pda_account,
        utils::{load_ix_data, DataLen},
    },
    state::{ProposalState, TransactionState},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct CreateTransactionIxData {
    pub transaction_index: u64, // 8 bytes
    pub tx_buffer: [u8; 512],   // 512 bytes
    pub buffer_size: u16,       // 2 bytes
}

impl DataLen for CreateTransactionIxData {
    const LEN: usize = core::mem::size_of::<CreateTransactionIxData>();
}

pub fn process_create_transaction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer, multisig_acc, proposal_acc, transaction_acc, sysvar_rent_acc, _system_program, _rest @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    check_signer(&payer)?;

    if !transaction_acc.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if multisig_acc.data_is_empty() || proposal_acc.data_is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::from_account_info(sysvar_rent_acc)?;

    let ix_data = unsafe { load_ix_data::<CreateTransactionIxData>(&data)? };

    let proposal = ProposalState::from_account_info(proposal_acc)?;

    if proposal.yes_votes != 0 || proposal.no_votes != 0 {
        return Err(ProgramError::InvalidAccountData);
    }

    ProposalState::validate_pda(
        proposal_acc.key(),
        multisig_acc.key(),
        proposal.bump,
        proposal.proposal_id,
    )?;

    if !is_authorized_multisig_creator(payer, multisig_acc)? {
        return Err(ProgramError::InvalidAccountData);
    }

    let seeds = &[
        TransactionState::SEED.as_bytes(),
        proposal_acc.key().as_slice(),
    ];

    let (derived_transaction_pda, bump) = pubkey::find_program_address(seeds, &crate::ID);

    if derived_transaction_pda.ne(transaction_acc.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(TransactionState::SEED.as_bytes()),
        Seed::from(proposal_acc.key().as_slice()),
        Seed::from(&bump_bytes[..]),
    ];

    create_pda_account::<TransactionState>(&payer, &transaction_acc, &signer_seeds, &rent)?;

    TransactionState::initialize(transaction_acc, ix_data, bump)?;

    Ok(())
}
