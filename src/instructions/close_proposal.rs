use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_log::log;

use crate::state::{ProposalState, MultisigConfig};

pub fn process_close_proposal_instruction(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let [proposal_info, multisig_config_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let clock = Clock::get()?;
    let current_slot = clock.slot;

    if proposal_info.owner() != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let proposal_account = ProposalState::from_account_info(proposal_info)?;
   
    let current_status = unsafe { 
        *(&proposal_account.result as *const _ as *const u8) 
    };
    
    if current_status != 1 {
        log!("Proposal is not active, current status: {}", current_status);
        return Err(ProgramError::InvalidAccountData);
    }

    if multisig_config_info.owner() != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let config_account = MultisigConfig::from_account_info(multisig_config_info)?;

    if current_slot < proposal_account.expiry {

        log!("Proposal not yet expired (current: {}, expiry: {}), setting to cancelled", 
             current_slot, proposal_account.expiry);
        
        unsafe {
            let status_ptr = &mut proposal_account.result as *mut _ as *mut u8;
            *status_ptr = 4; // Cancelled
        }
        return Ok(());
    }

    let mut yes_votes = 0u32;
    let mut no_votes = 0u32;
    let mut abstain_votes = 0u32;
    let mut total_voted = 0u32;

    for &vote in proposal_account.votes.iter() {
        match vote {
            0 => {}, // Not voted
            1 => { yes_votes += 1; total_voted += 1; }, // Yes
            2 => { no_votes += 1; total_voted += 1; },  // No  
            3 => { abstain_votes += 1; total_voted += 1; }, // Abstain
            _ => {
                log!("Invalid vote value: {}", vote);
                return Err(ProgramError::InvalidAccountData);
            }
        }
    }

    log!("Vote tally - Yes: {}, No: {}, Abstain: {}, Total: {}", 
         yes_votes, no_votes, abstain_votes, total_voted);
    log!("Required threshold: {}", config_account.min_threshold);

    // Determine final status based on threshold  
    unsafe {
        let status_ptr = &mut proposal_account.result as *mut _ as *mut u8;
        if yes_votes >= config_account.min_threshold as u32 {
            *status_ptr = 3; // Succeeded
            log!("Proposal succeeded with {} yes votes", yes_votes);
        } else {
            *status_ptr = 2; // Failed
            log!("Proposal failed with {} yes votes (needed {})", 
                 yes_votes, config_account.min_threshold);
        }
    }

    Ok(())
}