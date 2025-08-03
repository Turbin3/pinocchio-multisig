use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_log::log;

use crate::state::Multisig;

pub fn process_init_multisig_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, multisig, treasury, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // Multisig Config PDA
    let seed = [(b"multisig"), creator.key().as_slice()];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = pubkey::find_program_address(seeds, &crate::ID);
    assert_eq!(&pda_multisig, multisig.key());
        
    // Treasury PDA
    let treasury_seed = [(b"treasury"), multisig.key().as_slice()];
    let treasury_seeds = &treasury_seed[..];
    let (pda_treasury, treasury_bump) = pubkey::find_program_address(treasury_seeds, &crate::ID);
    assert_eq!(&pda_treasury, treasury.key());

    if multisig.owner() != &crate::ID {
        log!("Creating Multisig Account");

        // Create Multisig Account
        pinocchio_system::instructions::CreateAccount {
            from: creator,
            to: multisig,
            lamports: Rent::get()?.minimum_balance(Multisig::LEN),
            space: Multisig::LEN as u64,
            owner: &crate::ID,
        }
        .invoke()?;

        let multisig_account = Multisig::from_account_info(&multisig)?;
        multisig_account.new(
            creator.key(),
            treasury.key(),
            treasury_bump,
            multisig_bump,
            data,
        );

    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if treasury.owner() != &crate::ID {
        log!("Creating Treasury SystemAccount");

        // Create Treasury Account
        pinocchio_system::instructions::CreateAccount {
            from: creator,
            to: treasury,
            lamports: Rent::get()?.minimum_balance(0),
            space: 0,
            owner: &pinocchio_system::ID,
        }
        .invoke()?;
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    Ok(())
}