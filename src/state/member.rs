use pinocchio::{account_info::{AccountInfo, Ref}, program_error::ProgramError, pubkey::Pubkey};
use core::mem::size_of;

use crate::ID;

#[repr(C)]
pub struct MemberState {
    pub pubkey: Pubkey,
    pub id: u8,
    pub status: u8,
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
        let creator_pubkey = unsafe { *(bytes.as_ptr() as *const [u8; 32]) };
        let id = bytes[32];
        let status = bytes[33];
        Ok(MemberState {
            pubkey: creator_pubkey,
            id,
            status,
        })
    }

    pub fn to_bytes(&self) -> Result<[u8; Self::LEN], ProgramError> {
        let mut bytes = [0u8; Self::LEN];
        bytes[0..32].copy_from_slice(&self.pubkey);
        bytes[32..33].copy_from_slice(&self.id.to_le_bytes());
        bytes[33..34].copy_from_slice(&self.status.to_le_bytes());
        Ok(bytes)
    }

}