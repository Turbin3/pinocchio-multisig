use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

#[repr(C)]
pub struct MultisigConfig {
    pub min_threshold: u64, // minimum number of signers required to execute a proposal
    pub max_expiry: u64,    // Adjust size as needed
    pub proposal_count: u64, // proposal counter
    pub bump: u8,           // Bump seed for PDA
}

impl MultisigConfig {
    pub const SEED: &'static [u8] = b"multisig_config";

    pub const LEN: usize = 8 + 8 + 8 + 1; // 32 bytes for creator, 1 byte for num_members, and 32 bytes for each member

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

    pub fn validate_pda(bump: u8, pda: &Pubkey, owner: &Pubkey) -> Result<(), ProgramError> {
        let seeds = &[Self::SEED, owner];
        let derived = pinocchio_pubkey::derive_address(seeds, Some(bump), &crate::ID);
        if derived != *pda {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(())
    }
}
