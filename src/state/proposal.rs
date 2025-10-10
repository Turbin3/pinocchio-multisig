use crate::helper::account_init::StateDefinition;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct ProposalState {
    pub proposal_id: u16, // Unique identifier for the proposal
    pub expiry: u64,      // Adjust size as needed is it needed here?
    pub created_time: u64,
    pub status: ProposalStatus,
    pub tx_type: ProposalType,
    pub bump: u8,          // Bump seed for PDA
    pub yes_votes: u8,     // Number of yes votes
    pub no_votes: u8,      // Number of no votes
    pub _padding: [u8; 3], // padding to reach multiple of 8
}

impl StateDefinition for ProposalState {
    const LEN: usize = core::mem::size_of::<ProposalState>();
    const SEED: &'static str = "proposal";
}

impl ProposalState {
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

    pub fn validate_pda(
        pda: &Pubkey,
        owner: &Pubkey,
        proposal_bump: u8,
        proposal_primary_seed: u16,
    ) -> Result<(), ProgramError> {
        let seeds = &[
            ProposalState::SEED.as_bytes(),
            owner.as_slice(),
            &proposal_primary_seed.to_le_bytes(),
        ];
        let derived = pinocchio_pubkey::derive_address(seeds, Some(proposal_bump), &crate::ID);
        if derived != *pda {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self {
            proposal_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            expiry: u64::from_le_bytes([
                bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9],
            ]),
            created_time: u64::from_le_bytes([
                bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], bytes[16],
                bytes[17],
            ]),
            status: ProposalStatus::try_from(&bytes[18])?,
            tx_type: ProposalType::try_from(&bytes[19])?,
            bump: bytes[20],
            yes_votes: bytes[21],
            no_votes: bytes[22],
            _padding: [0; 3],
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut bytes = [0u8; Self::LEN];
        bytes[0..2].copy_from_slice(&self.proposal_id.to_le_bytes());
        bytes[2..10].copy_from_slice(&self.expiry.to_le_bytes());
        bytes[10..18].copy_from_slice(&self.created_time.to_le_bytes());
        bytes[18] = self.status as u8;
        bytes[19] = self.tx_type as u8;
        bytes[20] = self.bump;
        bytes[21] = self.yes_votes;
        bytes[22] = self.no_votes;
        bytes[23..26].copy_from_slice(&self._padding);
        bytes
    }

    pub fn new(
        &mut self,
        proposal_id: u16,
        expiry: u64,
        status: ProposalStatus,
        bump: u8,
        created_time: u64,
        tx_type: ProposalType,
    ) {
        self.proposal_id = proposal_id;
        self.expiry = expiry;
        self.created_time = created_time;
        self.status = status;
        self.bump = bump;
        self.tx_type = tx_type;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ProposalType {
    Cpi = 0,
    UpdateMember = 1,
    UpdateMultisig = 2,
}

impl TryFrom<&u8> for ProposalType {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(ProposalType::Cpi),
            1 => Ok(ProposalType::UpdateMember),
            2 => Ok(ProposalType::UpdateMultisig),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
