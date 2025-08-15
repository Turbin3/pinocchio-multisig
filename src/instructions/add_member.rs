use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use pinocchio_log::log;

use crate::state::Multisig;

pub fn add_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    //Basic check
    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let [multisig_account] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };
    let multisig_data = Multisig::from_account_info(multisig_account)?;

    // Get the first 32 bytes of data as it contains the pubkey of the new member
    let new_member: Pubkey = unsafe {
        // Cast to pointer to Pubkey
        let ptr = data.as_ptr() as *const Pubkey;

        // Dereference to copy the value into a new Pubkey
        std::ptr::read(ptr)
    };

    let mut empty_slot_index: Option<usize> = None;

    // This loop is doing 2 things in one pass.
    // a. Check if member already exist in the multisig
    // b. Check for the first empty slot (0000000000000 pubkey)

    for i in 0..multisig_data.num_members {
        let existing_member = multisig_data.members[i as usize];

        if existing_member == new_member {
            log!("Member is already in the multisig");
            return Err(ProgramError::Custom(ErrorCode::ExistingMember as u32));
        }

        if existing_member == Pubkey::default() {
            empty_slot_index = Some(i.into());
        }
    }

    let Some(index) = empty_slot_index else {
        log!("Multisig member list is full");
        return Err(ProgramError::Custom(ErrorCode::MaxMembers as u32));
    };

    multisig_data.members[index] = new_member;

    Ok(())
}

pub enum ErrorCode {
    MaxMembers,
    ExistingMember,
}
