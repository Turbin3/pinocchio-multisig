use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey
};

#[repr(C)]
pub struct ProposalState {
    pub proposal_id: u64, // Unique identifier for the proposal
    pub expiry: u64,// Adjust size as needed is it needed here?
    pub result: ProposalStatus,
    pub bump: u8, // Bump seed for PDA
    pub active_members: [Pubkey; 10], // Array to hold active members, adjust size as needed

    //VOTE 0 - NOT VOTED
    //VOTE 1 - FOR
    //VOTE 2 - AGAINST
    //VOTE 3 - ABSTAIN
    pub votes:[u8; 10],

    // imo slot
    pub created_time: u64,
    // analysis period
}

impl ProposalState {
    pub const LEN: usize = 8 + 8 + 1 + 1 + 32 * 10 + 10 + 8; // proposal_id + expiry + result + bump + active_members + votes + created_time

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
#[repr(u8)]
pub enum ProposalStatus {
    Draft = 0,
    Active = 1,
    Failed = 2,
    Succeeded = 3,
    Cancelled = 4,
}


impl TryFrom<&u8> for ProposalStatus {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(ProposalStatus::Draft),
            1 => Ok(ProposalStatus::Active),
            2 => Ok(ProposalStatus::Failed),
            3 => Ok(ProposalStatus::Succeeded),
            4 => Ok(ProposalStatus::Cancelled),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}