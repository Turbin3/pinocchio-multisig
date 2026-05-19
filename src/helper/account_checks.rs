use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    helper::account_init::StateDefinition,
    state::{MemberState, MultisigState},
};

#[inline(always)]
pub fn check_signer(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

pub fn is_authorized_multisig_creator(
    signer: &AccountInfo,
    multisig_acc: &AccountInfo,
) -> Result<bool, ProgramError> {
    let (admin_counter, num_members) = {
        let multisig = MultisigState::from_account_info(multisig_acc)?;
        (multisig.admin_counter, multisig.num_members)
    };

    let allowed_count = if admin_counter > 0 {
        admin_counter
    } else {
        num_members
    } as usize;

    if allowed_count == 0 {
        return Ok(false);
    }

    let (_, member_data) = unsafe {
        multisig_acc
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(MultisigState::LEN)
    };

    let allowed_len = allowed_count
        .checked_mul(MemberState::LEN)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    if member_data.len() < allowed_len {
        return Err(ProgramError::InvalidAccountData);
    }

    for member_bytes in member_data[..allowed_len].chunks_exact(MemberState::LEN) {
        let member = MemberState::from_bytes(member_bytes)?;
        if member.pubkey == *signer.key() {
            return Ok(true);
        }
    }

    Ok(false)
}
