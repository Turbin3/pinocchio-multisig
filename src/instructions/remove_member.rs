use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};
use pinocchio_log::log;

use crate::state::Multisig;

pub fn remove_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let [multisig_account] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };
    let multisig_data = Multisig::from_account_info(multisig_account)?;

    let member_to_remove: Pubkey = unsafe {
        // Cast to pointer to Pubkey
        let ptr = data.as_ptr() as *const Pubkey;

        // Dereference to copy the value into a new Pubkey
        std::ptr::read(ptr)
    };

    let mut member_index: Option<usize> = None; //Helper pointer
    let mut last_active_index: Option<usize> = None; //Helper pointer

    // We are finding the member we want to remove and last active member
    for i in 0..multisig_data.num_members {
        let current = multisig_data.members[i as usize];

        if current == member_to_remove {
            member_index = Some(i.into());
        }

        if current != Pubkey::default() {
            last_active_index = Some(i.into());
        }
    }

    let Some(target_index) = member_index else {
        log!("Member not found in multisig :(");
        return Err(ProgramError::Custom(ErrorCode::MemberNotFound as u32));
    };

    let Some(last_index) = last_active_index else {
        log!("No active members found");
        return Err(ProgramError::Custom(ErrorCode::MemberNotFound as u32));
    };

    // Let's swappppppppppp
    if target_index != last_index {
        multisig_data.members[target_index] = multisig_data.members[last_index];
    }

    // Zero out the last member
    multisig_data.members[last_index] = Pubkey::default();

    // Decrement count
    multisig_data.num_members = multisig_data.num_members - 1;

    log!("Removed member: {}", &member_to_remove);
    log!("Remaining active members: {}", multisig_data.num_members);

    Ok(())
}

pub enum ErrorCode {
    MemberNotFound,
}
