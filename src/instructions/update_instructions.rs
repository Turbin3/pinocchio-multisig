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
pub struct UpdateMultisigIxData {
    pub type: u8, // 1 for update threshold, 2 for update spending limit, 3 for stale transaction index
    pub value: u8,
}

pub fn process_update_multisig(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer, multisig, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    let ix_data = load_ix_data::<UpdateMultisigIxData>(data)?;

    let multisig_state = MultisigState::from_account_info(multisig)?;

    match ix_data.type {
        1 => multisig_state.update_threshold(ix_data.value as u8),
        2 => multisig_state.update_spending_limit(ix_data.value as u64),
        3 => multisig_state.update_stale_transaction_index(ix_data.value as u64),
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    Ok(())
}