use pinocchio::{
    account_info::AccountInfo,
    instruction::Seed,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
    sysvars::rent::Rent,
};

use crate::helper::account_init::StateDefinition;
use crate::{
    state::{
        TransactionState,
    },
    helper::{
        utils::{load_ix_data, DataLen},
        account_checks::check_signer,
        account_init::{create_pda_account, HasOwner},
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct CreateTransactionIxData {
    pub transaction_index: u64,  // 8 bytes
    pub tx_maker: Pubkey,        // 32 bytes
    pub tx_buffer: [u8; 512],    // 512 bytes
    pub buffer_size: u16,        // 2 bytes
}

impl DataLen for CreateTransactionIxData {
    const LEN: usize = core::mem::size_of::<CreateTransactionIxData>();
}

impl HasOwner for CreateTransactionIxData {
    fn owner(&self) -> &Pubkey {
        &self.tx_maker
    }
}

pub fn process_create_transaction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [
        payer,
        transaction_acc,
        sysvar_rent_acc,
        _system_program,
        _rest @..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    check_signer(&payer)?;

    if !transaction_acc.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(sysvar_rent_acc)?;

    let ix_data = unsafe { load_ix_data::<CreateTransactionIxData>(&data)? };

    if ix_data.tx_maker.ne(payer.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let seeds = &[TransactionState::SEED.as_bytes(), &ix_data.tx_maker];

    let (derived_transaction_pda, bump) = pubkey::find_program_address(seeds, &crate::ID);

    if derived_transaction_pda.ne(transaction_acc.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(TransactionState::SEED.as_bytes()),
        Seed::from(ix_data.owner()),
        Seed::from(&bump_bytes[..]),
    ];

    create_pda_account::<TransactionState>(&payer, &transaction_acc, &signer_seeds, &rent)?;
    
    TransactionState::initialize(transaction_acc, ix_data, bump)?;

    Ok(())
}
