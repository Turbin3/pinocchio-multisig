use pinocchio::{account_info::{AccountInfo, Ref}, program_error::ProgramError, pubkey::Pubkey, ProgramResult};
use core::mem::size_of;

use bytemuck::{Pod, Zeroable};

use crate::helper::account_init::StateDefinition;
use crate::instructions::init_multisig::InitMultisigIxData;

#[derive(Pod, Zeroable, Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct MultisigState {
    pub seed: u64,
    /// Admin spending limit
    pub admin_spending_limit: u64,
    /// Maximum expiry time for proposals
    pub max_expiry: u64,
    /// The index of the last transaction executed
    pub transaction_index: u64,
    // Last stale transaction index. All transactions up until this index are stale.
    pub stale_transaction_index: u64,
    pub primary_seed: u16,
    /// Admin account for the multisig optional
    pub admin: Pubkey,    
    /// Treasury account for the multisig, optional
    pub treasury: Pubkey,      
    /// Bump seed for the treasury PDA
    pub treasury_bump: u8,    
    /// Bump seed for the multisig PDA 
    pub bump: u8,             
    /// Minimum number of signers required to execute a proposal
    pub min_threshold: u8,    
    pub num_members: u8,
    pub members_counter: u8,
    pub admin_counter: u8,
}

impl StateDefinition for MultisigState {
    const LEN: usize = core::mem::size_of::<MultisigState>();
    const SEED: &'static str = "multisig";
}

impl MultisigState {
    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
    }

    pub fn from_account_info(
        account_info: &AccountInfo,
    ) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }

    pub fn new(
        &mut self,
        treasury: &Pubkey,
        treasury_bump: u8,
        multisig_bump: u8,
        ix_data: &InitMultisigIxData,
    ) {
        self.admin = Pubkey::default();
        self.admin_spending_limit = 0;
        self.treasury = *treasury;
        self.treasury_bump = treasury_bump;
        self.bump = multisig_bump;
        self.min_threshold = ix_data.min_threshold;
        self.max_expiry = ix_data.max_expiry;
        self.transaction_index = 0;
        self.stale_transaction_index = 0;
        self.num_members = ix_data.num_members;
        self.members_counter = 0;
        self.admin_counter = 0;
        self.primary_seed = ix_data.primary_seed;
    }
}