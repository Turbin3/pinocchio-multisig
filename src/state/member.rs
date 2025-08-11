use core::mem::size_of;
use pinocchio::{
    account_info::{AccountInfo, Ref},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::ID;

#[repr(C)]
pub struct Member {
    pub pubkey: [u8; 32],
    pub id: u8,
    pub status: u8,
}

impl Member {
    pub const LEN: usize = size_of::<Self>();

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProgramError> {
        let creator_pubkey = unsafe { *(bytes.as_ptr() as *const [u8; 32]) };
        let id = bytes[32];
        let status = bytes[33];
        Ok(Member {
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
