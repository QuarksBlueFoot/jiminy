//! Time and deadline constraint checks for DeFi programs.
//!
//! Nearly every DeFi instruction involves time: order expiry, listing
//! deadlines, vesting schedules, cooldown periods, oracle staleness.
//! These checks compose with the sysvar clock reader to provide
//! one-line time validation.

use pinocchio::{error::ProgramError, ProgramResult};

/// Verify the current time has NOT passed a deadline.
///
/// Returns `Ok(())` if `current_timestamp <= deadline`.
/// Use this for orders, listings, bids, and any time-limited operation.
///
/// ```rust,ignore
/// let (_, now) = read_clock(clock_account)?;
/// check_not_expired(now, order.expiry_timestamp)?;
/// ```
#[inline(always)]
pub fn check_not_expired(current_timestamp: i64, deadline: i64) -> ProgramResult {
    if current_timestamp > deadline {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify the current time HAS passed a deadline.
///
/// Returns `Ok(())` if `current_timestamp > deadline`.
/// Use this for claim-after-vesting, settlement after expiry, or
/// cancellation of expired orders.
///
/// ```rust,ignore
/// let (_, now) = read_clock(clock_account)?;
/// check_expired(now, vesting.unlock_timestamp)?;
/// ```
#[inline(always)]
pub fn check_expired(current_timestamp: i64, deadline: i64) -> ProgramResult {
    if current_timestamp <= deadline {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify the current time is within an inclusive window `[start, end]`.
///
/// Useful for auction periods, sale windows, or any operation that's only
/// valid during a specific time range.
///
/// ```rust,ignore
/// let (_, now) = read_clock(clock_account)?;
/// check_within_window(now, auction.start_time, auction.end_time)?;
/// ```
#[inline(always)]
pub fn check_within_window(current_timestamp: i64, start: i64, end: i64) -> ProgramResult {
    if current_timestamp < start || current_timestamp > end {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify enough time has passed since the last action (cooldown/rate-limit).
///
/// Returns `Ok(())` if `current_timestamp >= last_action + cooldown_seconds`.
/// Use this for rate-limiting operations like price oracle updates,
/// admin parameter changes, or withdrawal cooldowns.
///
/// ```rust,ignore
/// let (_, now) = read_clock(clock_account)?;
/// check_cooldown(pool.last_rebalance, 3600, now)?; // 1-hour cooldown
/// ```
#[inline(always)]
pub fn check_cooldown(
    last_action_timestamp: i64,
    cooldown_seconds: i64,
    current_timestamp: i64,
) -> ProgramResult {
    let earliest_allowed = last_action_timestamp
        .checked_add(cooldown_seconds)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    if current_timestamp < earliest_allowed {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Combined: read clock timestamp and verify not expired.
///
/// Reads the Clock sysvar account and checks that the current unix
/// timestamp has not passed the deadline.
///
/// ```rust,ignore
/// check_deadline(clock_account, order.expiry)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_deadline(
    clock_account: &pinocchio::AccountView,
    deadline: i64,
) -> ProgramResult {
    let timestamp = crate::sysvar::read_clock_timestamp(clock_account)?;
    check_not_expired(timestamp, deadline)
}

/// Combined: read clock timestamp and verify expired.
///
/// Reads the Clock sysvar account and checks that the current unix
/// timestamp has passed the deadline. For claim-after-expiry patterns.
///
/// ```rust,ignore
/// check_after(clock_account, vesting.unlock_time)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_after(
    clock_account: &pinocchio::AccountView,
    deadline: i64,
) -> ProgramResult {
    let timestamp = crate::sysvar::read_clock_timestamp(clock_account)?;
    check_expired(timestamp, deadline)
}

/// Verify an on-chain data feed (oracle, price feed, etc.) is not stale.
///
/// Compares the slot when the data was last updated against the current slot,
/// rejecting if the gap exceeds `max_age_slots`. Every program integrating
/// Pyth, Switchboard, or any on-chain oracle should call this before using
/// the price.
///
/// Typical staleness thresholds:
/// - Aggressive DeFi (liquidations): 5-25 slots (~2-10 seconds)
/// - Standard DeFi (swaps): 50-150 slots (~20-60 seconds)
/// - Relaxed (display/analytics): 300+ slots
///
/// ```rust,ignore
/// let (current_slot, _) = read_clock(clock)?;
/// let oracle_data = oracle_account.try_borrow()?;
/// let last_update_slot = u64::from_le_bytes(oracle_data[8..16].try_into().unwrap());
/// check_slot_staleness(last_update_slot, current_slot, 50)?; // max 50 slots old
/// ```
#[inline(always)]
pub fn check_slot_staleness(
    last_update_slot: u64,
    current_slot: u64,
    max_age_slots: u64,
) -> ProgramResult {
    let age = current_slot.saturating_sub(last_update_slot);
    if age > max_age_slots {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
