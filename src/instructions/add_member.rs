use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::rent::Rent,
    ProgramResult,
};
use pinocchio_log::log;
use pinocchio_system;

use crate::state::{member, multisig};
use crate::ID;

pub fn add_member(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {

    let [admin_account, multisig_account, member_account, rent_acc, _remaining@ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };


    // Instruction must contain at least 32 bytes (pubkey of new member)
    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    

    // Load multisig account
    let multisig_state = multisig::MultisigState::from_account_info(multisig_account)?;
    

    // Check admin authorization
    if admin_account.key() != &multisig_state.admin {
        log!("unauthorized admin: {}", admin_account.key());
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Capacity check: if full, reject
    if multisig_state.members_counter >= multisig_state.num_members {
        log!(
            "multisig full: members_counter={} / num_members={}",
            multisig_state.members_counter,
            multisig_state.num_members
        );
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Extract new member pubkey
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&data[..32]);
    let new_member_pubkey = Pubkey::from(pk_bytes);

    // Derive PDA for the next index using members_counter
    let idx = multisig_state.members_counter;
    let idx_bytes = [idx];
    let seeds = [
        b"member".as_ref(),
        multisig_account.key().as_slice(),
        &idx_bytes,
    ];
    let (expected_pda, bump) = pubkey::find_program_address(&seeds, &ID);

    if expected_pda != *member_account.key() {
        log!("invalid member_account provided: {}", member_account.key());
        return Err(ProgramError::InvalidAccountData);
    }

    // Rent sysvar
    let rent = Rent::from_account_info(rent_acc)?;

    // Create the new member account
    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"member"),
        Seed::from(multisig_account.key().as_ref()),
        Seed::from(&idx_bytes),
        Seed::from(&bump_bytes),
    ];
    let cpi_signer = Signer::from(&signer_seeds[..]);

    pinocchio_system::instructions::CreateAccount {
        from: admin_account,
        to: member_account,
        lamports: rent.minimum_balance(member::MemberState::LEN),
        space: member::MemberState::LEN as u64,
        owner: &ID,
    }
        .invoke_signed(&[cpi_signer])?;

    // Initialize member state
    let member_state = member::MemberState::from_account_info(member_account)?;
    member_state.pubkey = new_member_pubkey;
    member_state.id = idx;
    member_state.status = 1;

    // Advance counter
    multisig_state.members_counter = multisig_state
        .members_counter
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;


    log!("Created new member idx={} pubkey={}", idx, &new_member_pubkey);
    Ok(())
}
