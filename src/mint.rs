//! Zero-copy readers and checks for SPL Token mint accounts.
//!
//! SPL Token mints are 82 bytes with a fixed layout. These functions
//! read fields directly from raw account data without deserialization,
//! borsh, or any allocator.
//!
//! Layout (SPL Token mint, 82 bytes):
//! ```text
//!  0..4    mint_authority (Option tag, u32: 0 = None, 1 = Some)
//!  4..36   mint_authority key (Address, if present)
//! 36..44   supply        (u64 LE)
//! 44       decimals      (u8)
//! 45       is_initialized (bool: 0/1)
//! 46..50   freeze_authority (Option tag, u32)
//! 50..82   freeze_authority key (Address, if present)
//! ```

use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

/// Minimum size of an SPL Token mint account.
pub const MINT_LEN: usize = 82;

/// Read the mint authority field (bytes 0..36).
///
/// Returns `Some(&Address)` if a mint authority is set, `None` otherwise.
/// The mint authority can mint new tokens for this mint.
///
/// ```rust,ignore
/// let authority = mint_authority(mint_account)?;
/// if let Some(auth) = authority {
///     require_keys_eq!(auth, program_pda, MyError::WrongMintAuthority);
/// }
/// ```
#[inline(always)]
pub fn mint_authority(account: &AccountView) -> Result<Option<&Address>, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < MINT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let tag = u32::from_le_bytes(
        data[0..4]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    let ptr = data.as_ptr();
    drop(data);
    if tag == 0 {
        Ok(None)
    } else {
        Ok(Some(unsafe { &*(ptr.add(4) as *const Address) }))
    }
}

/// Read the total supply field (bytes 36..44).
///
/// Returns the total number of tokens minted for this mint.
///
/// ```rust,ignore
/// let supply = mint_supply(mint_account)?;
/// require!(supply <= MAX_SUPPLY, MyError::SupplyExceeded);
/// ```
#[inline(always)]
pub fn mint_supply(account: &AccountView) -> Result<u64, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < MINT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let val = u64::from_le_bytes(
        data[36..44]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok(val)
}

/// Read the decimals field (byte 44).
///
/// Returns the number of decimal places for this mint (0-255).
///
/// ```rust,ignore
/// let decimals = mint_decimals(mint_account)?;
/// let scale = 10u64.pow(decimals as u32);
/// ```
#[inline(always)]
pub fn mint_decimals(account: &AccountView) -> Result<u8, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < MINT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(data[44])
}

/// Check if the mint is initialized (byte 45).
///
/// Returns `true` if `data[45] != 0`.
///
/// ```rust,ignore
/// require!(mint_is_initialized(mint_account)?, MyError::MintNotInitialized);
/// ```
#[inline(always)]
pub fn mint_is_initialized(account: &AccountView) -> Result<bool, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < MINT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(data[45] != 0)
}

/// Read the freeze authority field (bytes 46..82).
///
/// Returns `Some(&Address)` if a freeze authority is set, `None` otherwise.
/// A freeze authority can freeze any token account for this mint, blocking
/// transfers out.
///
/// DeFi programs should be aware of mints with freeze authorities. A
/// malicious authority could freeze pool token accounts, locking funds.
///
/// ```rust,ignore
/// let freeze_auth = mint_freeze_authority(mint_account)?;
/// require!(freeze_auth.is_none(), MyError::MintHasFreezeAuthority);
/// ```
#[inline(always)]
pub fn mint_freeze_authority(account: &AccountView) -> Result<Option<&Address>, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < MINT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let tag = u32::from_le_bytes(
        data[46..50]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    let ptr = data.as_ptr();
    drop(data);
    if tag == 0 {
        Ok(None)
    } else {
        Ok(Some(unsafe { &*(ptr.add(50) as *const Address) }))
    }
}

/// Verify a mint account is owned by the expected token program.
///
/// Token-2022 mints are owned by the Token-2022 program, while classic
/// SPL mints are owned by the original Token program. Passing the wrong
/// token program in a CPI will fail or produce exploitable behavior.
///
/// ```rust,ignore
/// check_mint_owner(mint_account, &programs::TOKEN)?;
/// ```
#[inline(always)]
pub fn check_mint_owner(account: &AccountView, token_program: &Address) -> ProgramResult {
    if !account.owned_by(token_program) {
        return Err(ProgramError::IncorrectProgramId);
    }
    let data = account.try_borrow()?;
    if data.len() < MINT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(())
}

/// Verify the mint authority matches an expected address.
///
/// Use this before minting tokens to confirm your program's PDA is the
/// authorized minter. Errors if the mint has no authority or the authority
/// doesn't match.
///
/// ```rust,ignore
/// check_mint_authority(mint_account, &my_program_pda)?;
/// ```
#[inline(always)]
pub fn check_mint_authority(account: &AccountView, expected: &Address) -> ProgramResult {
    match mint_authority(account)? {
        Some(auth) if auth == expected => Ok(()),
        _ => Err(ProgramError::InvalidArgument),
    }
}
