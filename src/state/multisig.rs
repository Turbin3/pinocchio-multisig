// use pinocchio::{
//     account_info::AccountInfo,
//     pubkey::Pubkey
// };

// #[repr(C)]
// pub struct Multisig {
//     pub creator: Pubkey,
//     pub num_members: u8,
//     pub members: [Pubkey; 10], // Adjust size as needed
//     pub bump: u8, // Bump seed for PDA
//     pub treasury: Pubkey, // Treasury account for the multisig
//     pub treasury_bump: u8, // Bump seed for the treasury PDA

//     //threshold
//     //treasury
//     //treasury_bump
// }

// impl Multisig {
//     pub const LEN: usize = 32 + 1 + 32 * 10 + 1; // 32 bytes for creator, 1 byte for num_members, and 32 bytes for each member

//     pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
//         unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
//     }

//     pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
//         if account_info.data_len() < Self::LEN {
//             return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
//         }
//         Ok(Self::from_account_info_unchecked(account_info))
//     }
// }

use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

#[repr(C)]
pub struct Multisig {
    /// Admin account for the multisig optional
    /// This field is set to None in the init instruction
    pub admin: Option<Pubkey>,
    /// Admin spending limit
    /// This field is set to None in the init instruction
    pub admin_spending_limit: Option<u64>,
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
    pub members: [Pubkey; 10], // Adjust size as needed
}

impl Multisig {
    pub const LEN: usize = size_of::<Option<Pubkey>>()
        + size_of::<Option<u64>>()
        + size_of::<Pubkey>()
        + size_of::<Pubkey>()
        + size_of::<u8>()
        + size_of::<u8>()
        + size_of::<u64>()
        + size_of::<u64>()
        + size_of::<u64>()
        + size_of::<u8>()
        + size_of::<[Pubkey; 10]>();

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
}
