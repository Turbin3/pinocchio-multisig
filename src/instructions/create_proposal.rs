use crate::{
    helper::{check_signer, create_pda_account},
    state::{
        multisig::MultisigState,
        proposal::{self, ProposalState, ProposalStatus},
    },
};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey,
    pubkey::Pubkey,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};

pub fn process_create_proposal_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, proposal_account, multisig_account, rent, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    check_signer(&creator)?;

    let multisig = MultisigState::from_account_info(multisig_account)?;

    if data.len() < 2 {
        return Err(ProgramError::InvalidInstructionData);
    }

    const MAX_MEMBERS: usize = 10; // or whatever your max is
    let total_members = multisig.num_members as usize;
    let mut members = [Pubkey::default(); MAX_MEMBERS];

    let mut offset = 1;
    for i in 0..total_members.min(MAX_MEMBERS) {
        let pubkey_bytes = &data[offset..offset + 32];
        let member =
            Pubkey::try_from(pubkey_bytes).map_err(|_| ProgramError::InvalidInstructionData)?;
        members[i] = member;
        offset += 32;
    }

    let seeds = [(b"proposal"), multisig_account.key().as_slice()];
    let proposal_seeds = &seeds[..];
    let (proposal_pda, proposal_bump) = pubkey::find_program_address(proposal_seeds, &crate::ID);
    if proposal_pda.ne(proposal_account.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_bytes = [proposal_bump];

    let rent = Rent::from_account_info(rent)?;
    let signer_seeds = [
        Seed::from(b"proposal"),
        Seed::from(multisig_account.key().as_ref()),
        Seed::from(&bump_bytes[..]),
    ];

    create_pda_account::<ProposalState>(&creator, &proposal_account, &signer_seeds, &rent)?;

    let proposal = ProposalState::from_account_info(proposal_account)?;
    proposal.bump = proposal_bump;

    proposal.new(
        0,
        0,
        ProposalStatus::Draft,
        proposal_bump,
        members,
        [0; 10],
        0,
    );

    Ok(())
}
