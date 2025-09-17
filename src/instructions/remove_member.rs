use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};
use crate::state::{member::MemberState, multisig::MultisigState};
use crate::helper::account_init::StateDefinition;

pub(crate) fn remove_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer, multisig_account, system_program, _remaining @ ..] = accounts else {

        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut multisig_state = MultisigState::from_account_info(multisig_account)?;

    // Member to remove pubkey
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let member_to_remove = Pubkey::from(pk_bytes);

    // Get current members data
    let (_, member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(MultisigState::LEN)
    };

    // Find member to remove
    let mut found_idx: Option<usize> = None;
    for (idx, m) in member_data.chunks_exact(MemberState::LEN).enumerate() {
        let member = MemberState::from_bytes(m)?;
        if member.pubkey == member_to_remove {
            found_idx = Some(idx);
            break;
        }
    }
    let idx = found_idx.ok_or(ProgramError::InvalidInstructionData)?;
    // Determine if it's an admin based on position
    let is_admin = idx < multisig_state.admin_counter as usize;

    if is_admin {
        // Admin: swap with last admin, then left shift all normal members
        let last_admin_idx = multisig_state.admin_counter as usize - 1;

        if idx != last_admin_idx {
            let member1_start = idx * MemberState::LEN;
            let member2_start = last_admin_idx * MemberState::LEN;

            // Swap the two members
            for i in 0..MemberState::LEN {
                let temp = member_data[member1_start + i];
                member_data[member1_start + i] = member_data[member2_start + i];
                member_data[member2_start + i] = temp;
            }
        }
        // Left shift all normal members (after admin section)
        let admin_section_end = multisig_state.admin_counter as usize * MemberState::LEN;
        let normal_members_start = admin_section_end;
        let normal_members_end = multisig_state.num_members as usize * MemberState::LEN;

        for i in normal_members_start..normal_members_end - MemberState::LEN {
            member_data[i] = member_data[i + MemberState::LEN];
        }

        // Zero out the last slot
        let last_offset = (multisig_state.num_members as usize - 1) * MemberState::LEN;
        member_data[last_offset..last_offset + MemberState::LEN].fill(0);

        // Update counters
        multisig_state.admin_counter = multisig_state.admin_counter.checked_sub(1).ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        // Normal member: swap with last member
        let last_member_idx = multisig_state.num_members as usize - 1;

        if idx != last_member_idx {
            let member1_start = idx * MemberState::LEN;
            let member2_start = last_member_idx * MemberState::LEN;

            // Swap the two members
            for i in 0..MemberState::LEN {
                let temp = member_data[member1_start + i];
                member_data[member1_start + i] = member_data[member2_start + i];
                member_data[member2_start + i] = temp;
            }
        }
        // Zero out the last slot
        let last_offset = (multisig_state.num_members as usize - 1) * MemberState::LEN;
        member_data[last_offset..last_offset + MemberState::LEN].fill(0);
    }

    // Resize account to shrink by one member
    let new_size = multisig_account.data_len() - MemberState::LEN;
    multisig_account.resize(new_size)?;

    // Update num_members counter
    multisig_state.num_members = multisig_state.num_members.checked_sub(1).ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(())
}
