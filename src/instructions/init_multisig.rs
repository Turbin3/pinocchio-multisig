use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
    sysvars::rent::Rent,
};

use crate::state::MultisigState;
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
}

impl DataLen for InitMultisigIxData {
    const LEN: usize = core::mem::size_of::<InitMultisigIxData>();
}

pub fn process_init_multisig_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, multisig, treasury, rent, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    check_signer(&creator)?;

    if !multisig.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(rent)?;

    let ix_data = unsafe { load_ix_data::<InitMultisigIxData>(&data)? };

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

    create_pda_account::<MultisigState>(&creator, &multisig, &signer_seeds, &rent)?;

    let multisig_account = MultisigState::from_account_info(&multisig)?;
    multisig_account.new(
        treasury.key(),
        treasury_bump,
        multisig_bump,
        ix_data,
    );

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

    create_pda_account::<MultisigState>(&creator, &treasury, &treasury_signer_seeds, &rent)?;

    Ok(())
}