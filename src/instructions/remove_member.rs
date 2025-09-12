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

    // Check if we have an admin for multisig or not
    if multisig_state.admin != Pubkey::default() {
        //If we have an admin then check if it is the real admin or not
        if !admin_account.is_signer() || admin_account.key() != &multisig_state.admin {
            log!("unauthorized admin: {}", admin_account.key());
            return Err(ProgramError::MissingRequiredSignature);
        }
    }

    // Target member pubkey extraction
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let member_to_remove = Pubkey::from(pk_bytes);

    // Locate member in dynamic member region, clear/delete if found
    let (_header, member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(multisig::MultisigState::LEN)
    };
    let mut found_idx: Option<usize> = None;

    for (idx, m) in member_data.chunks_exact(member::MemberState::LEN).enumerate() {
        let existing = member::MemberState::from_bytes(m)?;
        if existing.pubkey == member_to_remove && existing.status != 0 {
            found_idx = Some(idx);
            break;
        }
    }

    let idx = found_idx.ok_or(ProgramError::InvalidInstructionData)?;

    let offset = idx * member::MemberState::LEN;
    let slot = &mut member_data[offset..offset + member::MemberState::LEN];

    // Load, mutate, re-serialize
    let mut member = member::MemberState::from_bytes(slot)?;
    member.status = 0;
    member.pubkey = Pubkey::default();
    slot.copy_from_slice(&member.to_bytes()?);

    // Decrement count
    multisig_state.num_members = multisig_state
        .num_members
        .checked_sub(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    log!("Removed member: {} at index {}", &member_to_remove, idx);
    Ok(())
}
