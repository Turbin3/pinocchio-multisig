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

    if data.len() < 33 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let rent = Rent::from_account_info(rent_acc)?;

    let (_header_data, member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(multisig::MultisigState::LEN)
    };

    let multisig_state = multisig::MultisigState::from_account_info(multisig_account)?;
    if admin_account.key() != &multisig_state.admin {
        log!("unauthorized admin: {}", admin_account.key());
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let new_member_pubkey = Pubkey::from(pk_bytes);

    let role = data[32]; // 1 = admin, 0 = member (assuming role is passed as last byte of input)

    // Check for duplicate
    for m in member_data.chunks_exact(member::MemberState::LEN) {
        let existing = member::MemberState::from_bytes(m)?;
        if existing.pubkey == new_member_pubkey {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    let new_member = member::MemberState {
        pubkey: new_member_pubkey,
        id: multisig_state.num_members,
        status: role,
    };

    // Find the correct insert position: after last admin (if adding admin), or at end
    let mut admin_end = 0;
    for m in member_data.chunks_exact(member::MemberState::LEN) {
        let existing = member::MemberState::from_bytes(m)?;
        if existing.status == 1 { // 1 = admin
            admin_end += 1;
        }
    }
    let insert_pos = if role == 1
        {
            admin_end
        } else {
            multisig_state.num_members as usize
        };

    // Resize to add a new member
    let new_size = multisig_account.data_len() + member::MemberState::LEN;
    let min_balance = rent.minimum_balance(new_size);
    let lamports_needed = min_balance.saturating_sub(multisig_account.lamports());
    if lamports_needed > 0 {
        unsafe {
            *admin_account.borrow_mut_lamports_unchecked() -= lamports_needed;
            *multisig_account.borrow_mut_lamports_unchecked() += lamports_needed;
        }
    }
    multisig_account.resize(new_size)?;

    // Write new member, shifting items if needed
    let (_, new_member_data) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(multisig::MultisigState::LEN)
    };

    // Shift members to make room at insert_pos
    let old_members = new_member_data.to_vec();
    if insert_pos < multisig_state.num_members as usize {
        new_member_data[(insert_pos+1)*member::MemberState::LEN..(multisig_state.num_members as usize + 1)*member::MemberState::LEN]
            .copy_from_slice(&old_members[insert_pos*member::MemberState::LEN..multisig_state.num_members as usize * member::MemberState::LEN]);
    }

    // Insert the new member at insert_pos
    new_member_data[insert_pos*member::MemberState::LEN..(insert_pos+1)*member::MemberState::LEN]
        .copy_from_slice(&new_member.to_bytes()?);

    // Update counters
    multisig_state.num_members = multisig_state
        .num_members
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if role == 1 {
        multisig_state.admin_counter = multisig_state
            .admin_counter
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    log!("Added new {} pubkey={}", if role == 1 { "admin" } else { "member" }, &new_member_pubkey);
    Ok(())
}
