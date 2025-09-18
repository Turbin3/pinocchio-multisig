use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
    sysvars::rent::Rent,
};

use crate::state::{MultisigState, MemberRole};
use crate::helper::{
    utils::{load_ix_data, DataLen},
    account_checks::check_signer,
    account_init::{create_pda_account, StateDefinition},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct InitMultisigIxData {
    pub max_expiry: u64,      // 8 bytes
    pub primary_seed: u16,    // 2 bytes
    pub min_threshold: u8,    // 1 byte
    pub num_members: u8,      // 1 byte
    pub num_admins: u8,       // 1 byte
}

impl DataLen for InitMultisigIxData {
    const LEN: usize = core::mem::size_of::<InitMultisigIxData>();
}

pub fn process_init_multisig_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, multisig, treasury, rent, system_program, remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    check_signer(&creator)?;

    if !multisig.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent_account = Rent::from_account_info(rent)?;

    let ix_data = unsafe { load_ix_data::<InitMultisigIxData>(&data)? };

    if ix_data.num_members < ix_data.num_admins {
        return Err(ProgramError::InvalidAccountData);
    }

    // Multisig Config PDA
    let seeds = &[MultisigState::SEED.as_bytes(), &ix_data.primary_seed.to_le_bytes()];
    let (pda_multisig, multisig_bump) = pubkey::find_program_address(seeds, &crate::ID);

    if pda_multisig.ne(multisig.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Treasury PDA
    let treasury_seed = [(b"treasury"), multisig.key().as_slice()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = pubkey::find_program_address(treasury_seeds, &crate::ID);

    if pda_treasury.ne(treasury.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_bytes = [multisig_bump];
    let primary_seed_bytes = ix_data.primary_seed.to_le_bytes();
    let signer_seeds = [
        Seed::from(MultisigState::SEED.as_bytes()),
        Seed::from(&primary_seed_bytes),
        Seed::from(&bump_bytes[..]),
    ];

    create_pda_account::<MultisigState>(&creator, &multisig, &signer_seeds, &rent_account)?;

    let multisig_account = MultisigState::from_account_info(&multisig)?;
    multisig_account.new(
        treasury.key(),
        treasury_bump,
        multisig_bump,
        ix_data,
    );

    // Add all members
    add_all_members(creator, multisig, &rent_account, multisig_account, remaining, ix_data)?;

    if !treasury.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Create treasury account with correct signer seeds
    let treasury_bump_bytes = [treasury_bump];
    let treasury_signer_seeds = [
        Seed::from(b"treasury"),
        Seed::from(multisig.key()),
        Seed::from(&treasury_bump_bytes),
    ];

    create_pda_account::<MultisigState>(&creator, &treasury, &treasury_signer_seeds, &rent_account)?;

    Ok(())
}

fn add_all_members(
    creator: &AccountInfo,
    multisig: &AccountInfo,
    rent_account: &Rent,
    multisig_account: &mut MultisigState,
    remaining: &[AccountInfo],
    ix_data: &InitMultisigIxData,
) -> ProgramResult {
    if ix_data.num_members > 0 {
        // Calculate total size needed for all members
        let total_member_size = ix_data.num_members as usize * crate::state::member::MemberState::LEN;
        let new_size = multisig.data_len() + total_member_size;
        let min_balance = rent_account.minimum_balance(new_size);
        let rent_diff = min_balance.saturating_sub(multisig.lamports());

        if rent_diff > 0 {
            use pinocchio_system::instructions::Transfer;
            Transfer {
                from: creator,
                to: multisig,
                lamports: rent_diff,
            }
            .invoke()?;
        }

        multisig.resize(new_size)?;

        // Get member data section
        let (_, member_data) = unsafe {
            multisig
                .borrow_mut_data_unchecked()
                .split_at_mut_unchecked(MultisigState::LEN)
        };

        // Add all members in order (admins first, then normal members)
        for i in 0..ix_data.num_members as usize {
            let member = &remaining[i];
            let member_start = i * crate::state::member::MemberState::LEN;
            let member_end = member_start + crate::state::member::MemberState::LEN;
            
            // Copy the pubkey directly
            member_data[member_start..member_end].copy_from_slice(member.key());
        }

        // Update counters
        multisig_account.num_members = ix_data.num_members;
        multisig_account.admin_counter = ix_data.num_admins;
    }
    
    Ok(())
}
