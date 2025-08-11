use crate::state::{member::Member, multisig::Multisig, proposal::ProposalState, MultisigConfig};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

#[repr(C)]
pub struct VoteIxData {
    pub multisig_bump: u8,
    pub proposal_bump: u8,
    pub vote: u8,
}

impl VoteIxData {
    pub const LEN: usize = 1 + 1 + 1;

    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(Self {
            multisig_bump: data[0],
            proposal_bump: data[1],
            vote: data[2],
        })
    }

    pub fn to_bytes(&self) -> Result<[u8; Self::LEN], ProgramError> {
        let mut bytes = [0u8; Self::LEN];
        bytes[0] = self.multisig_bump;
        bytes[1] = self.proposal_bump;
        bytes[2] = self.vote;
        Ok(bytes)
    }
}

pub fn process_vote_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [voter, multisig_account, proposal_account, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = VoteIxData::from_bytes(data)?;

    MultisigConfig::validate_pda(ix_data.multisig_bump, multisig_account.key(), &crate::ID)?;

    let (_, members) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(Multisig::LEN)
    };

    let mut member_exists = None;
    for member in members.chunks_exact(Member::LEN) {
        let member = Member::from_bytes(member)?;
        if member.pubkey == *voter.key() {
            member_exists = Some(member.id);
            break;
        }
    }

    if member_exists.is_none() {
        // Member not a part of the multisig
        return Err(ProgramError::InvalidInstructionData);
    }

    ProposalState::validate_pda(
        ix_data.proposal_bump,
        proposal_account.key(),
        multisig_account.key(),
    )?;

    let (proposal, votes) = unsafe {
        proposal_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(ProposalState::LEN)
    };

    let mut proposal = ProposalState::from_bytes(proposal)?;

    let mut voted = false;
    let mut vote_index = 0;
    for (i, vote) in votes.chunks_exact(1).enumerate() {
        if let Some(member_id) = member_exists {
            if vote[0] == member_id {
                if i < proposal.yes_votes as usize && ix_data.vote == 1 {
                    // Already voted yes
                    return Err(ProgramError::InvalidInstructionData);
                } else if proposal.yes_votes as usize <= i && ix_data.vote == 0 {
                    // Already voted no
                    return Err(ProgramError::InvalidInstructionData);
                }
                voted = true;
                vote_index = i;
                break;
            }
        }
    }

    if voted {
        // Swap and update the vote count
        if ix_data.vote == 1 {
            // replace the last yes vote with current vote and reduce the yes vote count and increase the no vote count
            let temp = votes[vote_index];
            votes[vote_index] = votes[proposal.yes_votes as usize];
            votes[proposal.yes_votes as usize] = temp;
            proposal.yes_votes -= 1;
            proposal.no_votes += 1;
        } else {
            // replace the last no vote with current vote and increase the yes vote count and reduce the no vote count
            let temp = votes[vote_index];
            votes[vote_index] = votes[proposal.no_votes as usize];
            votes[proposal.no_votes as usize] = temp;
            proposal.yes_votes += 1;
            proposal.no_votes -= 1;
        }
    } else {
        // Increase the size of the account to add new vote
        let new_size = proposal_account.data_len() + 1;
        let rent_diff = Rent::get()?.minimum_balance(new_size) - proposal_account.lamports();
        proposal_account.resize(new_size)?;
        unsafe {
            *voter.borrow_mut_lamports_unchecked() -= rent_diff;
            *proposal_account.borrow_mut_lamports_unchecked() += rent_diff;
        }

        let (new_proposal, new_votes) = unsafe {
            proposal_account
                .borrow_mut_data_unchecked()
                .split_at_mut_unchecked(ProposalState::LEN)
        };

        new_proposal.copy_from_slice(&proposal.to_bytes()?);
        if ix_data.vote == 1 {
            // swap the last yes vote with first no vote
            let temp = new_votes[proposal.yes_votes as usize];
            new_votes[proposal.yes_votes as usize] = new_votes[0];
            new_votes[0] = temp;
            proposal.yes_votes += 1;
        } else {
            // Add the last no vote
            new_votes[proposal.no_votes as usize] = member_exists.unwrap();
            proposal.no_votes += 1;
        }
    }

    Ok(())
}
