//! Two-step authority handoff (propose + accept).
//!
//! The standard pattern for safe authority rotation: current authority
//! writes a `pending_authority` field, then the new authority calls
//! `accept` to finalize. Prevents fat-finger transfers to the wrong key.
//!
//! Every serious DeFi protocol uses this pattern. Nobody in the pinocchio
//! ecosystem provides check functions for it. Now you get it for free.
//!
//! ## Account layout assumption
//!
//! Your account stores:
//! - `authority` at some byte offset (32 bytes)
//! - `pending_authority` at some byte offset (32 bytes)
//!
//! You tell us the offsets. We read zero-copy from the account data.
//!
//! ```rust,ignore
//! // In propose_authority: write the new pending authority
//! write_pending_authority(vault_data, PENDING_OFFSET, new_authority.address())?;
//!
//! // In accept_authority: verify caller is the pending authority, then promote
//! accept_authority(vault_data, AUTHORITY_OFFSET, PENDING_OFFSET, caller.address())?;
//! ```

use pinocchio::{error::ProgramError, Address};

/// Verify the pending_authority field matches the expected address.
///
/// Reads 32 bytes at `pending_offset` from the account data and compares
/// against `expected`. Returns error if they don't match or if the
/// pending authority is zeroed (no pending handoff).
///
/// ```rust,ignore
/// let data = vault.try_borrow()?;
/// check_pending_authority(&data, PENDING_OFFSET, caller.address())?;
/// ```
#[inline(always)]
pub fn check_pending_authority(
    data: &[u8],
    pending_offset: usize,
    expected: &Address,
) -> Result<(), ProgramError> {
    if pending_offset + 32 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let stored = &data[pending_offset..pending_offset + 32];

    // Reject if pending authority is zeroed (no handoff in progress).
    if stored == [0u8; 32] {
        return Err(ProgramError::InvalidAccountData);
    }

    if stored != expected.as_ref() {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

/// Write a new pending authority into account data.
///
/// Writes 32 bytes at `pending_offset`. The caller is responsible for
/// verifying the current authority signed the transaction before calling this.
///
/// ```rust,ignore
/// let data = vault.try_borrow_mut()?;
/// write_pending_authority(data, PENDING_OFFSET, new_authority.address())?;
/// ```
#[inline(always)]
pub fn write_pending_authority(
    data: &mut [u8],
    pending_offset: usize,
    new_authority: &Address,
) -> Result<(), ProgramError> {
    if pending_offset + 32 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[pending_offset..pending_offset + 32].copy_from_slice(new_authority.as_ref());
    Ok(())
}

/// Accept an authority handoff: promote pending to active, clear pending.
///
/// 1. Reads `pending_authority` at `pending_offset`
/// 2. Verifies it matches `caller`
/// 3. Copies pending into `authority_offset`
/// 4. Zeroes out `pending_offset`
///
/// After this call, `caller` is the new authority and there is no
/// pending handoff.
///
/// ```rust,ignore
/// let data = vault.try_borrow_mut()?;
/// accept_authority(data, AUTHORITY_OFFSET, PENDING_OFFSET, caller.address())?;
/// ```
#[inline(always)]
pub fn accept_authority(
    data: &mut [u8],
    authority_offset: usize,
    pending_offset: usize,
    caller: &Address,
) -> Result<(), ProgramError> {
    // Bounds check both fields.
    if authority_offset + 32 > data.len() || pending_offset + 32 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Read and verify pending matches caller.
    let mut pending = [0u8; 32];
    pending.copy_from_slice(&data[pending_offset..pending_offset + 32]);

    if pending == [0u8; 32] {
        return Err(ProgramError::InvalidAccountData);
    }
    if pending != *caller.as_ref() {
        return Err(ProgramError::InvalidArgument);
    }

    // Promote: copy pending into authority slot.
    data[authority_offset..authority_offset + 32].copy_from_slice(&pending);

    // Clear pending.
    data[pending_offset..pending_offset + 32].copy_from_slice(&[0u8; 32]);

    Ok(())
}
