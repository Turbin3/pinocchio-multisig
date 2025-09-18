use pinocchio::{account_info::{AccountInfo, Ref}, program_error::ProgramError, pubkey::Pubkey};
use core::mem::size_of;
use bytemuck::{Pod, Zeroable};

use crate::ID;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemberRole {
    Admin = 1,
    Member = 0,
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct MemberState {
    pub pubkey: Pubkey,
}

impl MemberState {
    pub const LEN: usize = core::mem::size_of::<MemberState>();

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

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() < 32 {
            return Err(ProgramError::InvalidAccountData);
        }
        let pubkey_bytes = unsafe { *(bytes.as_ptr() as *const [u8; 32]) };
        Ok(MemberState {
            pubkey: Pubkey::from(pubkey_bytes),
        })
    }

    pub fn to_bytes(&self) -> Result<[u8; Self::LEN], ProgramError> {
        let mut bytes = [0u8; Self::LEN];
        bytes.copy_from_slice(&self.pubkey.as_ref());
        Ok(bytes)
    }

}
