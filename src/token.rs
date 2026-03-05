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

use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

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

/// Read the state byte from a token account (byte 108).
///
/// Returns the raw state value:
/// - `0` = Uninitialized
/// - `1` = Initialized
/// - `2` = Frozen
///
/// ```rust,ignore
/// let state = token_account_state(token_account)?;
/// require_eq!(state, 1, MyError::TokenAccountNotInitialized);
/// ```
#[inline(always)]
pub fn token_account_state(account: &AccountView) -> Result<u8, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(data[108])
}

/// Read the close authority field from a token account (bytes 129..165).
///
/// Returns `Some(&Address)` if a close authority is set, `None` otherwise.
/// An unexpected close authority can drain the token account by closing it.
///
/// ```rust,ignore
/// let close_auth = token_account_close_authority(token_account)?;
/// require!(close_auth.is_none(), MyError::UnexpectedCloseAuthority);
/// ```
#[inline(always)]
pub fn token_account_close_authority(
    account: &AccountView,
) -> Result<Option<&Address>, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let tag = u32::from_le_bytes(
        data[129..133]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    let ptr = data.as_ptr();
    drop(data);
    if tag == 0 {
        Ok(None)
    } else {
        Ok(Some(unsafe { &*(ptr.add(133) as *const Address) }))
    }
}

/// Read the delegated amount from a token account (bytes 121..129).
///
/// Returns the number of tokens delegated to the delegate address.
/// Non-zero only when a delegate is set.
///
/// ```rust,ignore
/// let delegated = token_account_delegated_amount(token_account)?;
/// ```
#[inline(always)]
pub fn token_account_delegated_amount(account: &AccountView) -> Result<u64, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let val = u64::from_le_bytes(
        data[121..129]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok(val)
}

// ── Token Account Assertions ─────────────────────────────────────────────────
//
// Composable single-line checks that combine a reader + comparison.
// These are the Jiminy equivalents of Anchor's `token::mint` and
// `token::authority` constraints.

/// Verify a token account's mint matches the expected mint address.
///
/// This is the #1 most-exploited missing check in Solana DeFi: without it,
/// an attacker passes a token account for the wrong mint. Equivalent to
/// Anchor's `token::mint = expected_mint` constraint.
///
/// ```rust,ignore
/// check_token_account_mint(user_token, &usdc_mint)?;
/// ```
#[inline(always)]
pub fn check_token_account_mint(
    account: &AccountView,
    expected_mint: &Address,
) -> ProgramResult {
    let mint = token_account_mint(account)?;
    if mint != expected_mint {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify a token account's owner matches the expected authority.
///
/// Equivalent to Anchor's `token::authority = expected_authority`.
/// Use this to confirm the token account belongs to the correct wallet
/// or PDA before transferring tokens out of it.
///
/// ```rust,ignore
/// check_token_account_owner(user_token, user.address())?;
/// ```
#[inline(always)]
pub fn check_token_account_owner(
    account: &AccountView,
    expected_owner: &Address,
) -> ProgramResult {
    let owner = token_account_owner(account)?;
    if owner != expected_owner {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify a token account is in the `Initialized` state (state byte == 1).
///
/// Rejects both uninitialized (0) and frozen (2) accounts. Frozen token
/// accounts will cause CPI transfers to fail opaquely — checking state
/// upfront gives a clear error.
///
/// ```rust,ignore
/// check_token_account_initialized(user_token)?;
/// ```
#[inline(always)]
pub fn check_token_account_initialized(account: &AccountView) -> ProgramResult {
    let state = token_account_state(account)?;
    if state != 1 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify a token account is frozen (state byte == 2).
///
/// Use this when you need to confirm an account IS frozen (e.g., for an
/// unfreeze instruction).
///
/// ```rust,ignore
/// check_token_account_frozen(escrow_token)?;
/// ```
#[inline(always)]
pub fn check_token_account_frozen(account: &AccountView) -> ProgramResult {
    let state = token_account_state(account)?;
    if state != 2 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Reject frozen token accounts.
///
/// Transfers to/from frozen accounts will fail at the token program level,
/// but checking upfront saves CU and gives a clearer error. Use this
/// before any transfer, burn, or close CPI.
///
/// ```rust,ignore
/// check_not_frozen(source_token)?;
/// check_not_frozen(dest_token)?;
/// safe_transfer_tokens(source_token, dest_token, authority, amount)?;
/// ```
#[inline(always)]
pub fn check_not_frozen(account: &AccountView) -> ProgramResult {
    let state = token_account_state(account)?;
    if state == 2 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Reject token accounts that have an active delegate.
///
/// Delegated token accounts can have funds pulled by the delegate at any
/// time. DeFi vaults and escrows should reject deposits from delegated
/// accounts to prevent unexpected fund movement.
///
/// ```rust,ignore
/// check_no_delegate(deposit_token)?;
/// ```
#[inline(always)]
pub fn check_no_delegate(account: &AccountView) -> ProgramResult {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let tag = u32::from_le_bytes(
        data[72..76]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    if tag != 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Reject token accounts that have a close authority set.
///
/// An unexpected close authority allows someone to close the token account
/// and steal the rent-exempt lamports (and implicitly delete the token
/// balance). Use this to ensure the account can't be closed out from
/// under your program.
///
/// ```rust,ignore
/// check_no_close_authority(vault_token)?;
/// ```
#[inline(always)]
pub fn check_no_close_authority(account: &AccountView) -> ProgramResult {
    let data = account.try_borrow()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let tag = u32::from_le_bytes(
        data[129..133]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    if tag != 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify that a token account's owning program matches the passed token program.
///
/// Critical for Token-2022 support: the token account must be owned by the
/// same token program you're passing to CPI transfers. Mismatches cause
/// silent failures or exploits.
///
/// ```rust,ignore
/// check_token_program_match(user_token, token_program)?;
/// ```
#[inline(always)]
pub fn check_token_program_match(
    account: &AccountView,
    token_program: &AccountView,
) -> ProgramResult {
    if !account.owned_by(token_program.address()) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify a token account holds at least `min_amount` tokens.
///
/// Common pre-transfer check to ensure sufficient balance before
/// attempting a CPI transfer that would otherwise fail inside the
/// token program.
///
/// ```rust,ignore
/// check_token_balance_gte(source_token, transfer_amount)?;
/// ```
#[inline(always)]
pub fn check_token_balance_gte(account: &AccountView, min_amount: u64) -> ProgramResult {
    let amount = token_account_amount(account)?;
    if amount < min_amount {
        return Err(ProgramError::InsufficientFunds);
    }
    Ok(())
}
