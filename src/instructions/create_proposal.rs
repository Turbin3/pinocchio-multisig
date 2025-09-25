use crate::helper::{
    account_init::{create_pda_account, StateDefinition},
    utils::{load_ix_data, DataLen},
    account_checks::check_signer,
};
use crate::state::{
    multisig::MultisigState,
    member::MemberState,
    proposal::{self, ProposalState, ProposalStatus, ProposalType},
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct CreateProposalIxData {    
    pub expiry: u64,         // 8 bytes
    pub primary_seed: u16,    // 2 bytes
    pub tx_type: ProposalType,
}

impl DataLen for CreateProposalIxData {
    const LEN: usize = core::mem::size_of::<CreateProposalIxData>();
}

pub fn process_create_proposal_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, proposal_account, multisig_account, rent_sysvar_acc, clock_sysvar_acc, _remaining @ ..] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    check_signer(&creator)?;

    if !proposal_account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent_account = Rent::from_account_info(rent_sysvar_acc)?;
    let ix_data = unsafe { load_ix_data::<CreateProposalIxData>(&data)? };

    let multisig = MultisigState::from_account_info(multisig_account)?;

    // Check if creator is an admin (only if there are admins)
    if multisig.admin_counter > 0 {
        let admin_counter = multisig.admin_counter;

        // Get all members from the multisig account
        let (_, member_data) = unsafe {
            multisig_account
                .borrow_mut_data_unchecked()
                .split_at_mut_unchecked(MultisigState::LEN)
        };

        // Check if creator is an admin
        let mut is_creator_admin = false;
        for i in 0..admin_counter as usize {
            let member_start = i * MemberState::LEN;
            let member_end = member_start + MemberState::LEN;
            let member_bytes = &member_data[member_start..member_end];
            let member_pubkey = Pubkey::from(*unsafe { &*(member_bytes.as_ptr() as *const [u8; 32]) });

            if member_pubkey == *creator.key() {
                is_creator_admin = true;
                break;
            }
        }

        if !is_creator_admin {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let seeds = &[
        ProposalState::SEED.as_bytes(),
        multisig_account.key().as_slice(),
        &ix_data.primary_seed.to_le_bytes()
    ];
    let (pda_proposal, proposal_bump) = pubkey::find_program_address(seeds, &crate::ID);

    if pda_proposal.ne(proposal_account.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_bytes = [proposal_bump];
    let primary_seed_bytes = ix_data.primary_seed.to_le_bytes();
    let signer_seeds = [
        Seed::from(ProposalState::SEED.as_bytes()),
        Seed::from(multisig_account.key().as_slice()),
        Seed::from(&primary_seed_bytes),
        Seed::from(&bump_bytes[..]),
    ];

    create_pda_account::<ProposalState>(&creator, &proposal_account, &signer_seeds, &rent_account)?;

    let current_time = Clock::from_account_info(clock_sysvar_acc)?.unix_timestamp as u64;

    let proposal = ProposalState::from_account_info(&proposal_account)?;
    proposal.new(ix_data.primary_seed, ix_data.expiry, ProposalStatus::Draft, proposal_bump, current_time, ix_data.tx_type);

    Ok(())
}
