//! Token vesting schedule calculations.
//!
//! Common vesting primitives for team tokens, investor unlocks, and grant
//! programs. Linear vesting with cliff, stepped/periodic unlocks, and
//! safe claimable amount computation.
//!
//! All pure arithmetic, zero alloc, `#[inline(always)]`.

use hopper_runtime::ProgramError;

/// Compute the vested amount under a linear schedule with cliff.
///
/// Returns 0 before the cliff, `total` after `end`, and a proportional
/// amount in between. Uses u128 intermediate to avoid overflow.
///
/// ```rust,ignore
/// let vested = vested_amount(1_000_000, start, cliff, end, now);
/// ```
#[inline(always)]
pub fn vested_amount(total: u64, start: i64, cliff: i64, end: i64, now: i64) -> u64 {
    if now < cliff {
        return 0;
    }
    if now >= end {
        return total;
    }
    // Linear interpolation: total * (now - start) / (end - start)
    let elapsed = (now - start) as u128;
    let duration = (end - start) as u128;
    if duration == 0 {
        return total;
    }
    let vested = (total as u128) * elapsed / duration;
    if vested > total as u128 {
        total
    } else {
        vested as u64
    }
}

/// Check that the cliff timestamp has been reached.
///
/// Returns `InvalidAccountData` if `now < cliff_time`.
///
/// ```rust,ignore
/// check_cliff_reached(grant.cliff_time, current_time)?;
/// ```
#[inline(always)]
pub fn check_cliff_reached(cliff_time: i64, now: i64) -> Result<(), ProgramError> {
    if now < cliff_time {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Compute the unlocked amount under a stepped/periodic schedule.
///
/// Total is divided into `num_steps` equal portions. Returns
/// `total * min(steps_elapsed, num_steps) / num_steps`.
///
/// ```rust,ignore
/// let unlocked = unlocked_at_step(1_000_000, 12, months_elapsed);
/// ```
#[inline(always)]
pub fn unlocked_at_step(total: u64, num_steps: u32, steps_elapsed: u32) -> u64 {
    if num_steps == 0 {
        return total;
    }
    if steps_elapsed >= num_steps {
        return total;
    }
    let unlocked = (total as u128) * (steps_elapsed as u128) / (num_steps as u128);
    unlocked as u64
}

/// Compute the claimable amount (vested minus already claimed).
///
/// Safe subtraction: returns 0 if `already_claimed >= vested`.
///
/// ```rust,ignore
/// let claim = claimable(vested, user.claimed);
/// ```
#[inline(always)]
pub fn claimable(vested: u64, already_claimed: u64) -> u64 {
    vested.saturating_sub(already_claimed)
}

/// Compute the number of elapsed vesting steps given timestamps.
///
/// `step_duration` is the duration of each step in seconds.
/// Returns the number of completed steps since `start`.
///
/// ```rust,ignore
/// let steps = elapsed_steps(grant.start, now, 30 * 86400); // monthly steps
/// ```
#[inline(always)]
pub fn elapsed_steps(start: i64, now: i64, step_duration: i64) -> u32 {
    if now <= start || step_duration <= 0 {
        return 0;
    }
    let elapsed = (now - start) as u64;
    let steps = elapsed / step_duration as u64;
    if steps > u32::MAX as u64 {
        u32::MAX
    } else {
        steps as u32
    }
}
