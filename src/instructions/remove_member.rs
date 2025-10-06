use crate::helper::account_init::StateDefinition;
use crate::state::{member::MemberState, multisig::MultisigState};
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

pub(crate) fn remove_member(accounts: &[&AccountInfo], data: &[u8]) -> ProgramResult {
    let [_, multisig_account, _rent_acc, _system_program_acc, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut multisig_state = MultisigState::from_account_info(multisig_account)?;
    if multisig_state.num_members == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // member area slice (bytes after header)
    let header_len = MultisigState::LEN;
    let member_area = unsafe {
        &mut multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(header_len)
            .1
    };

    let total_members = multisig_state.num_members as usize;
    let member_len = MemberState::LEN;

    if member_area.len() < total_members * member_len {
        return Err(ProgramError::InvalidAccountData);
    }

    // parse target pubkey
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let member_to_remove = Pubkey::from(pk_bytes);

    // find index
    let mut found_idx: Option<usize> = None;
    for (idx, chunk) in member_area.chunks_exact(member_len).enumerate() {
        let m = MemberState::from_bytes(chunk)?;
        if m.pubkey == member_to_remove {
            found_idx = Some(idx);
            break;
        }
    }
    let idx = found_idx.ok_or(ProgramError::InvalidInstructionData)?;

    let admin_count = multisig_state.admin_counter as usize;
    let is_admin = idx < admin_count;

    if is_admin {
        // Admin removal:
        // 1) swap the selected admin with last admin slot (if different)
        // 2) shift the normal-members block left by one member slot
        let last_admin_idx = admin_count
            .checked_sub(1)
            .ok_or(ProgramError::InvalidAccountData)?;

        if idx != last_admin_idx {
            let a = idx * member_len;
            let b = last_admin_idx * member_len;
            for j in 0..member_len {
                member_area.swap(a + j, b + j);
            }
        }

        // Shift normal members left by one member slot (if any normal members exist)
        let admin_section_end = admin_count * member_len;
        let normal_members_end = total_members * member_len;
        let normal_count = total_members - admin_count;

        if normal_count > 0 {
            // dst = (admin_section_end - member_len)..(normal_members_end - member_len)
            // src = (admin_section_end)..(normal_members_end)
            member_area.copy_within(
                admin_section_end..normal_members_end,
                admin_section_end - member_len,
            );

            let last_offset = (total_members - 1) * member_len;
            member_area[last_offset..last_offset + member_len].fill(0);
        } else {
            let last_offset = (total_members - 1) * member_len;
            member_area[last_offset..last_offset + member_len].fill(0);
        }

        multisig_state.admin_counter = multisig_state
            .admin_counter
            .checked_sub(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        // Normal member removal:
        let last_member_idx = total_members - 1;
        if idx != last_member_idx {
            let a = idx * member_len;
            let b = last_member_idx * member_len;
            for j in 0..member_len {
                member_area.swap(a + j, b + j);
            }
        }
        let last_offset = (total_members - 1) * member_len;
        member_area[last_offset..last_offset + member_len].fill(0);
    }

    multisig_state.num_members = multisig_state
        .num_members
        .checked_sub(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let new_size = multisig_account
        .data_len()
        .checked_sub(member_len)
        .ok_or(ProgramError::InvalidAccountData)?;
    multisig_account.resize(new_size)?;

    Ok(())
}
