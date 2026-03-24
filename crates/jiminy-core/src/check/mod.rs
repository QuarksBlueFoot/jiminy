//! Account and instruction validation checks.
//!
//! Every function returns `ProgramResult`: `Ok(())` on pass,
//! an appropriate `ProgramError` variant on failure.

pub mod pda;

use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

#[cfg(feature = "programs")]
use crate::programs;

// ── Identity & permissions ───────────────────────────────────────────────────

/// The canonical system program address (all-zero pubkey).
const SYSTEM_PROGRAM_ID: Address = Address::new_from_array([0u8; 32]);

/// Verify the account signed the transaction.
#[inline(always)]
pub fn check_signer(account: &AccountView) -> ProgramResult {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// Verify the account is marked writable in the transaction.
#[inline(always)]
pub fn check_writable(account: &AccountView) -> ProgramResult {
    if !account.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify the account is owned by `program_id`.
#[inline(always)]
pub fn check_owner(account: &AccountView, program_id: &Address) -> ProgramResult {
    if !account.owned_by(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify the account's address equals the expected PDA.
#[inline(always)]
pub fn check_pda(account: &AccountView, expected: &Address) -> ProgramResult {
    if *account.address() != *expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}

/// Verify the account is the canonical system program.
#[inline(always)]
pub fn check_system_program(account: &AccountView) -> ProgramResult {
    if *account.address() != SYSTEM_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify the account has no data (uninitialized). Prevents reinitialization attacks.
#[inline(always)]
pub fn check_uninitialized(account: &AccountView) -> ProgramResult {
    if !account.is_data_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}

/// Verify the account is an executable program.
#[inline(always)]
pub fn check_executable(account: &AccountView) -> ProgramResult {
    if !account.executable() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

// ── Data shape ───────────────────────────────────────────────────────────────

/// Verify account data is at least `min_len` bytes.
#[inline(always)]
pub fn check_size(data: &[u8], min_len: usize) -> ProgramResult {
    if data.len() < min_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(())
}

/// Verify the first byte of account data matches the expected discriminator.
#[inline(always)]
pub fn check_discriminator(data: &[u8], expected: u8) -> ProgramResult {
    if data.is_empty() || data[0] != expected {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Combined check: ownership + minimum size + discriminator.
#[inline(always)]
pub fn check_account(
    account: &AccountView,
    program_id: &Address,
    discriminator: u8,
    min_len: usize,
) -> ProgramResult {
    check_owner(account, program_id)?;
    let data = account.try_borrow()?;
    check_size(&data, min_len)?;
    check_discriminator(&data, discriminator)?;
    Ok(())
}

/// Verify the header version byte (`data[1]`) meets a minimum version.
#[inline(always)]
pub fn check_version(data: &[u8], min_version: u8) -> ProgramResult {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[1] < min_version {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── Keys & addresses ─────────────────────────────────────────────────────────

/// Verify two addresses are equal.
#[inline(always)]
pub fn check_keys_eq(a: &Address, b: &Address) -> ProgramResult {
    if *a != *b {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify that a stored address field matches an account's actual address.
///
/// Runtime equivalent of Anchor's `has_one` constraint.
#[inline(always)]
pub fn check_has_one(stored: &Address, account: &AccountView) -> ProgramResult {
    if stored != account.address() {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

// ── Rent & lamports ──────────────────────────────────────────────────────────

/// Approximate minimum lamports for rent exemption at the current mainnet rate.
///
/// Formula: `(128 + data_len) * 6960`
#[inline(always)]
pub fn rent_exempt_min(data_len: usize) -> u64 {
    (128u64 + data_len as u64).saturating_mul(6960)
}

/// Verify an account holds enough lamports to be rent-exempt for its data size.
#[inline(always)]
pub fn check_rent_exempt(account: &AccountView) -> ProgramResult {
    let data = account.try_borrow()?;
    let min = rent_exempt_min(data.len());
    drop(data);
    if account.lamports() < min {
        return Err(ProgramError::InsufficientFunds);
    }
    Ok(())
}

/// Verify `account` holds at least `min_lamports`.
#[inline(always)]
pub fn check_lamports_gte(account: &AccountView, min_lamports: u64) -> ProgramResult {
    if account.lamports() < min_lamports {
        return Err(ProgramError::InsufficientFunds);
    }
    Ok(())
}

/// Verify an account is fully closed: zero lamports and empty data.
#[inline(always)]
pub fn check_closed(account: &AccountView) -> ProgramResult {
    if account.lamports() != 0 || !account.is_data_empty() {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── Instruction data ─────────────────────────────────────────────────────────

/// Verify instruction data is exactly the expected length.
#[inline(always)]
pub fn check_instruction_data_len(data: &[u8], expected_len: usize) -> ProgramResult {
    if data.len() != expected_len {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

/// Verify instruction data has at least N bytes.
#[inline(always)]
pub fn check_instruction_data_min(data: &[u8], min_len: usize) -> ProgramResult {
    if data.len() < min_len {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

// ── Uniqueness ───────────────────────────────────────────────────────────────

/// Verify two accounts have different addresses.
#[inline(always)]
pub fn check_accounts_unique_2(a: &AccountView, b: &AccountView) -> ProgramResult {
    if a.address() == b.address() {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify three accounts all have different addresses.
#[inline(always)]
pub fn check_accounts_unique_3(
    a: &AccountView,
    b: &AccountView,
    c: &AccountView,
) -> ProgramResult {
    if a.address() == b.address()
        || a.address() == c.address()
        || b.address() == c.address()
    {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify four accounts all have different addresses.
#[inline(always)]
pub fn check_accounts_unique_4(
    a: &AccountView,
    b: &AccountView,
    c: &AccountView,
    d: &AccountView,
) -> ProgramResult {
    if a.address() == b.address()
        || a.address() == c.address()
        || a.address() == d.address()
        || b.address() == c.address()
        || b.address() == d.address()
        || c.address() == d.address()
    {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

// ── Assert helpers (folded from asserts.rs) ──────────────────────────────────

/// Derive a PDA from seeds, verify it matches the account, return the bump.
///
/// Calls `find_program_address` (syscall on-chain).
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

/// Verify a PDA matches when the bump is already known. Cheaper, single derivation.
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
#[inline(always)]
pub fn assert_pda_external(
    account: &AccountView,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<u8, ProgramError> {
    assert_pda(account, seeds, program_id)
}

/// Verify the account is the SPL Token program (Token or Token-2022).
#[cfg(feature = "programs")]
#[inline(always)]
pub fn assert_token_program(account: &AccountView) -> ProgramResult {
    if *account.address() != programs::TOKEN && *account.address() != programs::TOKEN_2022 {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify an account's address matches an expected address exactly.
#[inline(always)]
pub fn assert_address(account: &AccountView, expected: &Address) -> ProgramResult {
    if *account.address() != *expected {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify an account's address matches a known program id + is executable.
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

/// Verify an account has never been initialized (lamports == 0).
#[inline(always)]
pub fn assert_not_initialized(account: &AccountView) -> ProgramResult {
    if account.lamports() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}

// ── Program allowlist ────────────────────────────────────────────────────────

/// Verify the account is owned by one of the programs in `allowed`.
///
/// Use this when accepting accounts from a known set of trusted programs.
/// Rejects anything not in the list.
///
/// ```rust,ignore
/// const ALLOWED: &[Address] = &[PROGRAM_A, PROGRAM_B];
/// check_program_allowed(account, ALLOWED)?;
/// ```
#[inline(always)]
pub fn check_program_allowed(
    account: &AccountView,
    allowed: &[Address],
) -> ProgramResult {
    let owner = unsafe { account.owner() };
    let mut i = 0;
    while i < allowed.len() {
        if *owner == allowed[i] {
            return Ok(());
        }
        i += 1;
    }
    Err(ProgramError::IncorrectProgramId)
}
