pub mod add_member;
pub mod init_multisig;
pub mod update_multisig;
pub mod remove_member;

pub use add_member::*;
pub use init_multisig::*;
pub use update_multisig::*;
pub use remove_member::*;

use pinocchio::program_error::ProgramError;

pub enum MultisigInstructions {
    InitMultisig = 0, // Johnny + Raunit
    //update expiry
    //update threshold
    //update members
    //UpdateMultisig = 1, // Glacier + SOLDADDY + Zubayr + Yunohu
    // CreateProposal = 2, // Nishant + Umang
    //Vote = 3, // Shrinath + Mohammed + shradesh
    // will close if expiry achieved & votes < threshold || execute if votes >= threshold
    // CloseProposal = 4, // Nanasi + Mishal + Apaar + Ghazal

    //Santoshi CHAD own version
    AddMember = 5, //SolDaddy
    RemoveMember = 6, //SolDaddy
}

impl TryFrom<&u8> for MultisigInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(MultisigInstructions::InitMultisig),
            //1 => Ok(MultisigInstructions::UpdateMultisig),
            //2 => Ok(MultisigInstructions::CreateProposal),
            //3 => Ok(MultisigInstructions::Vote),
            //4 => Ok(MultisigInstructions::CloseProposal),
            5 => Ok(MultisigInstructions::AddMember),
            6=>Ok(MultisigInstructions::RemoveMember),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
