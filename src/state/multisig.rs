use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

#[repr(C)]
pub struct Multisig {
    pub creator: Pubkey,
    pub num_members: u8,
    pub members: [Pubkey; 10], // Adjust size as needed
    pub treasury: Pubkey,      // Treasury account for the multisig
    pub treasury_bump: u8,     // Bump seed for the treasury PDA
    pub bump: u8,              // Bump seed for PDA

                               //threshold
                               //treasury
                               //treasury_bump
}

impl Multisig {
    pub const LEN: usize = 32 + 1 + 32 * 10 + 1; // 32 bytes for creator, 1 byte for num_members, and 32 bytes for each member

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