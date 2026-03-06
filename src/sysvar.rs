//! Zero-copy sysvar readers.
//!
//! Read Clock and Rent fields directly from sysvar account data without
//! deserialization. Each reader validates the sysvar address first, then
//! reads the fixed-layout fields using cursor-style offset reads.
//!
//! ## Clock layout (40 bytes)
//!
//! ```text
//!  0..8    slot                    (u64 LE)
//!  8..16   epoch_start_timestamp   (i64 LE)
//! 16..24   epoch                   (u64 LE)
//! 24..32   leader_schedule_epoch   (u64 LE)
//! 32..40   unix_timestamp          (i64 LE)
//! ```
//!
//! ## Rent layout (17 bytes)
//!
//! ```text
//!  0..8    lamports_per_byte_year  (u64 LE)
//!  8..16   exemption_threshold     (f64 LE, always 2.0 on mainnet)
//! 16       burn_percent            (u8)
//! ```

use pinocchio::{error::ProgramError, AccountView, ProgramResult};

#[cfg(feature = "programs")]
use crate::programs;

// ── Clock Sysvar ─────────────────────────────────────────────────────────────

/// Minimum size of the Clock sysvar data.
const CLOCK_LEN: usize = 40;

/// Verify the account is the Clock sysvar.
///
/// ```rust,ignore
/// check_clock_sysvar(clock_account)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_clock_sysvar(account: &AccountView) -> ProgramResult {
    if *account.address() != programs::SYSVAR_CLOCK {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Read both slot and unix_timestamp from the Clock sysvar account.
///
/// Validates the sysvar address, then returns `(slot, unix_timestamp)`.
/// This is the most common Clock usage. Almost every DeFi instruction
/// needs both the slot (for oracle staleness) and timestamp (for deadlines).
///
/// ```rust,ignore
/// let (slot, timestamp) = read_clock(clock_account)?;
/// require!(timestamp <= deadline, MyError::Expired);
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn read_clock(account: &AccountView) -> Result<(u64, i64), ProgramError> {
    check_clock_sysvar(account)?;
    let data = account.try_borrow()?;
    if data.len() < CLOCK_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let slot = u64::from_le_bytes(
        data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    let timestamp = i64::from_le_bytes(
        data[32..40]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok((slot, timestamp))
}

/// Read just the slot from the Clock sysvar.
///
/// ```rust,ignore
/// let slot = read_clock_slot(clock_account)?;
/// require!(slot - last_oracle_slot <= MAX_STALENESS, MyError::StaleOracle);
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn read_clock_slot(account: &AccountView) -> Result<u64, ProgramError> {
    check_clock_sysvar(account)?;
    let data = account.try_borrow()?;
    if data.len() < CLOCK_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let slot = u64::from_le_bytes(
        data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok(slot)
}

/// Read just the unix_timestamp from the Clock sysvar.
///
/// ```rust,ignore
/// let now = read_clock_timestamp(clock_account)?;
/// check_not_expired(now, listing.expiry)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn read_clock_timestamp(account: &AccountView) -> Result<i64, ProgramError> {
    check_clock_sysvar(account)?;
    let data = account.try_borrow()?;
    if data.len() < CLOCK_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let timestamp = i64::from_le_bytes(
        data[32..40]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok(timestamp)
}

/// Read the epoch from the Clock sysvar.
///
/// ```rust,ignore
/// let epoch = read_clock_epoch(clock_account)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn read_clock_epoch(account: &AccountView) -> Result<u64, ProgramError> {
    check_clock_sysvar(account)?;
    let data = account.try_borrow()?;
    if data.len() < CLOCK_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let epoch = u64::from_le_bytes(
        data[16..24]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok(epoch)
}

// ── Rent Sysvar ──────────────────────────────────────────────────────────────

/// Minimum size of the Rent sysvar data.
const RENT_LEN: usize = 17;

/// Verify the account is the Rent sysvar.
///
/// ```rust,ignore
/// check_rent_sysvar(rent_account)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_rent_sysvar(account: &AccountView) -> ProgramResult {
    if *account.address() != programs::SYSVAR_RENT {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Read the lamports_per_byte_year from the Rent sysvar.
///
/// For dynamic rent computation when the hardcoded rate in `rent_exempt_min`
/// isn't sufficient (e.g., if the rate ever changes).
///
/// ```rust,ignore
/// let rate = read_rent_lamports_per_byte_year(rent_account)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn read_rent_lamports_per_byte_year(account: &AccountView) -> Result<u64, ProgramError> {
    check_rent_sysvar(account)?;
    let data = account.try_borrow()?;
    if data.len() < RENT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let rate = u64::from_le_bytes(
        data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    Ok(rate)
}
