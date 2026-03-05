use pinocchio::{error::ProgramError, AccountView, Address};

use crate::checks::{
    check_account, check_executable, check_signer, check_system_program, check_writable,
};
use crate::token::{check_token_account_mint, check_token_account_owner, TOKEN_ACCOUNT_LEN};
use crate::mint::MINT_LEN;

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

    /// Consume the next account as a writable signer state account.
    ///
    /// Combines signer + writable + ownership + size + discriminator checks.
    /// The full equivalent of Anchor's `#[account(mut, signer)]` for a
    /// program-owned state account.
    ///
    /// ```rust,ignore
    /// let state = accs.next_signer_writable_account(program_id, STATE_DISC, STATE_LEN)?;
    /// ```
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

    /// Consume the next account as a validated token account.
    ///
    /// Verifies: data size ≥ 165, mint matches, owner matches. This is
    /// the zero-copy equivalent of Anchor's:
    /// ```text
    /// #[account(token::mint = expected_mint, token::authority = expected_owner)]
    /// ```
    ///
    /// ```rust,ignore
    /// let user_token = accs.next_token_account(&usdc_mint, user.address())?;
    /// ```
    #[inline(always)]
    pub fn next_token_account(
        &mut self,
        expected_mint: &Address,
        expected_owner: &Address,
    ) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        // Verify data is large enough for a token account.
        let data = acc.try_borrow()?;
        if data.len() < TOKEN_ACCOUNT_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        drop(data);
        check_token_account_mint(acc, expected_mint)?;
        check_token_account_owner(acc, expected_owner)?;
        Ok(acc)
    }

    /// Consume the next account as a validated mint.
    ///
    /// Verifies: data size ≥ 82 and owned by the expected token program.
    ///
    /// ```rust,ignore
    /// let mint = accs.next_mint(&programs::TOKEN)?;
    /// ```
    #[inline(always)]
    pub fn next_mint(
        &mut self,
        expected_token_program: &Address,
    ) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        if !acc.owned_by(expected_token_program) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let data = acc.try_borrow()?;
        if data.len() < MINT_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(acc)
    }

    /// Consume the next account and verify it is the Clock sysvar.
    ///
    /// ```rust,ignore
    /// let clock = accs.next_clock()?;
    /// let (slot, timestamp) = jiminy::sysvar::read_clock(clock)?;
    /// ```
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
    ///
    /// Required for CPI guard checks (`check_no_cpi_caller`).
    ///
    /// ```rust,ignore
    /// let sysvar_ix = accs.next_sysvar_instructions()?;
    /// jiminy::cpi_guard::check_no_cpi_caller(sysvar_ix, program_id)?;
    /// ```
    #[cfg(feature = "programs")]
    #[inline(always)]
    pub fn next_sysvar_instructions(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        if *acc.address() != crate::programs::SYSVAR_INSTRUCTIONS {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(acc)
    }

    /// Consume the next account and verify it is one of the two token programs
    /// (SPL Token or Token-2022).
    ///
    /// ```rust,ignore
    /// let token_program = accs.next_token_program()?;
    /// ```
    #[cfg(feature = "programs")]
    #[inline(always)]
    pub fn next_token_program(&mut self) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        if *acc.address() != crate::programs::TOKEN
            && *acc.address() != crate::programs::TOKEN_2022
        {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(acc)
    }

    /// Consume the next account as a validated **writable** token account.
    ///
    /// Verifies: writable + data size >= 165 + mint matches + owner matches.
    /// Use this for destination or source accounts that will be modified.
    ///
    /// ```rust,ignore
    /// let user_ata = accs.next_writable_token_account(&usdc_mint, user.address())?;
    /// ```
    #[inline(always)]
    pub fn next_writable_token_account(
        &mut self,
        expected_mint: &Address,
        expected_owner: &Address,
    ) -> Result<&'a AccountView, ProgramError> {
        let acc = self.next()?;
        check_writable(acc)?;
        let data = acc.try_borrow()?;
        if data.len() < TOKEN_ACCOUNT_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        drop(data);
        check_token_account_mint(acc, expected_mint)?;
        check_token_account_owner(acc, expected_owner)?;
        Ok(acc)
    }

    /// Consume the next account and verify it is the Rent sysvar.
    ///
    /// ```rust,ignore
    /// let rent = accs.next_rent()?;
    /// ```
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
