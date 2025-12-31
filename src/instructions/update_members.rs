use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

use crate::helper::utils::{load_ix_data, DataLen};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct UpdateMemberIxData {
    pub operation: u8,         // 1 for add, 2 for remove
    pub member_data: [u8; 33], // 32 bytes pubkey + 1 byte role (for add) or just 32 bytes pubkey (for remove)
}

impl DataLen for UpdateMemberIxData {
    const LEN: usize = core::mem::size_of::<UpdateMemberIxData>();
}

pub(crate) fn process_update_member(accounts: &[&AccountInfo], data: &[u8]) -> ProgramResult {
    let ix_data = unsafe { load_ix_data::<UpdateMemberIxData>(data)? };

    match ix_data.operation {
        1 => {
            // Add member - pass the member_data as the data parameter
            super::add_member::add_member(accounts, &ix_data.member_data)
        }
        2 => {
            // Remove member - pass only the first 32 bytes (pubkey) as the data parameter
            super::remove_member::remove_member(accounts, &ix_data.member_data[..32])
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
