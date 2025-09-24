use core::mem::MaybeUninit;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    instruction::{Instruction, Seed, Signer, AccountMeta},
    ProgramResult,
    cpi::{MAX_CPI_ACCOUNTS, slice_invoke_signed},
};
use bytemuck::{Pod, Zeroable};
use crate::instructions::create_transaction::CreateTransactionIxData;
use crate::helper::account_init::StateDefinition;
use crate::state::multisig::MultisigState;
use crate::state::proposal::ProposalType;

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

    /// deserialize fun
    fn deserialize_instruction(&self) -> Result<(Pubkey, &[u8]), ProgramError> {
        let buffer = &self.tx_buffer[..self.buffer_size as usize];

        if buffer.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let program_id_bytes: [u8; 32] = buffer[0..32].try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        let program_id = Pubkey::from(program_id_bytes);

        let instruction_data = &buffer[32..];

        Ok((program_id, instruction_data))
    }

    fn get_account_metas<'a>(cpi_accounts_slice: &'a [&AccountInfo]) -> Result<&'a [AccountMeta<'a>], ProgramError> {
        const UNINIT_META: MaybeUninit<AccountMeta> = MaybeUninit::<AccountMeta>::uninit();

        let mut metas: [MaybeUninit<AccountMeta>; MAX_CPI_ACCOUNTS] = [UNINIT_META; MAX_CPI_ACCOUNTS];

        if cpi_accounts_slice.len() > MAX_CPI_ACCOUNTS {
            return Err(ProgramError::InvalidArgument);
        }

        for i in 0..cpi_accounts_slice.len() {
            unsafe {
                metas
                    .get_unchecked_mut(i)
                    .write(AccountMeta::from(*cpi_accounts_slice.get_unchecked(i)));
            }
        }

        let meta_slice: &[AccountMeta<'a>] = unsafe {
            core::slice::from_raw_parts(metas.as_ptr() as _, cpi_accounts_slice.len())
        };

        Ok(meta_slice)
    }

    /// execute tx fun
    pub fn execute(tx_type: ProposalType, accounts: &[&AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let multisig_acc: &AccountInfo = account_info_iter.next().ok_or(ProgramError::InvalidAccountData)?;
        let transaction_acc: &AccountInfo = account_info_iter.next().ok_or(ProgramError::InvalidAccountData)?;

        let cpi_accounts_slice: &[&AccountInfo] = account_info_iter.as_slice();

        let multisig_state = MultisigState::from_account_info(multisig_acc)?;
        let transaction_state = Self::from_account_info(transaction_acc)?;

        let (cpi_program_id, cpi_data_slice) = transaction_state.deserialize_instruction()?;

        let meta_slice = Self::get_account_metas(cpi_accounts_slice)?;

        let cpi_instruction = Instruction {
            program_id: &cpi_program_id,
            accounts: meta_slice,
            data: cpi_data_slice,
        };

        let binding = multisig_state.bump.to_le_bytes();
        let primary_seed_bytes = multisig_state.primary_seed.to_le_bytes();
        let signer_seeds = [
            Seed::from(MultisigState::SEED.as_bytes()),
            Seed::from(&primary_seed_bytes),
            Seed::from(&binding),
        ];

        let signers = [Signer::from(&signer_seeds[..])];

        match tx_type {
            ProposalType::Cpi => { // Base transaction - execute CPI
                slice_invoke_signed(&cpi_instruction, cpi_accounts_slice, &signers)?;
            },
            ProposalType::AddMember => { // AddMember
                // TODO: Implement member addition logic
                // add_member(cpi_accounts_slice[0])?;
            },
            ProposalType::RemoveMember => { // RemoveMember
                // TODO: Implement member removal logic
                // remove_member(cpi_accounts_slice[0])?;
            },
            ProposalType::ChangeThreshold => { // ChangeThreshold
                // TODO: Implement threshold change logic
                // change_threshold(cpi_accounts_slice[0], cpi_data_slice[0])?;
            },
            ProposalType::ChangeSpendingLimit => { // ChangeSpendingLimit
                // TODO: Implement spending limit change logic
                // change_spending_limit(cpi_accounts_slice[0], cpi_data_slice[0])?;
            },
            ProposalType::StaleTransactionIndex => { // StaleTransactionIndex
                // TODO: Implement stale transaction index logic
                // stale_transaction_index(cpi_accounts_slice[0])?;
            },
            _ => {
                return Err(ProgramError::InvalidInstructionData);
            }
        }

        Ok(())
    }
}