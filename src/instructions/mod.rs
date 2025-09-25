pub mod add_member;
pub mod create_proposal;
pub mod create_transaction;
pub mod execute_transaction;
pub mod init_multisig;
pub mod remove_member;
pub mod update_members;
pub mod update_multisig;
pub mod vote;
pub use create_proposal::*;
pub use create_transaction::*;
pub use execute_transaction::*;
pub use init_multisig::*;
pub use update_members::*;
pub use update_multisig::*;
pub use vote::*;

use pinocchio::program_error::ProgramError;

pub enum MultisigInstructions {
    InitMultisig = 0,
    UpdateMultisig = 1,
    CreateProposal = 2,
    Vote = 3,
    CreateTransaction = 4,
    ExecuteTransaction = 5,
}

impl TryFrom<&u8> for MultisigInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(MultisigInstructions::InitMultisig),
            1 => Ok(MultisigInstructions::UpdateMultisig),
            2 => Ok(MultisigInstructions::CreateProposal),
            3 => Ok(MultisigInstructions::Vote),
            4 => Ok(MultisigInstructions::CreateTransaction),
            5 => Ok(MultisigInstructions::ExecuteTransaction),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
