use pinocchio::{account_info::{AccountInfo, Ref}, program_error::ProgramError, pubkey::Pubkey};

use crate::ID;

#[repr(C)]
pub struct Multisig {
    pub seed: u64,
    /// Admin account for the multisig optional
    pub admin: Pubkey, 
    /// Admin spending limit
    pub admin_spending_limit: u64, 
    pub creator: Pubkey,
    /// Treasury account for the multisig, optional
    pub treasury: Pubkey,      
    /// Bump seed for the treasury PDA
    pub treasury_bump: u8,    
    /// Bump seed for the multisig PDA 
    pub bump: u8,             
    /// Minimum number of signers required to execute a proposal
    pub min_threshold: u8,     
    /// Maximum expiry time for proposals
    pub max_expiry: u64,       
    /// The index of the last transaction executed
    pub transaction_index: u64, 
    // Last stale transaction index. All transactions up until this index are stale.
    pub stale_transaction_index: u64, 
    pub num_members: u8,
    pub members_counter: u8, 
}


impl Multisig {
    pub const LEN: usize = size_of::<Self>();

    /// Populate the multisig account with initialization data
    pub fn new(
        &mut self,
        creator: &Pubkey,
        treasury: &Pubkey,
        treasury_bump: u8,
        multisig_bump: u8,
        data: &[u8],
    ) {
        self.admin = Pubkey::default();
        self.admin_spending_limit = 0;
        self.creator = *creator;
        self.treasury = *treasury;
        self.treasury_bump = treasury_bump;
        self.bump = multisig_bump;
        self.min_threshold = unsafe { *(data.as_ptr() as *const u8) };
        self.max_expiry = unsafe { *(data.as_ptr().add(1) as *const u64) };
        self.transaction_index = 0;
        self.stale_transaction_index = 0;
        self.num_members = unsafe { *(data.as_ptr().add(9) as *const u8) };
        self.members_counter = self.num_members;
    }

    #[inline]
    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
    }
    #[inline]
    pub fn from_account_info(
        account_info: &AccountInfo,
    ) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }
}
