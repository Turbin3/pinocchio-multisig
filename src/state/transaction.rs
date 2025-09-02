use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    ProgramResult,
};
use bytemuck::{Pod, Zeroable};
use crate::instructions::create_transaction::CreateTransactionIxData;
use crate::helper::account_init::StateDefinition;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankAccount, Pod, Zeroable)]
pub struct TransactionState {
    pub transaction_index: u64,
    pub buffer_size: u16,
    pub tx_buffer: [u8; 512],
    pub bump: u8,
    pub _padding: [u8; 5],
}

impl StateDefinition for TransactionState {
    const LEN: usize = core::mem::size_of::<TransactionState>();
    const SEED: &'static str = "transaction";
}

impl TransactionState {
    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
    }

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }

    pub fn initialize(
        transaction_acc: &AccountInfo,
        ix_data: &CreateTransactionIxData,
        bump: u8,
    ) -> ProgramResult {
        let transaction_state = TransactionState::from_account_info(&transaction_acc)?;

        transaction_state.transaction_index = ix_data.transaction_index;
        transaction_state.tx_buffer = ix_data.tx_buffer;
        transaction_state.buffer_size = ix_data.buffer_size;
        transaction_state.bump = bump;

        Ok(())
    }
}