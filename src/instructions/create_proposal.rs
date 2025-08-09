use crate::state::{
    Multisig,
    proposal::{ProposalState, ProposalStatus, TxType},
};
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
pub fn process_create_proposal_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, proposal_account, multisig_account,rent_sysvar_acc, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let multisig = Multisig::from_account_info(multisig_account)?;

    let bump = unsafe { *(data.as_ptr() as *const u8) };
    let bump_bytes = bump.to_le_bytes();
    let seed = [
        (b"proposal"),
        multisig_account.key().as_ref(),
        bump_bytes.as_ref(),
    ];
    let pda = match pubkey::create_program_address(&seed[..], &crate::ID) {
        Ok(pda) => pda,
        Err(_) => return Err(ProgramError::InvalidSeeds),
    };

    if &pda != proposal_account.key() {
        return Err(ProgramError::InvalidSeeds);
    }

    if proposal_account.owner() != &crate::ID {
        let rent = Rent::from_account_info(rent_sysvar_acc)?;

        pinocchio_system::instructions::CreateAccount {
            from: creator,
            to: proposal_account,
            lamports: rent.minimum_balance(ProposalState::LEN),
            space: ProposalState::LEN as u64,
            owner: &crate::ID,
        }
        .invoke()?;
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let proposal = ProposalState::from_account_info(proposal_account)?;
    proposal.bump = bump;
    proposal.multisig = *multisig_account.key();
    proposal.transaction_index = multisig.transaction_index;
    proposal.status = ProposalStatus::Draft;
    proposal.tx_type = TxType::Base;
    proposal.yes_votes = 0;
    proposal.no_votes = 0;
    proposal.expiry = Clock::get()?.slot;

    Ok(())
}
