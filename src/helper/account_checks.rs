use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::helper::utils::DataLen;

#[inline(always)]
pub fn check_signer(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

#[inline(always)]
pub fn check_pda_valid(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.data_is_empty() && account.owner().ne(&crate::ID){
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}


 