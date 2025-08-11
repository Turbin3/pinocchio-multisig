use pinocchio::{
    program_error::ProgramError,
    pubkey::{self, Pubkey},
};

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

impl ProposalStatus {
    pub fn to_u8(&self) -> u8 {
        match self {
            ProposalStatus::Draft => 0,
            ProposalStatus::Active => 1,
            ProposalStatus::Failed => 2,
            ProposalStatus::Succeeded => 3,
            ProposalStatus::Cancelled => 4,
        }
    }
}

#[repr(C)]
pub struct ProposalState {
    pub multisig: [u8; 32],
    pub transaction_index: u64,
    pub status: ProposalStatus,
    pub tx_type: u8,
    pub yes_votes: u8,
    pub no_votes: u8,
    pub expiry: u64,
    pub bump: u8,
}

impl ProposalState {
    pub const LEN: usize = 32 + 8 + 1 + 1 + 1 + 1 + 8 + 1;

    pub const SEED: &'static [u8] = b"proposal";

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProgramError> {
        let multisig = unsafe { *(bytes.as_ptr() as *const [u8; 32]) };
        let transaction_index = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
        let status = ProposalStatus::try_from(&bytes[40])?;
        let tx_type = bytes[41];
        let yes_votes = bytes[42];
        let no_votes = bytes[43];
        let expiry = u64::from_le_bytes(bytes[44..52].try_into().unwrap());
        let bump = bytes[52];
        Ok(Self {
            multisig,
            transaction_index,
            status,
            tx_type,
            yes_votes,
            no_votes,
            expiry,
            bump,
        })
    }

    pub fn to_bytes(&self) -> Result<[u8; Self::LEN], ProgramError> {
        let mut bytes = [0u8; Self::LEN];
        bytes[0..32].copy_from_slice(&self.multisig);
        bytes[32..40].copy_from_slice(&self.transaction_index.to_le_bytes());
        bytes[40] = self.status.to_u8();
        bytes[41] = self.tx_type;
        bytes[42] = self.yes_votes;
        bytes[43] = self.no_votes;
        bytes[44..52].copy_from_slice(&self.expiry.to_le_bytes());
        bytes[52] = self.bump;
        Ok(bytes)
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
