//! Account lifecycle helpers: initialization, closure, reallocation.
//!
//! Consolidates safe close, revival detection, and reallocation into
//! a single module. These are the functions you reach for when an account
//! is being created, resized, or destroyed.

use pinocchio::{error::ProgramError, AccountView, ProgramResult};

use crate::math::{checked_add, checked_sub};
use crate::check::rent_exempt_min;

// ── Close ────────────────────────────────────────────────────────────────────

/// Dead sentinel written to the first 8 bytes of a closed account.
pub const CLOSE_SENTINEL: [u8; 8] = [0xFF; 8];

/// Close `account`, sending all its lamports to `destination`.
///
/// Both accounts **must be writable** - caller is responsible for that check.
///
/// # Safety
/// Caller must guarantee no active borrows exist on `account` at call time.
#[inline(always)]
pub fn safe_close(account: &AccountView, destination: &AccountView) -> ProgramResult {
    let lamports = account.lamports();
    let new_dest = checked_add(destination.lamports(), lamports)?;
    destination.set_lamports(new_dest);
    account.set_lamports(0);
    unsafe { account.close_unchecked() };
    Ok(())
}

/// Close `account` with a dead sentinel to prevent revival attacks.
///
/// Writes `[0xFF; 8]` to the first 8 bytes before zeroing lamports and
/// closing. Defends against Sealevel Attack #9 (account revival).
///
/// # Safety
/// Caller must guarantee no active borrows exist on `account` at call time.
#[inline(always)]
pub fn safe_close_with_sentinel(
    account: &AccountView,
    destination: &AccountView,
) -> ProgramResult {
    {
        let mut data = account.try_borrow_mut()?;
        if data.len() >= 8 {
            data[..8].copy_from_slice(&CLOSE_SENTINEL);
        }
    }
    let lamports = account.lamports();
    let new_dest = checked_add(destination.lamports(), lamports)?;
    destination.set_lamports(new_dest);
    account.set_lamports(0);
    unsafe { account.close_unchecked() };
    Ok(())
}

/// Check that an account has not been revived after closure.
///
/// Returns `InvalidAccountData` if the first 8 bytes match the dead sentinel.
#[inline(always)]
pub fn check_not_revived(account: &AccountView) -> ProgramResult {
    let data = account.try_borrow()?;
    if data.len() >= 8 && data[..8] == CLOSE_SENTINEL {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Combined check: account is not revived AND has the expected discriminator.
#[inline(always)]
pub fn check_alive(account: &AccountView, discriminator: u8) -> ProgramResult {
    let data = account.try_borrow()?;
    if data.len() >= 8 && data[..8] == CLOSE_SENTINEL {
        return Err(ProgramError::InvalidAccountData);
    }
    if data.is_empty() || data[0] != discriminator {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── Realloc ──────────────────────────────────────────────────────────────────

/// Resize an account and top up lamports from `payer` to maintain rent exemption.
///
/// Both `account` and `payer` must be writable. `payer` must be a signer.
#[inline(always)]
pub fn safe_realloc(
    account: &AccountView,
    new_size: usize,
    payer: &AccountView,
) -> ProgramResult {
    let old_size = account.data_len();
    account.resize(new_size)?;

    let old_rent = rent_exempt_min(old_size);
    let new_rent = rent_exempt_min(new_size);

    if new_rent > old_rent {
        let diff = checked_sub(new_rent, old_rent)?;
        let new_payer_lamports = checked_sub(payer.lamports(), diff)?;
        let new_account_lamports = checked_add(account.lamports(), diff)?;
        payer.set_lamports(new_payer_lamports);
        account.set_lamports(new_account_lamports);
    } else if new_rent < old_rent {
        let diff = checked_sub(old_rent, new_rent)?;
        let new_payer_lamports = checked_add(payer.lamports(), diff)?;
        let new_account_lamports = checked_sub(account.lamports(), diff)?;
        payer.set_lamports(new_payer_lamports);
        account.set_lamports(new_account_lamports);
    }

    Ok(())
}

/// Resize an account without a payer. Only allows shrinking.
///
/// Returns excess rent lamports to `destination`. Fails if `new_size`
/// is larger than the current size.
#[inline(always)]
pub fn safe_realloc_shrink(
    account: &AccountView,
    new_size: usize,
    destination: &AccountView,
) -> ProgramResult {
    let old_size = account.data_len();
    if new_size > old_size {
        return Err(ProgramError::InvalidArgument);
    }

    account.resize(new_size)?;

    let old_rent = rent_exempt_min(old_size);
    let new_rent = rent_exempt_min(new_size);
    if old_rent > new_rent {
        let diff = checked_sub(old_rent, new_rent)?;
        let new_dest = checked_add(destination.lamports(), diff)?;
        let new_acc = checked_sub(account.lamports(), diff)?;
        destination.set_lamports(new_dest);
        account.set_lamports(new_acc);
    }

    Ok(())
}

// ── Zero-init ────────────────────────────────────────────────────────────────

/// Zero-fill account data. Call before first write to a newly created account.
#[inline(always)]
pub fn zero_init(data: &mut [u8]) {
    super::cursor::zero_init(data);
}
