use core::mem::MaybeUninit;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    ProgramResult,
    cpi::MAX_CPI_ACCOUNTS
};

use crate::state::{multisig_config, Multisig, Transaction};

pub fn convert_accounts_to_refs(accounts: &[AccountInfo]) -> Result<&[&AccountInfo], ProgramError> {
    let num_accounts = accounts.len();
    
    if num_accounts > MAX_CPI_ACCOUNTS {
        return Err(ProgramError::InvalidArgument);
    }

    if num_accounts == 0 {
        return Ok(&[]);
    }
    
    const UNINIT_REF: MaybeUninit<&AccountInfo> = MaybeUninit::<&AccountInfo>::uninit();
    let mut account_refs = [UNINIT_REF; MAX_CPI_ACCOUNTS];
    
    for i in 0..num_accounts {
        unsafe {
            let account: &AccountInfo = accounts.get_unchecked(i);
            
            account_refs
                .get_unchecked_mut(i)
                .write(account);
        }
    }
    
    Ok(unsafe {
        core::slice::from_raw_parts(account_refs.as_ptr() as *const &AccountInfo, num_accounts)
    })
}

pub fn process_execute_transaction_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer, proposal, multisig, transaction, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if multisig.data_is_empty() || proposal.data_is_empty() || transaction.data_is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }

    let multisig_data = unsafe { multisig.borrow_mut_data_unchecked() };

    // check if payer is a member of the multisig

    let proposal_data = unsafe { proposal.borrow_mut_data_unchecked() };

    // if proposal_data.status != Proposal::Executed {
    //     return Err(ProgramError::InvalidAccountData);
    // }

    // if proposal_data.yes_votes < multisig_data.threshold {
    //     return Err(ProgramError::InvalidAccountData);
    // }

    // if Clock::get()?.unix_timestamp > proposal_data.expiry {
    //     return Err(ProgramError::InvalidAccountData);
    // }

    let transaction_data = unsafe { transaction.borrow_mut_data_unchecked() };

    // if multisig_data.transaction_index != transaction_data.index  && multisig_data.transaction_index!= proposal_data.transaction_index{
    //     return Err(ProgramError::InvalidAccountData);
    // }

    let accounts_for_execute = &accounts[2..];

    let account_refs = convert_accounts_to_refs(accounts_for_execute)?;

    Transaction::execute(0, account_refs)?; // 0 will be changed to proposal_data.tx_type

    Ok(())
}