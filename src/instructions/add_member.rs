use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::rent::Rent,
    ProgramResult,
};
use pinocchio_log::log;
use crate::helper::StateDefinition;
use crate::state::{member, multisig};

pub fn add_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [admin_account, multisig_account, rent_acc, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Must contain at least 32 bytes (pubkey)
    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Rent sysvar
    let rent = Rent::from_account_info(rent_acc)?;

    // Mutable multisig data split for existing members
    let (_header_data, member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(multisig::MultisigState::LEN)
    };

    // Load multisig state (checked)
    let multisig_state = multisig::MultisigState::from_account_info(multisig_account)?;
    if admin_account.key() != &multisig_state.admin {
        log!("unauthorized admin: {}", admin_account.key());
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Extract proposed member pubkey
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let new_member_pubkey = Pubkey::from(pk_bytes);

    // Check for duplicate member
    for m in member_data.chunks_exact(member::MemberState::LEN) {
        let existing = member::MemberState::from_bytes(m)?;
        if existing.pubkey == new_member_pubkey {
            return Err(ProgramError::InvalidInstructionData); // Already present
        }
    }

    // Prepare new member
    let mut new_member = member::MemberState {
        pubkey: new_member_pubkey,
        id: multisig_state.num_members,
        status: 1,
    };

    // New account size
    let new_size = multisig_account.data_len() + member::MemberState::LEN;
    let min_balance = rent.minimum_balance(new_size);
    let lamports_needed = min_balance.saturating_sub(multisig_account.lamports());
    if lamports_needed > 0 {
        //Might as well add a transfer function to gracefully do this
        unsafe {
            *admin_account.borrow_mut_lamports_unchecked() -= lamports_needed;
            *multisig_account.borrow_mut_lamports_unchecked() += lamports_needed;
        }
    }

    // Resize for new member
    multisig_account.resize(new_size)?;

    // Write new member after resize
    let (_, new_member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(multisig::MultisigState::LEN)
    };

    // Actually write the member data
    let offset = (multisig_state.num_members as usize) * member::MemberState::LEN;
    new_member_data[offset..offset + member::MemberState::LEN]
        .copy_from_slice(&new_member.to_bytes()?);

    // Update count
    multisig_state.num_members = multisig_state
        .num_members
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    log!("Added new member pubkey={}", &new_member_pubkey);
    Ok(())
}
