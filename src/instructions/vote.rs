use crate::helper::account_init::StateDefinition;
use crate::state::{member::MemberState, multisig::MultisigState, proposal::ProposalState};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::Transfer;

use crate::helper::utils::DataLen;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct VoteIxData {
    pub multisig_bump: u8,
    pub proposal_bump: u8,
    pub vote: u8,
}

impl DataLen for VoteIxData {
    const LEN: usize = core::mem::size_of::<VoteIxData>();
}

impl VoteIxData {
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

    let multisig_header = MultisigState::from_account_info(multisig_account)?;
    let derived_multisig = pinocchio_pubkey::derive_address(
        &[MultisigState::SEED.as_bytes(), &multisig_header.primary_seed.to_le_bytes()],
        Some(ix_data.multisig_bump),
        &crate::ID,
    );
    if derived_multisig != *multisig_account.key() {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (_, members) = unsafe {
        multisig_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(MultisigState::LEN)
    };

    let mut member_exists: Option<MemberState> = None;
    for (_, m) in members.chunks_exact(MemberState::LEN).enumerate() {
        let member = MemberState::from_bytes(m)?;
        if member.pubkey == *voter.key() {
            member_exists = Some(member);
            break;
        }
    }

    if member_exists.is_none() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let (proposal, votes) = unsafe {
        proposal_account
            .borrow_mut_data_unchecked()
            .split_at_mut_unchecked(ProposalState::LEN)
    };

    let mut proposal = ProposalState::from_bytes(proposal)?;

    ProposalState::validate_pda(
        proposal_account.key(),
        multisig_account.key(),
        ix_data.proposal_bump,
        proposal.proposal_id,
    )?;

    let mut voted = false;
    let mut vote_index = 0;
    for (i, vote) in votes.chunks_exact(32).enumerate() {
        if let Some(member) = member_exists {
            if vote == member.pubkey.as_ref() {
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
            let vote_start = vote_index * 32;
            let swap_start = (proposal.yes_votes as usize) * 32;

            if vote_start != swap_start {
                // Use temporary variable to avoid borrowing issues
                let mut temp_vote = [0u8; 32];
                temp_vote.copy_from_slice(&votes[vote_start..vote_start + 32]);
                let swap_data = votes[swap_start..swap_start + 32].to_vec();
                votes[vote_start..vote_start + 32].copy_from_slice(&swap_data);
                votes[swap_start..swap_start + 32].copy_from_slice(&temp_vote);
            }

            proposal.yes_votes += 1;
            proposal.no_votes -= 1;
        } else {
            // replace the last no vote with current vote and increase the yes vote count and reduce the no vote count
            let vote_start = vote_index * 32;
            let swap_start = (proposal.no_votes + proposal.yes_votes - 1) as usize * 32;

            if vote_start != swap_start {
                // Use temporary variable to avoid borrowing issues
                let mut temp_vote = [0u8; 32];
                temp_vote.copy_from_slice(&votes[vote_start..vote_start + 32]);
                let swap_data = votes[swap_start..swap_start + 32].to_vec();
                votes[vote_start..vote_start + 32].copy_from_slice(&swap_data);
                votes[swap_start..swap_start + 32].copy_from_slice(&temp_vote);
            }

            proposal.yes_votes -= 1;
            proposal.no_votes += 1;
        }

        unsafe {
            proposal_account.borrow_mut_data_unchecked()[..ProposalState::LEN]
                .copy_from_slice(proposal.to_bytes().as_ref());
            proposal_account.borrow_mut_data_unchecked()[ProposalState::LEN..]
                .copy_from_slice(&votes);
        }
    } else {
        // Increase the size of the account to add new vote
        let new_size = proposal_account.data_len() + 32;
        let rent_diff = Rent::get()?.minimum_balance(new_size) - proposal_account.lamports();

        if rent_diff > 0 {
            Transfer {
                from: voter,
                to: proposal_account,
                lamports: rent_diff,
            }
            .invoke()?;
        }

        proposal_account.resize(new_size)?;
        let (new_proposal, new_votes) = unsafe {
            proposal_account
                .borrow_mut_data_unchecked()
                .split_at_mut_unchecked(ProposalState::LEN)
        };

        let mut new_proposal_data = ProposalState::from_bytes(new_proposal)?;

        if let Some(member) = member_exists {
            let last_vote_start = (proposal.yes_votes + proposal.no_votes) as usize * 32;

            new_votes[last_vote_start..last_vote_start + 32]
                .copy_from_slice(member.pubkey.as_ref());

            if ix_data.vote == 1 {
                if proposal.no_votes > 0 {
                    let no_start = proposal.no_votes as usize * 32;
                    // Swap the new vote (at the end) with the first no vote
                    if last_vote_start != no_start {
                        // Use temporary variables to avoid borrowing issues
                        let mut temp_vote = [0u8; 32];
                        temp_vote.copy_from_slice(&new_votes[no_start..no_start + 32]);
                        let new_vote_data =
                            new_votes[last_vote_start..last_vote_start + 32].to_vec();
                        new_votes[no_start..no_start + 32].copy_from_slice(&new_vote_data);
                        new_votes[last_vote_start..last_vote_start + 32]
                            .copy_from_slice(&temp_vote);
                    }
                }
                new_proposal_data.yes_votes += 1;
            } else {
                // Add the new no vote
                new_proposal_data.no_votes += 1;
            }
        }

        unsafe {
            proposal_account.borrow_mut_data_unchecked()[..ProposalState::LEN]
                .copy_from_slice(new_proposal_data.to_bytes().as_ref());
            proposal_account.borrow_mut_data_unchecked()[ProposalState::LEN..]
                .copy_from_slice(&new_votes);
        }
    }

    Ok(())
}
