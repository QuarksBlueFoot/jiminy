use pinocchio::{error::ProgramError, AccountView, Address};

use crate::checks::{
    check_account, check_executable, check_signer, check_system_program, check_writable,
};

/// Iterator-style account accessor with inline constraint checks.
///
/// In raw pinocchio you typically write:
///
/// ```rust,ignore
/// if accounts.len() < 4 {
///     return Err(ProgramError::NotEnoughAccountKeys);
/// }
/// let payer  = &accounts[0];
/// check_signer(payer)?;
/// let vault  = &accounts[1];
/// check_writable(vault)?;
/// let system = &accounts[2];
/// check_system_program(system)?;
/// let state  = &accounts[3];
/// check_account(state, program_id, STATE_DISC, STATE_LEN)?;
/// ```
///
/// `AccountList` collapses that to:
///
/// ```rust,ignore
/// let mut accs = AccountList::new(accounts);
/// let payer  = accs.next_signer()?;
/// let vault  = accs.next_writable()?;
/// let system = accs.next_system_program()?;
/// let state  = accs.next_account(program_id, STATE_DISC, STATE_LEN)?;
/// ```
///
/// Same instructions generated at the end - just far less noise.
pub struct AccountList<'a> {
    accounts: &'a [AccountView],
    pos: usize,
}

impl<'a> AccountList<'a> {
    #[inline(always)]
    pub fn new(accounts: &'a [AccountView]) -> Self {
        Self { accounts, pos: 0 }
    }

    /// How many accounts haven't been consumed yet.
    #[inline(always)]
    pub fn remaining(&self) -> usize {
        self.accounts.len().saturating_sub(self.pos)
    }

    /// Consume the next account with no additional checks.
    ///
    /// Use this for accounts where you want to do custom validation
    /// immediately after, or for accounts that only need to exist.
    #[inline(always)]
    pub fn next(&mut self) -> Result<&'a AccountView, ProgramError> {
        if self.pos >= self.accounts.len() {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let acc = &self.accounts[self.pos];
        self.pos += 1;
        Ok(acc)
    }

    /// Consume the next account and verify it signed the transaction.
    #[inline(always)]
    pub fn next_signer(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_signer(acc)?;
        Ok(acc)
    }

    /// Consume the next account and verify it is marked writable.
    #[inline(always)]
    pub fn next_writable(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_writable(acc)?;
        Ok(acc)
    }

    /// Consume the next account and verify it is a writable signer.
    ///
    /// Common pattern for the fee payer or the instruction authority
    /// that is also being mutated (e.g. depositing lamports).
    #[inline(always)]
    pub fn next_writable_signer(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_signer(acc)?;
        check_writable(acc)?;
        Ok(acc)
    }

    /// Consume the next account and verify it is the system program.
    #[inline(always)]
    pub fn next_system_program(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_system_program(acc)?;
        Ok(acc)
    }

    /// Consume the next account and verify its address matches `expected`.
    ///
    /// Use this to assert any well-known program or sysvar address:
    ///
    /// ```rust,ignore
    /// let token_prog = accs.next_with_address(&jiminy::programs::TOKEN)?;
    /// ```
    #[inline(always)]
    pub fn next_with_address(
        &mut self,
        expected: &Address,
    ) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        if acc.address() != expected {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(acc)
    }

    /// Consume the next account and run the combined ownership + size +
    /// discriminator check. This is the most common pattern for your
    /// program's own state accounts.
    #[inline(always)]
    pub fn next_account(
        &mut self,
        program_id: &Address,
        discriminator: u8,
        min_len: usize,
    ) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_account(acc, program_id, discriminator, min_len)?;
        Ok(acc)
    }

    /// Consume the next account as a writable state account.
    ///
    /// Combines `next_account` with a writable check - the most common
    /// pattern for accounts being modified in an instruction.
    #[inline(always)]
    pub fn next_writable_account(
        &mut self,
        program_id: &Address,
        discriminator: u8,
        min_len: usize,
    ) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_writable(acc)?;
        check_account(acc, program_id, discriminator, min_len)?;
        Ok(acc)
    }

    /// Consume the next account and verify it is an executable program.
    ///
    /// Use for CPI target programs passed as instruction accounts, where you
    /// want to confirm the caller didn't pass a regular data account.
    #[inline(always)]
    pub fn next_executable(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_executable(acc)?;
        Ok(acc)
    }
}
