use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::rent::Rent,
    ProgramResult,
};

use crate::helper::{
    account_checks::check_signer,
    account_init::{create_pda_account, StateDefinition},
    utils::{load_ix_data, DataLen},
};
use crate::state::MultisigState;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct UpdateMultisigIxData {
    pub value: u64,      // For spending limit and stale transaction index
    pub update_type: u8, // 1 for update threshold, 2 for update spending limit, 3 for stale transaction index
    pub threshold: u8,   // For threshold updates
}

impl DataLen for UpdateMultisigIxData {
    const LEN: usize = core::mem::size_of::<UpdateMultisigIxData>();
}

pub(crate) fn process_update_multisig(accounts: &[&AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer, multisig, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let ix_data = unsafe { load_ix_data::<UpdateMultisigIxData>(data)? };

    let mut multisig_state = MultisigState::from_account_info(multisig)?;

    match ix_data.update_type {
        1 => multisig_state.update_threshold(ix_data.threshold),
        2 => multisig_state.update_spending_limit(ix_data.value),
        3 => multisig_state.update_stale_transaction_index(ix_data.value),
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    Ok(())
}
