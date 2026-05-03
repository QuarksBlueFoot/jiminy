//! Zero-copy sysvar readers.
//!
//! Two access paths:
//!
//! 1. **Syscall** (`clock_timestamp()`, `clock_slot()`, etc.): reads via
//!    `sol_get_clock_sysvar` / `sol_get_rent_sysvar`. No account needed,
//!    saves one account slot in your instruction. Available on-chain only.
//!
//! 2. **Account-based** (`read_clock()`, `read_clock_slot()`, etc.): reads
//!    from a passed-in Clock or Rent sysvar account. Works in tests and
//!    anywhere you already have the account.
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

use hopper_runtime::{ProgramError, AccountView, ProgramResult};

#[cfg(feature = "programs")]
use crate::programs;

// ── Syscall-based Clock access (no account needed) ───────────────────────────

#[cfg(target_os = "solana")]
extern "C" {
    fn sol_get_clock_sysvar(addr: *mut u8) -> u64;
    fn sol_get_rent_sysvar(addr: *mut u8) -> u64;
}

/// Read the unix timestamp from the Clock sysvar via syscall.
///
/// No account needed. Saves one account slot per instruction compared
/// to passing the Clock sysvar account.
///
/// ```rust,ignore
/// let now = clock_timestamp()?;
/// check_not_expired(now, order.expiry)?;
/// ```
#[inline(always)]
pub fn clock_timestamp() -> Result<i64, ProgramError> {
    let buf = get_clock_buf()?;
    Ok(i64::from_le_bytes([
        buf[32], buf[33], buf[34], buf[35], buf[36], buf[37], buf[38], buf[39],
    ]))
}

/// Read the current slot from the Clock sysvar via syscall.
///
/// ```rust,ignore
/// let slot = clock_slot()?;
/// check_slot_staleness(oracle_slot, slot, 50)?;
/// ```
#[inline(always)]
pub fn clock_slot() -> Result<u64, ProgramError> {
    let buf = get_clock_buf()?;
    Ok(u64::from_le_bytes([
        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
    ]))
}

/// Read both slot and unix_timestamp from the Clock sysvar via syscall.
///
/// ```rust,ignore
/// let (slot, ts) = clock_slot_and_timestamp()?;
/// ```
#[inline(always)]
pub fn clock_slot_and_timestamp() -> Result<(u64, i64), ProgramError> {
    let buf = get_clock_buf()?;
    let slot = u64::from_le_bytes([
        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
    ]);
    let ts = i64::from_le_bytes([
        buf[32], buf[33], buf[34], buf[35], buf[36], buf[37], buf[38], buf[39],
    ]);
    Ok((slot, ts))
}

/// Read the epoch from the Clock sysvar via syscall.
///
/// ```rust,ignore
/// let epoch = clock_epoch()?;
/// ```
#[inline(always)]
pub fn clock_epoch() -> Result<u64, ProgramError> {
    let buf = get_clock_buf()?;
    Ok(u64::from_le_bytes([
        buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
    ]))
}

/// Read the lamports_per_byte_year from the Rent sysvar via syscall.
///
/// ```rust,ignore
/// let rate = rent_lamports_per_byte_year()?;
/// ```
#[inline(always)]
pub fn rent_lamports_per_byte_year() -> Result<u64, ProgramError> {
    let buf = get_rent_buf()?;
    Ok(u64::from_le_bytes([
        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
    ]))
}

/// Read the full Clock sysvar into a stack buffer via syscall.
#[inline(always)]
fn get_clock_buf() -> Result<[u8; CLOCK_LEN], ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let mut buf = [0u8; CLOCK_LEN];
        // SAFETY: buf is CLOCK_LEN bytes; sol_get_clock_sysvar writes at most that many.
        let rc = unsafe { sol_get_clock_sysvar(buf.as_mut_ptr()) };
        if rc != 0 {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(buf)
    }
    #[cfg(not(target_os = "solana"))]
    {
        Err(ProgramError::InvalidArgument)
    }
}

/// Read the full Rent sysvar into a stack buffer via syscall.
#[inline(always)]
fn get_rent_buf() -> Result<[u8; RENT_LEN], ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let mut buf = [0u8; RENT_LEN];
        // SAFETY: buf is RENT_LEN bytes; sol_get_rent_sysvar writes at most that many.
        let rc = unsafe { sol_get_rent_sysvar(buf.as_mut_ptr()) };
        if rc != 0 {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(buf)
    }
    #[cfg(not(target_os = "solana"))]
    {
        Err(ProgramError::InvalidArgument)
    }
}

// ── Account-based Clock access ───────────────────────────────────────────────

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

// ── Account-based Rent access ─────────────────────────────────────────────────

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
