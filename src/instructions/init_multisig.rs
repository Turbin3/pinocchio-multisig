use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_log::log;

use crate::state::{Multisig, MultisigConfig};

pub fn process_init_multisig_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, multisig, multisig_config, treasury, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // Multisig PDA
    let seed = [(b"multisig"), creator.key().as_slice()];
    let seeds = &seed[..];
    let (pda_multisig, multisig_bump) = pubkey::find_program_address(seeds, &crate::ID);
    assert_eq!(&pda_multisig, multisig.key());

    // Multisig_config PDA
    let multisig_config_seed = [(b"multisig_config"), multisig.key().as_slice()];
    let multisig_config_seeds = &multisig_config_seed[..];
    let (pda_config, multisig_config_bump) =
        pubkey::find_program_address(multisig_config_seeds, &crate::ID);
    assert_eq!(&pda_config, multisig_config.key());

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

        // Populate Multisig Account
        let multisig_account = Multisig::from_account_info(&multisig)?;
        multisig_account.creator = *creator.key();
        multisig_account.num_members = unsafe { *(data.as_ptr().add(1) as *const u8) };
        multisig_account.members = [Pubkey::default(); 10]; 
        multisig_account.treasury = *treasury.key();
        multisig_account.treasury_bump = treasury_bump;
        multisig_account.bump = multisig_bump;

        match multisig_account.num_members {
            0..=10 => {
                for i in 0..multisig_account.num_members as usize {
                    let member_key = unsafe { *(data.as_ptr().add(2 + i * 32) as *const [u8; 32]) };
                    multisig_account.members[i] = member_key;
                }
            }
            _ => return Err(ProgramError::InvalidAccountData),
        }

        log!("members: {}", unsafe {
            *(data.as_ptr().add(1) as *const u8)
        });
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if multisig_config.owner() != &crate::ID {
        log!("Creating Multisig Config Account");

        // Create Multisig Config Account
        pinocchio_system::instructions::CreateAccount {
            from: creator,
            to: multisig_config,
            lamports: Rent::get()?.minimum_balance(MultisigConfig::LEN),
            space: MultisigConfig::LEN as u64,
            owner: &crate::ID,
        }
        .invoke()?;

        // Populate Multisig Config Account
        let multisig_config_account = MultisigConfig::from_account_info(&multisig_config)?;
        multisig_config_account.min_threshold = unsafe { *(data.as_ptr().add(8) as *const u64) };
        multisig_config_account.max_expiry = unsafe { *(data.as_ptr().add(16) as *const u64) };
        multisig_config_account.proposal_count = 0;
        multisig_config_account.bump = multisig_config_bump;

        log!("members: {}", unsafe {
            *(data.as_ptr().add(1) as *const u8)
        });
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