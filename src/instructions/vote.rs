use pinocchio::{
     account_info::AccountInfo, 
    program_error::ProgramError, 
    sysvars::{clock::Clock, Sysvar}, 
    ProgramResult
};

use crate::state::{Multisig, ProposalState, ProposalStatus, VoteState, VoteType};

pub fn process_vote_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [voter, multisig, proposal_state, vote_state, _] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    let proposal_id = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], 
        data[4], data[5], data[6], data[7]
    ]);
    let vote_type = VoteType::try_from(&data[8])?;

    if multisig.owner() != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if proposal_state.owner() != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let multisig_data = Multisig::from_account_info(multisig)?;

    let mut voter_index: Option<usize> = None;
    for (index, member) in multisig_data.members.iter().enumerate() {
        if index < multisig_data.num_members as usize && member == voter.key() {
            voter_index = Some(index);
            break;
        }
    }

    let voter_idx = voter_index.ok_or(ProgramError::MissingRequiredSignature)?;
    
    if !voter.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let proposal_data = ProposalState::from_account_info(proposal_state)?;
    
    if proposal_data.proposal_id != proposal_id {
        return Err(ProgramError::InvalidAccountData);
    }

    match proposal_data.result {
        ProposalStatus::Active => {},
        _ => return Err(ProgramError::InvalidAccountData),
    }

    let mut is_active_member = false;
    for member in proposal_data.active_members.iter() {
        if member == voter.key() {
            is_active_member = true;
            break;
        }
    }

    if !is_active_member {
        return Err(ProgramError::InvalidAccountData);
    }

    if proposal_data.votes[voter_idx] != 0 {
        return Err(ProgramError::Custom(1001));
    }

    let vote_type_value = vote_type as u8;
    proposal_data.votes[voter_idx] = vote_type_value;
    
    if vote_state.owner() == &crate::ID {
        let vote_state_data = VoteState::from_account_info(vote_state)?;
        vote_state_data.voter = *voter.key();
        vote_state_data.proposal_id = proposal_id;
        vote_state_data.vote_type = vote_type_value;
        vote_state_data.timestamp = Clock::get()?.unix_timestamp;
    }

    Ok(())
}