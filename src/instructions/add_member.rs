use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::rent::Rent,
    ProgramResult,
};
use crate::state::{member::{MemberState, MemberRole}, multisig::MultisigState};
use crate::helper::account_init::StateDefinition;
use pinocchio_system::instructions::Transfer;

pub(crate) fn add_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer, multisig_account, rent_acc, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 33 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let rent = Rent::from_account_info(rent_acc)?;
    let mut multisig_state = MultisigState::from_account_info(multisig_account)?;

    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let new_member_pubkey = Pubkey::from(pk_bytes);
    let role = data[32]; // 1 = admin, 0 = member

    // Get current members data
    let (_, member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(MultisigState::LEN)
    };

    // Check for duplicate
    for m in member_data.chunks_exact(MemberState::LEN) {
        let existing = MemberState::from_bytes(m)?;
        if existing.pubkey == new_member_pubkey {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    let new_member = MemberState {
        pubkey: new_member_pubkey,
    };

    // Find insert position: after last admin (if adding admin), or at end
    let insert_pos = if role == MemberRole::Admin as u8 {
        multisig_state.admin_counter as usize
    } else {
        multisig_state.num_members as usize
    };

    // Resize account to add new member
    let new_size = multisig_account.data_len() + MemberState::LEN;
    let rent_diff = rent.minimum_balance(new_size) - multisig_account.lamports();

    if rent_diff > 0 {
        Transfer {
            from: payer,
            to: multisig_account,
            lamports: rent_diff,
        }
        .invoke()?;
    }

    multisig_account.resize(new_size)?;

    // Get updated member data after resize
    let (_, new_member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(MultisigState::LEN)
    };

    if role == MemberRole::Admin as u8 {
        // Admin: insert at admin_counter position and shift all normal members right
        let shift_start = insert_pos * MemberState::LEN;
        let shift_end = multisig_state.num_members as usize * MemberState::LEN;

        // Shift all normal members right to make space for new admin
        for i in (shift_start..shift_end).rev() {
            new_member_data[i + MemberState::LEN] = new_member_data[i];
        }
        // Insert the new admin at admin_counter position
        let insert_start = insert_pos * MemberState::LEN;
        let insert_end = insert_start + MemberState::LEN;
        new_member_data[insert_start..insert_end].copy_from_slice(&new_member.to_bytes()?);
    } else {
        // Normal member: just append at the end
        let insert_start = insert_pos * MemberState::LEN;
        let insert_end = insert_start + MemberState::LEN;
        new_member_data[insert_start..insert_end].copy_from_slice(&new_member.to_bytes()?);
    }

    // Update counters
    multisig_state.num_members = multisig_state.num_members.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;

    if role == MemberRole::Admin as u8 {
        multisig_state.admin_counter = multisig_state.admin_counter.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
    }

    let header_len = MultisigState::LEN;

    // The expression inside the block now becomes the value of `header_data`
    let header_data = unsafe { &mut multisig_account.borrow_mut_data_unchecked()[..header_len] };

    // Pass a reference to `multisig_state`
    header_data.copy_from_slice(bytemuck::bytes_of(multisig_state));
    Ok(())
}
