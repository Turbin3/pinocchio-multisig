use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};
use pinocchio_log::log;
use crate::helper::StateDefinition;
use crate::state::{multisig, member};

pub fn remove_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [admin_account, multisig_account, rent_acc, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let multisig_state = multisig::MultisigState::from_account_info(multisig_account)?;

    // Admin check if present
    if multisig_state.admin != Pubkey::default() {
        if !admin_account.is_signer() || admin_account.key() != &multisig_state.admin {
            log!("unauthorized admin: {}", admin_account.key());
            return Err(ProgramError::MissingRequiredSignature);
        }
    }

    // Member to remove pubkey
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let member_to_remove = Pubkey::from(pk_bytes);

    // Load members
    let (_header, member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(multisig::MultisigState::LEN)
    };
    let num_members = multisig_state.num_members as usize;

    // Find and remove member (linear scan)
    let mut found_idx: Option<(usize, u8)> = None; // (index, status)
    for (idx, m) in member_data.chunks_exact(member::MemberState::LEN).enumerate() {
        let member = member::MemberState::from_bytes(m)?;
        if member.pubkey == member_to_remove {
            found_idx = Some((idx, member.status));
            break;
        }
    }
    let (idx, status) = found_idx.ok_or(ProgramError::InvalidInstructionData)?;

    // Shift subsequent members left
    for i in idx..num_members - 1 {
        let dst_offset = i * member::MemberState::LEN;
        let src_offset = (i + 1) * member::MemberState::LEN;

        let (left, right) = member_data.split_at_mut(src_offset);
        let src_slice = &right[..member::MemberState::LEN];
        let dst_slice = &mut left[dst_offset..dst_offset + member::MemberState::LEN];
        dst_slice.copy_from_slice(src_slice);
    }

    // Zero out the last slot (now orphaned)
    let last_offset = (num_members - 1) * member::MemberState::LEN;
    member_data[last_offset..last_offset + member::MemberState::LEN].fill(0);

    // Resize account to shrink by one member
    let old_size = multisig_account.data_len();
    let new_size = old_size - member::MemberState::LEN;
    multisig_account.resize(new_size)?;

    // Update counters
    multisig_state.num_members = multisig_state
        .num_members
        .checked_sub(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    if status == 1 {
        multisig_state.admin_counter = multisig_state
            .admin_counter
            .checked_sub(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    log!("Removed member: {} (was admin? {}) at index {}", &member_to_remove, status == 1, idx);
    Ok(())
}
