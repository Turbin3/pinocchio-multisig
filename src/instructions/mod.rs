pub mod init_multisig;
pub mod vote;

pub use vote::*;
pub use init_multisig::*;

use pinocchio::program_error::ProgramError;

pub enum MultisigInstructions {
    InitMultisig = 0, // Johnny + Raunit 
    //update expiry
    //update threshold
    //update members
    UpdateMultisig = 1, // Glacier + SOLDADDY + Zubayr + Yunohu
    CreateProposal = 2, // Nishant + Umang
    Vote = 3, // Shrinath + Mohammed + shradesh
    // will close if expiry achieved & votes < threshold || execute if votes >= threshold
    CloseProposal = 4, // Nanasi + Mishal + Apaar + Ghazal 

    //Santoshi CHAD own version
}

impl TryFrom<&u8> for MultisigInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(MultisigInstructions::InitMultisig),
            1 => Ok(MultisigInstructions::UpdateMultisig),
            2 => Ok(MultisigInstructions::CreateProposal),
            3 => Ok(MultisigInstructions::Vote),
            4 => Ok(MultisigInstructions::CloseProposal),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}