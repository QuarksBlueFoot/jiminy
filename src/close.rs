use pinocchio::{error::ProgramError, AccountView, ProgramResult};

use crate::math::checked_add;

/// Dead sentinel written to the first 8 bytes of a closed account.
///
/// If an attacker revives the account within the same transaction by
/// transferring lamports back, the sentinel bytes prevent the program
/// from treating it as a valid account on re-entry.
pub const CLOSE_SENTINEL: [u8; 8] = [0xFF; 8];

/// Close `account`, sending all its lamports to `destination`.
///
/// Steps:
/// 1. Overflow-checked addition of lamports into `destination`.
/// 2. Zero `account` lamports.
/// 3. `close_unchecked()`: zeros data and clears the owner field.
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
    // SAFETY: Caller guarantees no active borrows on `account`.
    unsafe { account.close_unchecked() };
    Ok(())
}

/// Close `account` with a dead sentinel to prevent revival attacks.
///
/// Writes `[0xFF; 8]` to the first 8 bytes before zeroing lamports and
/// closing. If an attacker revives the account within the same transaction,
/// the sentinel bytes will be present, allowing `check_not_revived` to
/// detect and reject it.
///
/// This defends against Sealevel Attack #9 (account revival/resurrection).
///
/// ```rust,ignore
/// safe_close_with_sentinel(vault, destination)?;
/// ```
///
/// # Safety
/// Caller must guarantee no active borrows exist on `account` at call time.
#[inline(always)]
pub fn safe_close_with_sentinel(
    account: &AccountView,
    destination: &AccountView,
) -> ProgramResult {
    // Write sentinel before closing
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
    // SAFETY: Caller guarantees no active borrows on `account`.
    unsafe { account.close_unchecked() };
    Ok(())
}

/// Check that an account has not been revived after closure.
///
/// Returns `InvalidAccountData` if the first 8 bytes match the dead sentinel.
/// Use this at the top of any instruction that accepts a previously-closable
/// account to detect revival attacks.
///
/// ```rust,ignore
/// check_not_revived(vault)?;
/// ```
#[inline(always)]
pub fn check_not_revived(account: &AccountView) -> ProgramResult {
    let data = account.try_borrow()?;
    if data.len() >= 8 && data[..8] == CLOSE_SENTINEL {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Combined check: account is not revived AND has the expected discriminator.
///
/// ```rust,ignore
/// check_alive(vault, MY_VAULT_DISC)?;
/// ```
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
