use crate::helper::account_init::StateDefinition;
use crate::state::{
    multisig::MultisigState,
    proposal::{self, ProposalState, ProposalStatus},
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
    let [creator, proposal_account, multisig_account, rent_sysvar_acc, _remaining @ ..] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let multisig = MultisigState::from_account_info(multisig_account)?;

    if data.len() < 2 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // const MAX_MEMBERS: usize = 10; // or whatever your max is
    // let total_members = multisig.num_members as usize;
    // let mut members = [Pubkey::default(); MAX_MEMBERS];

    // let mut offset = 2;
    // for i in 0..total_members.min(MAX_MEMBERS) {
    //     let pubkey_bytes = &data[offset..offset + 32];
    //     let member =
    //         Pubkey::try_from(pubkey_bytes).map_err(|_| ProgramError::InvalidInstructionData)?;
    //     members[i] = member;
    //     offset += 32;
    // }

    let seeds = [(b"proposal"), multisig_account.key().as_slice()];
    let proposal_seeds = &seeds[..];
    let (proposal_pda, bump) = pubkey::find_program_address(proposal_seeds, &crate::ID);
    if proposal_pda.ne(proposal_account.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_bytes = [bump];

    if proposal_account.owner() != &crate::ID {
        let rent = Rent::from_account_info(rent_sysvar_acc)?;
        let cpi_seed = [
            Seed::from(b"proposal"),
            Seed::from(multisig_account.key().as_ref()),
            Seed::from(&bump_bytes[..]),
        ];
        let cpi_signer = Signer::from(&cpi_seed[..]);

        pinocchio_system::instructions::CreateAccount {
            from: creator,
            to: proposal_account,
            lamports: rent.minimum_balance(ProposalState::LEN),
            space: ProposalState::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[cpi_signer])?;
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let proposal = ProposalState::from_account_info(proposal_account)?;
    proposal.bump = bump;

    proposal.new(0, 0, ProposalStatus::Draft, bump, [0; 10], 0);

    Ok(())
}
