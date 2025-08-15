use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::helper::utils::DataLen;

pub trait HasOwner {
    fn owner(&self) -> &Pubkey;
}

pub trait StateDefinition {
    const LEN: usize;
    const SEED: &'static str;
}

#[inline(always)]
pub fn create_pda_account<S>(
    payer: &AccountInfo,
    account: &AccountInfo,
    signer_seeds: &[Seed],
    rent: &Rent,
) -> Result<(), ProgramError>
where
    S: StateDefinition,
{
    let signers = [Signer::from(signer_seeds)];

    CreateAccount {
        from: payer,
        to: account,
        space: S::LEN as u64,
        owner: &crate::ID,
        lamports: rent.minimum_balance(S::LEN),
    }
    .invoke_signed(&signers)?;

    Ok(())
}
