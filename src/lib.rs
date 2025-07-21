#![allow(unexpected_cfgs)]
// #![no_std]

#[cfg(feature = "std")]
extern crate std;

use pinocchio::{
    account_info::AccountInfo, 
    entrypoint, 
    program_error::ProgramError, 
    pubkey::Pubkey,
    ProgramResult,
};

mod state;
mod instructions;

use instructions::*;

entrypoint!(process_instruction);

pinocchio_pubkey::declare_id!("3Cxo8aHmXk4thjhEM2Upm5Mdupj9NNhJ94LdkGaGskbs");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],

) -> ProgramResult {
    assert_eq!(program_id, &ID);
    


    let (discriminator, data) = data.split_first().ok_or(ProgramError::InvalidAccountData)?;

  match MultisigInstructions::try_from(discriminator)? {
    MultisigInstructions::InitMultisig => {
        instructions::process_init_multisig_instruction(accounts, data)?
    },
    MultisigInstructions::CloseProposal => {
        instructions::process_close_proposal_instruction(accounts, data)?
    },
    MultisigInstructions::UpdateMultisig 
    | MultisigInstructions::CreateProposal 
    | MultisigInstructions::Vote => {
        // Not yet implemented by other teams
        return Err(ProgramError::InvalidInstructionData);
    },
}

    Ok(())
}
