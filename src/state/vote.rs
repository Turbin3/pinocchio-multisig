use pinocchio::{
    account_info::AccountInfo, 
    pubkey::Pubkey,
    program_error::ProgramError
};

#[repr(u8)]
pub enum VoteType {
    NotVoted = 0,
    For = 1,
    Against = 2,
    Abstain = 3,
}

impl TryFrom<&u8> for VoteType {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(VoteType::NotVoted),
            1 => Ok(VoteType::For),
            2 => Ok(VoteType::Against),
            3 => Ok(VoteType::Abstain),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

#[repr(C)]
pub struct VoteState {
    pub voter: Pubkey,
    pub proposal_id: u64,
    pub vote_type: u8,
    pub timestamp: i64,
    pub bump: u8,
}

impl VoteState {
    pub const LEN: usize = 32 + 8 + 1 + 8 + 1; 
    
    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
    }

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }
}