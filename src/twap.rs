//! TWAP (time-weighted average price) accumulator math.
//!
//! Maintains a cumulative price sum that increases by `price * elapsed`
//! each update. The TWAP between any two observations is just the
//! difference in cumulative values divided by elapsed time.
//!
//! Uses u128 throughout to avoid overflow when accumulating over long
//! periods.

use pinocchio::error::ProgramError;

/// Advance the cumulative price accumulator.
///
/// `cumulative` - current running sum (scaled by time).
/// `price`      - current spot price as a u64.
/// `last_ts`    - unix timestamp of the previous update.
/// `now_ts`     - current unix timestamp.
///
/// Returns the new cumulative value. If `now_ts <= last_ts` the value
/// is returned unchanged (no time elapsed).
///
/// ```rust,ignore
/// pool.cumulative = update_twap_cumulative(
///     pool.cumulative, spot_price, pool.last_ts, now,
/// )?;
/// pool.last_ts = now;
/// ```
#[inline(always)]
pub fn update_twap_cumulative(
    cumulative: u128,
    price: u64,
    last_ts: i64,
    now_ts: i64,
) -> Result<u128, ProgramError> {
    if now_ts <= last_ts {
        return Ok(cumulative);
    }
    let elapsed = (now_ts - last_ts) as u128;
    let increment = (price as u128)
        .checked_mul(elapsed)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    cumulative
        .checked_add(increment)
        .ok_or(ProgramError::ArithmeticOverflow)
}

/// Compute a TWAP from two cumulative observations.
///
/// `twap = (end_cumulative - start_cumulative) / (end_ts - start_ts)`
///
/// Returns the time-weighted average as a u64.
///
/// ```rust,ignore
/// let twap = compute_twap(
///     old_obs.cumulative, new_obs.cumulative,
///     old_obs.timestamp,  new_obs.timestamp,
/// )?;
/// ```
#[inline(always)]
pub fn compute_twap(
    cumulative_start: u128,
    cumulative_end: u128,
    ts_start: i64,
    ts_end: i64,
) -> Result<u64, ProgramError> {
    if ts_end <= ts_start {
        return Err(ProgramError::InvalidArgument);
    }
    let elapsed = (ts_end - ts_start) as u128;
    let diff = cumulative_end
        .checked_sub(cumulative_start)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    let twap = diff / elapsed;
    if twap > u64::MAX as u128 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    Ok(twap as u64)
}

/// Fail if `spot_price` deviates from `twap_price` by more than
/// `max_deviation_bps` basis points.
///
/// Anti-manipulation guard: a large spot/TWAP spread suggests the
/// current price is being moved artificially.
///
/// ```rust,ignore
/// check_twap_deviation(spot, twap, 500)?; // max 5%
/// ```
#[inline(always)]
pub fn check_twap_deviation(
    spot_price: u64,
    twap_price: u64,
    max_deviation_bps: u64,
) -> Result<(), ProgramError> {
    if twap_price == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    let diff = if spot_price > twap_price {
        spot_price - twap_price
    } else {
        twap_price - spot_price
    };
    let deviation_bps = (diff as u128) * 10_000 / (twap_price as u128);
    if deviation_bps > max_deviation_bps as u128 {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
