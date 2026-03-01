//! Zero-copy readers for SPL Token account fields.
//!
//! SPL Token accounts are 165 bytes with a fixed layout. These functions
//! read fields directly from raw account data without deserialization,
//! borsh, or any allocator.
//!
//! Layout (SPL Token account, 165 bytes):
//! ```text
//!  0..32   mint          (Address)
//! 32..64   owner         (Address)
//! 64..72   amount        (u64 LE)
//! 72..76   delegate      (Option tag, u32)
//! 76..108  delegate key  (Address, if present)
//! 108..109 state         (u8: 0=uninitialized, 1=initialized, 2=frozen)
//! 109..113 is_native     (Option tag, u32)
//! 113..121 native amount (u64 LE, if present)
//! 121..129 delegated_amount (u64 LE)
//! 129..133 close_authority (Option tag, u32)
//! 133..165 close_authority key (Address, if present)
//! ```

use pinocchio::{error::ProgramError, AccountView, Address};

/// Minimum size of an SPL Token account.
pub const TOKEN_ACCOUNT_LEN: usize = 165;

/// Read the owner field from a token account (bytes 32..64).
///
/// Returns the 32-byte owner address without copying or deserializing.
/// Fails if account data is too small.
///
/// ```rust,ignore
/// let owner = token_account_owner(token_account)?;
/// require_keys_eq!(owner, authority.address(), ProgramError::InvalidArgument);
/// ```
#[inline(always)]
pub fn token_account_owner(account: &AccountView) -> Result<&Address, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    // SAFETY: data is borrowed and lives as long as the AccountView.
    // We return a reference into account data via pointer cast.
    // The borrow is dropped but the underlying data is pinned by the runtime.
    let ptr = data.as_ptr();
    drop(data);
    Ok(unsafe { &*(ptr.add(32) as *const Address) })
}

/// Read the amount field from a token account (bytes 64..72).
///
/// Returns the u64 token balance without copying or deserializing.
///
/// ```rust,ignore
/// let amount = token_account_amount(token_account)?;
/// require_gte!(amount, min_collateral, MyError::Undercollateralized);
/// ```
#[inline(always)]
pub fn token_account_amount(account: &AccountView) -> Result<u64, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let amount = u64::from_le_bytes(
        data[64..72]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok(amount)
}

/// Read the mint field from a token account (bytes 0..32).
///
/// Returns a reference to the 32-byte mint address.
///
/// ```rust,ignore
/// let mint = token_account_mint(token_account)?;
/// require_keys_eq!(mint, &expected_mint, MyError::WrongMint);
/// ```
#[inline(always)]
pub fn token_account_mint(account: &AccountView) -> Result<&Address, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let ptr = data.as_ptr();
    drop(data);
    Ok(unsafe { &*(ptr as *const Address) })
}

/// Read the delegate field from a token account (bytes 76..108).
///
/// Returns `Some(&Address)` if a delegate is set, `None` otherwise.
///
/// ```rust,ignore
/// if let Some(delegate) = token_account_delegate(token_account)? {
///     // handle delegated token account
/// }
/// ```
#[inline(always)]
pub fn token_account_delegate(account: &AccountView) -> Result<Option<&Address>, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let tag = u32::from_le_bytes(
        data[72..76]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    let ptr = data.as_ptr();
    drop(data);
    if tag == 0 {
        Ok(None)
    } else {
        Ok(Some(unsafe { &*(ptr.add(76) as *const Address) }))
    }
}
