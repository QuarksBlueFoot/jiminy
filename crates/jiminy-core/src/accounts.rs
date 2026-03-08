//! Iterator-style account accessor with inline constraint checks.
//!
//! [`AccountList`] provides sequential account consumption with validation,
//! replacing manual index arithmetic.

use pinocchio::{error::ProgramError, AccountView, Address};

use crate::checks::{
    check_account, check_executable, check_signer, check_system_program, check_writable,
};

/// Iterator-style account accessor with inline constraint checks.
///
/// ```rust,ignore
/// let mut accs = AccountList::new(accounts);
/// let payer  = accs.next_signer()?;
/// let vault  = accs.next_writable()?;
/// let system = accs.next_system_program()?;
/// let state  = accs.next_account(program_id, STATE_DISC, STATE_LEN)?;
/// ```
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
    /// discriminator check.
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
    #[inline(always)]
    pub fn next_executable(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_executable(acc)?;
        Ok(acc)
    }

    /// Consume the next account as a writable signer state account.
    #[inline(always)]
    pub fn next_signer_writable_account(
        &mut self,
        program_id: &Address,
        discriminator: u8,
        min_len: usize,
    ) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_signer(acc)?;
        check_writable(acc)?;
        check_account(acc, program_id, discriminator, min_len)?;
        Ok(acc)
    }

    /// Consume the next account and verify it is the Clock sysvar.
    #[cfg(feature = "programs")]
    #[inline(always)]
    pub fn next_clock(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        if *acc.address() != crate::programs::SYSVAR_CLOCK {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(acc)
    }

    /// Consume the next account and verify it is the Sysvar Instructions account.
    #[cfg(feature = "programs")]
    #[inline(always)]
    pub fn next_sysvar_instructions(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        if *acc.address() != crate::programs::SYSVAR_INSTRUCTIONS {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(acc)
    }

    /// Consume the next account and verify it is the Rent sysvar.
    #[cfg(feature = "programs")]
    #[inline(always)]
    pub fn next_rent(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        if *acc.address() != crate::programs::SYSVAR_RENT {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(acc)
    }
}
