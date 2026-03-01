use pinocchio::{AccountView, ProgramResult};

use crate::math::checked_add;

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
