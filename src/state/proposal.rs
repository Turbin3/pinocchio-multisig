use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

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

#[repr(u8)]
pub enum TxType {
    Base = 0,
    ConfigEdit = 1,
}

#[repr(C)]
pub struct ProposalState {
    pub multisig: Pubkey,
    pub transaction_index: u64,
    pub status: ProposalStatus,
    pub tx_type: TxType,
    pub yes_votes: u8,
    pub no_votes: u8,
    pub expiry: u64,
    pub bump: u8,
}

impl ProposalState {
    pub const LEN: usize = size_of::<Self>();

    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
    }

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }
}
