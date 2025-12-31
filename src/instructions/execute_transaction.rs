use core::mem::MaybeUninit;
use pinocchio::{
    account_info::AccountInfo,
    cpi::MAX_CPI_ACCOUNTS,
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use crate::state::multisig::MultisigState;
use crate::state::proposal::{ProposalState, ProposalStatus};
use crate::state::transaction::TransactionState;

pub struct AccountRefs<'a> {
    pub refs: [MaybeUninit<&'a AccountInfo>; MAX_CPI_ACCOUNTS],
    pub count: usize,
}

pub fn convert_accounts_to_refs<'a>(
    accounts: &'a [AccountInfo],
) -> Result<AccountRefs<'a>, ProgramError> {
    let num_accounts = accounts.len();

    if num_accounts > MAX_CPI_ACCOUNTS {
        return Err(ProgramError::InvalidArgument);
    }

    const UNINIT_REF: MaybeUninit<&AccountInfo> = MaybeUninit::<&AccountInfo>::uninit();
    let mut account_refs = [UNINIT_REF; MAX_CPI_ACCOUNTS];

    for i in 0..num_accounts {
        unsafe {
            let account: &AccountInfo = accounts.get_unchecked(i);

            account_refs.get_unchecked_mut(i).write(account);
        }
    }

    Ok(AccountRefs {
        refs: account_refs,
        count: num_accounts,
    })
}

pub fn process_execute_transaction_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer, multisig, proposal, transaction, rent, _system_program, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if multisig.data_is_empty() || proposal.data_is_empty() || transaction.data_is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }

    let multisig_data = MultisigState::from_account_info(multisig)?;

    let proposal_data = ProposalState::from_account_info(proposal)?;

    ProposalState::validate_pda(
        proposal.key(),
        multisig.key(),
        proposal_data.bump,
        proposal_data.proposal_id,
    )?;

    match proposal_data.status {
        ProposalStatus::Succeeded | ProposalStatus::Cancelled | ProposalStatus::Failed => {
            return Err(ProgramError::InvalidAccountData)
        }
        _ => {}
    }

    let yes_votes = proposal_data.yes_votes;

    if yes_votes < multisig_data.min_threshold {
        return Err(ProgramError::InvalidAccountData);
    }

    if Clock::get()?.unix_timestamp > proposal_data.expiry as i64 {
        return Err(ProgramError::InvalidAccountData);
    }

    let transaction_data = TransactionState::from_account_info(transaction)?;

    if multisig_data.transaction_index != transaction_data.transaction_index {
        return Err(ProgramError::InvalidAccountData);
    }

    let accounts_for_execute = &accounts;

    let account_refs_struct = convert_accounts_to_refs(accounts_for_execute)?;

    let account_refs = unsafe {
        core::slice::from_raw_parts(
            account_refs_struct.refs.as_ptr() as *const &AccountInfo,
            account_refs_struct.count,
        )
    };

    TransactionState::execute(proposal_data.tx_type, account_refs)?;

    proposal_data.status = ProposalStatus::Succeeded;

    Ok(())
}
