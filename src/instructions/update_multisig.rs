use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::find_program_address,
    ProgramResult,
};

use crate::state::MultisigConfig;

pub fn update_multisig_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [multisig, multisig_config, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    //Derive multisig_config pda and verify
    let multisig_config_seed = [(b"multisig_config"), multisig.key().as_slice()];
    let multisig_config_seeds = &multisig_config_seed[..];

    let (multisig_config_derived_pda, multisig_config_bump) =
        find_program_address(multisig_config_seeds, &crate::ID);

    assert_eq!(&multisig_config_derived_pda, multisig_config.key());

    //Update logic
    if multisig.owner() == &crate::ID {
        let multisig_config_data = MultisigConfig::from_account_info_unchecked(multisig_config);

        //Add some check to verify data length
        //Need review for the unsafe code block below
        unsafe {
            multisig_config_data.min_threshold = *(data.as_ptr().add(1) as *const u64);
            multisig_config_data.max_expiry = *(data.as_ptr().add(9) as *const u64);
        }
        Ok(())
    } else {
        return Err(ProgramError::InvalidAccountOwner);
    }
}
