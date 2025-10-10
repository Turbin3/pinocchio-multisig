use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

#[inline(always)]
pub fn close_account(account: &AccountInfo, destination: &AccountInfo) -> ProgramResult {
    {
        let mut data = account.try_borrow_mut_data()?;
        data[0] = 0xff;
    }

    *destination.try_borrow_mut_lamports()? += *account.try_borrow_lamports()?;

    account.realloc(1, true)?;
    account.close()
}
