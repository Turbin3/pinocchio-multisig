use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};
use pinocchio_log::log;

use crate::state::{multisig, member};
pub fn remove_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [admin_account, multisig_account, member_account, remaining @..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Load multisig
    let multisig_state = multisig::MultisigState::from_account_info(multisig_account)?;

    // Admin authorization
    if !admin_account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if admin_account.key() != &multisig_state.admin {
        log!("unauthorized admin: {}", admin_account.key());
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Extract member pubkey
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let member_to_remove = Pubkey::from(pk_bytes);

    // Load and verify member
    let member_state = member::MemberState::from_account_info(member_account)?;

    if member_state.pubkey != member_to_remove {
        log!("pubkey mismatch: expected={}, got={}", &member_state.pubkey, &member_to_remove);
        return Err(ProgramError::InvalidAccountData);
    }

    if member_state.status == 0 {
        log!("member already inactive: {}", &member_state.pubkey);
        return Err(ProgramError::UninitializedAccount);
    }

    // Update member state
    member_state.status = 0;
    member_state.pubkey = Pubkey::default();

    // Update multisig counter
    multisig_state.members_counter = multisig_state
        .members_counter
        .checked_sub(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    log!("Removed member: {} at index {}", &member_to_remove, member_state.id);
    Ok(())
}
