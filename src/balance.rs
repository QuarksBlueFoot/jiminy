//! Balance delta checks for CPI composition.
//!
//! When your program CPIs into an AMM or lending protocol, you can't trust
//! return values. Snapshot balances before the CPI, re-read after, and
//! assert the delta is what you expect.
//!
//! ```rust,ignore
//! // 1. Snapshot balance before CPI
//! let before = snapshot_token_balance(vault)?;
//!
//! // 2. CPI into AMM
//! safe_transfer_tokens(...)?;
//!
//! // 3. Verify balance changed correctly
//! check_balance_increased(vault, before, min_expected)?;
//! ```

use pinocchio::{error::ProgramError, AccountView, ProgramResult};

use crate::token::TOKEN_ACCOUNT_LEN;

/// Snapshot the current token balance from a token account.
///
/// Reads amount at bytes 64..72 of the SPL Token layout. Call this
/// **before** a CPI, then compare after.
///
/// ```rust,ignore
/// let before = snapshot_token_balance(vault)?;
/// ```
#[inline(always)]
pub fn snapshot_token_balance(account: &AccountView) -> Result<u64, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(u64::from_le_bytes([
        data[64], data[65], data[66], data[67], data[68], data[69], data[70], data[71],
    ]))
}

/// Snapshot the current lamport balance of any account.
///
/// ```rust,ignore
/// let before = snapshot_lamport_balance(account);
/// ```
#[inline(always)]
pub fn snapshot_lamport_balance(account: &AccountView) -> u64 {
    account.lamports()
}

/// Verify a token account balance increased by at least `min_increase` since the snapshot.
///
/// Call after a CPI that should have deposited tokens. Returns
/// `InvalidAccountData` if the increase is less than expected.
///
/// ```rust,ignore
/// check_balance_increased(vault, balance_before, min_output)?;
/// ```
#[inline(always)]
pub fn check_balance_increased(
    account: &AccountView,
    balance_before: u64,
    min_increase: u64,
) -> ProgramResult {
    let current = snapshot_token_balance(account)?;
    if current < balance_before {
        return Err(ProgramError::InvalidAccountData);
    }
    let delta = current - balance_before;
    if delta < min_increase {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify a token account balance decreased by at most `max_decrease` since the snapshot.
///
/// Call after a CPI that should have withdrawn tokens. Returns
/// `InvalidAccountData` if more was taken than expected.
///
/// ```rust,ignore
/// check_balance_decreased(vault, balance_before, max_cost)?;
/// ```
#[inline(always)]
pub fn check_balance_decreased(
    account: &AccountView,
    balance_before: u64,
    max_decrease: u64,
) -> ProgramResult {
    let current = snapshot_token_balance(account)?;
    if current > balance_before {
        return Err(ProgramError::InvalidAccountData);
    }
    let delta = balance_before - current;
    if delta > max_decrease {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify a token balance delta is within tolerance of the expected amount.
///
/// `tolerance_bps` is the acceptable deviation in basis points.
/// For example, `tolerance_bps = 50` allows 0.5% deviation.
///
/// ```rust,ignore
/// check_balance_delta(vault, before, expected_output, 50)?; // 0.5% tolerance
/// ```
#[inline(always)]
pub fn check_balance_delta(
    account: &AccountView,
    balance_before: u64,
    expected_delta: u64,
    tolerance_bps: u16,
) -> ProgramResult {
    let current = snapshot_token_balance(account)?;
    let actual_delta = if current >= balance_before {
        current - balance_before
    } else {
        balance_before - current
    };

    // |actual - expected| * 10_000 <= expected * tolerance_bps
    let diff = if actual_delta >= expected_delta {
        actual_delta - expected_delta
    } else {
        expected_delta - actual_delta
    };

    let max_allowed = (expected_delta as u128)
        .checked_mul(tolerance_bps as u128)
        .unwrap_or(u128::MAX)
        / 10_000;

    if diff as u128 > max_allowed {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify a lamport balance increased by at least `min_increase` since the snapshot.
#[inline(always)]
pub fn check_lamport_balance_increased(
    account: &AccountView,
    balance_before: u64,
    min_increase: u64,
) -> ProgramResult {
    let current = account.lamports();
    if current < balance_before {
        return Err(ProgramError::InvalidAccountData);
    }
    let delta = current - balance_before;
    if delta < min_increase {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
