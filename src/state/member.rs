use pinocchio::{account_info::{AccountInfo, Ref}, program_error::ProgramError, pubkey::Pubkey};

use crate::ID;

#[repr(C)]
pub struct Member {
    pub pubkey: Pubkey,
    pub id: u8,
    pub status: u8, 

}

impl Member {
    pub const LEN: usize = size_of::<Self>();


    #[inline]
    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
    }
    #[inline]
    pub fn from_account_info(
        account_info: &AccountInfo,
    ) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }

}
