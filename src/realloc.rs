//! Safe account reallocation with rent top-up.
//!
//! Resizing an account on Solana requires:
//! 1. Calling `resize` to change the data length
//! 2. Topping up lamports to maintain rent exemption
//!
//! Forgetting step 2 means the account becomes rent-liable and will be
//! cleaned up. `safe_realloc` bundles both steps with overflow checks.

use pinocchio::{error::ProgramError, AccountView, ProgramResult};

use crate::checks::rent_exempt_min;
use crate::math::{checked_add, checked_sub};

/// Resize an account and top up lamports from `payer` to maintain rent exemption.
///
/// If `new_size > current_size`, the difference in rent-exempt minimums is
/// transferred from `payer` to `account`. If `new_size < current_size`, excess
/// lamports are returned to `payer`.
///
/// Both `account` and `payer` must be writable. `payer` must be a signer.
///
/// ```rust,ignore
/// // Grow the account from 100 to 200 bytes
/// safe_realloc(state_account, 200, payer)?;
/// ```
#[inline(always)]
pub fn safe_realloc(
    account: &AccountView,
    new_size: usize,
    payer: &AccountView,
) -> ProgramResult {
    let old_size = account.data_len();

    // Resize the data buffer (handles zero-extension for growth).
    account.resize(new_size)?;

    // Compute rent difference.
    let old_rent = rent_exempt_min(old_size);
    let new_rent = rent_exempt_min(new_size);

    if new_rent > old_rent {
        // Growing: transfer additional lamports from payer -> account.
        let diff = checked_sub(new_rent, old_rent)?;
        let new_payer_lamports = checked_sub(payer.lamports(), diff)?;
        let new_account_lamports = checked_add(account.lamports(), diff)?;
        payer.set_lamports(new_payer_lamports);
        account.set_lamports(new_account_lamports);
    } else if new_rent < old_rent {
        // Shrinking: return excess lamports to payer.
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
/// is larger than the current size (use [`safe_realloc`] with a payer
/// for growth).
///
/// ```rust,ignore
/// // Shrink after removing a field, return excess rent to authority
/// safe_realloc_shrink(state_account, 80, authority)?;
/// ```
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
    let diff = checked_sub(old_rent, new_rent)?;

    if diff > 0 {
        let new_dest = checked_add(destination.lamports(), diff)?;
        let new_account = checked_sub(account.lamports(), diff)?;
        destination.set_lamports(new_dest);
        account.set_lamports(new_account);
    }

    Ok(())
}
