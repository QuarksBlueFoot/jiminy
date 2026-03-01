use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

use crate::programs;

/// Derive a PDA from the given seeds and program id, verify it matches the
/// account's address, and return the bump.
///
/// Uses `find_program_address` under the hood (syscall on-chain). Returns
/// the canonical bump if the derived address matches. Errors with
/// `InvalidSeeds` if no bump produces a match.
///
/// This is the pinocchio equivalent of Anchor's `seeds + bump` constraint
/// but you get the bump back for storage or CPI signing.
///
/// ```rust,ignore
/// let bump = assert_pda(vault, &[b"vault", authority.as_ref()], program_id)?;
/// ```
#[inline(always)]
pub fn assert_pda(
    account: &AccountView,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<u8, ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let (derived, bump) = Address::find_program_address(seeds, program_id);
        if derived != *account.address() {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(bump)
    }
    #[cfg(not(target_os = "solana"))]
    {
        let _ = (account, seeds, program_id);
        Err(ProgramError::InvalidSeeds)
    }
}

/// Verify a PDA matches when the bump is already known. Way cheaper than
/// [`assert_pda`] because it only does one derivation instead of searching
/// all 256 bumps.
///
/// Use this when the bump is stored in account data or passed as
/// instruction data.
///
/// ```rust,ignore
/// assert_pda_with_bump(vault, &[b"vault", authority.as_ref()], bump, program_id)?;
/// ```
#[inline(always)]
pub fn assert_pda_with_bump(
    account: &AccountView,
    seeds: &[&[u8]],
    bump: u8,
    program_id: &Address,
) -> ProgramResult {
    #[cfg(target_os = "solana")]
    {
        let bump_bytes = [bump];
        let n = seeds.len();
        // Build seeds + bump slice. Max 16 seeds + 1 bump.
        let mut all_seeds: [&[u8]; 17] = [&[]; 17];
        let mut i = 0;
        while i < n {
            all_seeds[i] = seeds[i];
            i += 1;
        }
        all_seeds[n] = &bump_bytes;

        let derived = Address::create_program_address(&all_seeds[..n + 1], program_id)
            .map_err(|_| ProgramError::InvalidSeeds)?;
        if derived != *account.address() {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(())
    }
    #[cfg(not(target_os = "solana"))]
    {
        let _ = (account, seeds, bump, program_id);
        Err(ProgramError::InvalidSeeds)
    }
}

/// Verify a PDA derived from an external program's seeds. Returns the bump.
///
/// Same as [`assert_pda`] but makes intent clear: the program_id belongs
/// to another program (metadata, ATA, etc.), not yours.
///
/// ```rust,ignore
/// let bump = assert_pda_external(
///     metadata_account,
///     &[b"metadata", metadata_program.as_ref(), mint.as_ref()],
///     &programs::METADATA,
/// )?;
/// ```
#[inline(always)]
pub fn assert_pda_external(
    account: &AccountView,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<u8, ProgramError> {
    assert_pda(account, seeds, program_id)
}

/// Verify the account is the SPL Token program.
///
/// ```rust,ignore
/// assert_token_program(token_program_account)?;
/// ```
#[inline(always)]
pub fn assert_token_program(account: &AccountView) -> ProgramResult {
    if *account.address() != programs::TOKEN && *account.address() != programs::TOKEN_2022 {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify an account's address matches an expected address exactly.
///
/// Useful for singletons, well-known accounts, or config accounts
/// that must be a specific pubkey.
///
/// ```rust,ignore
/// assert_address(config_account, &EXPECTED_CONFIG_KEY)?;
/// ```
#[inline(always)]
pub fn assert_address(account: &AccountView, expected: &Address) -> ProgramResult {
    if *account.address() != *expected {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify an account's address matches a known program id.
///
/// Combines address check + executable check. Use this when your
/// instruction receives a program account for CPI and you need to
/// confirm it's the right one.
///
/// ```rust,ignore
/// assert_program(token_program, &programs::TOKEN)?;
/// ```
#[inline(always)]
pub fn assert_program(account: &AccountView, expected_program: &Address) -> ProgramResult {
    if *account.address() != *expected_program {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !account.executable() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify an account has never been initialized by checking that its
/// lamports are zero.
///
/// Different from [`check_uninitialized`](crate::check_uninitialized) which
/// checks for empty data. This checks lamports == 0, meaning the account
/// doesn't exist on-chain yet. Useful for create-if-not-exists patterns
/// where you want to confirm the account hasn't been funded.
///
/// ```rust,ignore
/// assert_not_initialized(new_vault)?;
/// ```
#[inline(always)]
pub fn assert_not_initialized(account: &AccountView) -> ProgramResult {
    if account.lamports() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}
