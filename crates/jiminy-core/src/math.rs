use pinocchio::error::ProgramError;

/// Checked u64 addition: returns `ArithmeticOverflow` on overflow.
#[inline(always)]
pub fn checked_add(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_add(b).ok_or(ProgramError::ArithmeticOverflow)
}

/// Checked u64 subtraction: returns `ArithmeticOverflow` on underflow.
#[inline(always)]
pub fn checked_sub(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_sub(b).ok_or(ProgramError::ArithmeticOverflow)
}

/// Checked u64 multiplication: returns `ArithmeticOverflow` on overflow.
#[inline(always)]
pub fn checked_mul(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_mul(b).ok_or(ProgramError::ArithmeticOverflow)
}

/// Checked u64 division: returns `ArithmeticOverflow` on divide-by-zero.
///
/// Every AMM price calculation involves division. This is the missing
/// companion to `checked_add`/`checked_sub`/`checked_mul`.
///
/// ```rust,ignore
/// let price = checked_div(reserve_b, reserve_a)?;
/// ```
#[inline(always)]
pub fn checked_div(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_div(b).ok_or(ProgramError::ArithmeticOverflow)
}

/// Checked ceiling division: `ceil(a / b)`. Returns `ArithmeticOverflow` on zero.
///
/// Rounds up instead of truncating. Use this for fee calculations and
/// minimum-output computations where truncation would favor the user
/// at the protocol's expense.
///
/// ```rust,ignore
/// let fee = checked_div_ceil(amount * fee_rate, 10_000)?;
/// ```
#[inline(always)]
pub fn checked_div_ceil(a: u64, b: u64) -> Result<u64, ProgramError> {
    if b == 0 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    // ceil(a / b) = (a + b - 1) / b, guarding against overflow in a + b - 1
    Ok(a.checked_add(b - 1)
        .ok_or(ProgramError::ArithmeticOverflow)?
        / b)
}

/// Compute `(a * b) / c` with u128 intermediate to prevent overflow.
///
/// **The single most important DeFi math primitive.** Without u128
/// intermediate, `a * b` overflows for any token amounts > ~4.2B.
/// Returns floor division.
///
/// ```rust,ignore
/// // Constant-product swap: dy = (y * dx) / (x + dx)
/// let output = checked_mul_div(reserve_y, input, reserve_x + input)?;
/// ```
#[inline(always)]
pub fn checked_mul_div(a: u64, b: u64, c: u64) -> Result<u64, ProgramError> {
    if c == 0 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    let result = (a as u128)
        .checked_mul(b as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?
        / (c as u128);
    to_u64(result)
}

/// Compute `ceil((a * b) / c)` with u128 intermediate.
///
/// Same as `checked_mul_div` but rounds up. Use this for fee calculations
/// to ensure the protocol never gets rounded down to zero fee.
///
/// ```rust,ignore
/// let fee = checked_mul_div_ceil(amount, fee_bps, 10_000)?;
/// ```
#[inline(always)]
pub fn checked_mul_div_ceil(a: u64, b: u64, c: u64) -> Result<u64, ProgramError> {
    if c == 0 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    let numerator = (a as u128)
        .checked_mul(b as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    let c128 = c as u128;
    // ceil(n / d) = (n + d - 1) / d
    let result = numerator
        .checked_add(c128 - 1)
        .ok_or(ProgramError::ArithmeticOverflow)?
        / c128;
    to_u64(result)
}

/// Compute basis-point fee: `amount * bps / 10_000` (floor).
///
/// Uses u128 intermediate to prevent overflow. Nearly every DeFi program
/// computes fees in basis points. This one-liner eliminates a whole class
/// of bugs.
///
/// ```rust,ignore
/// let fee = bps_of(trade_amount, 30)?; // 0.3% fee
/// ```
#[inline(always)]
pub fn bps_of(amount: u64, basis_points: u16) -> Result<u64, ProgramError> {
    checked_mul_div(amount, basis_points as u64, 10_000)
}

/// Compute basis-point fee with ceiling: `ceil(amount * bps / 10_000)`.
///
/// Fees must never round to zero. Use this to ensure the protocol always
/// collects at least 1 token unit of fee when a fee is configured.
///
/// ```rust,ignore
/// let fee = bps_of_ceil(trade_amount, 30)?; // 0.3% fee, always >= 1
/// ```
#[inline(always)]
pub fn bps_of_ceil(amount: u64, basis_points: u16) -> Result<u64, ProgramError> {
    checked_mul_div_ceil(amount, basis_points as u64, 10_000)
}

/// Checked exponentiation via repeated squaring.
///
/// Computes `base^exp` with overflow checking at each step. Useful for
/// compound interest calculations and exponential decay.
///
/// ```rust,ignore
/// let compound = checked_pow(rate_per_period, num_periods)?;
/// ```
#[inline(always)]
pub fn checked_pow(base: u64, exp: u32) -> Result<u64, ProgramError> {
    if exp == 0 {
        return Ok(1);
    }
    let mut result: u64 = 1;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result.checked_mul(b).ok_or(ProgramError::ArithmeticOverflow)?;
        }
        e >>= 1;
        if e > 0 {
            b = b.checked_mul(b).ok_or(ProgramError::ArithmeticOverflow)?;
        }
    }
    Ok(result)
}

/// Safe narrowing cast from u128 to u64.
///
/// Returns `ArithmeticOverflow` if the value exceeds `u64::MAX`.
/// Use this after u128 intermediate computations.
///
/// ```rust,ignore
/// let result_u128: u128 = (a as u128) * (b as u128) / (c as u128);
/// let result = to_u64(result_u128)?;
/// ```
#[inline(always)]
pub fn to_u64(val: u128) -> Result<u64, ProgramError> {
    if val > u64::MAX as u128 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    Ok(val as u64)
}

/// Scale a token amount between different decimal precisions.
///
/// Converts `amount` denominated in `from_decimals` to the equivalent
/// value in `to_decimals`. Uses u128 intermediate to prevent overflow
/// when scaling up.
///
/// The most common cross-mint arithmetic operation in DeFi:
/// comparing USDC (6 decimals) amounts to SOL (9 decimals), or pricing
/// a token with 8 decimals against one with 18.
///
/// ```rust,ignore
/// // Convert 1_000_000 USDC (6 decimals) to SOL-precision (9 decimals)
/// let scaled = scale_amount(1_000_000, 6, 9)?;
/// assert_eq!(scaled, 1_000_000_000);
///
/// // Convert 1_000_000_000 (9 decimals) down to 6 decimals
/// let scaled = scale_amount(1_000_000_000, 9, 6)?;
/// assert_eq!(scaled, 1_000_000);
/// ```
#[inline(always)]
pub fn scale_amount(amount: u64, from_decimals: u8, to_decimals: u8) -> Result<u64, ProgramError> {
    if from_decimals == to_decimals {
        return Ok(amount);
    }
    if to_decimals > from_decimals {
        // Scale up: amount * 10^(to - from)
        let factor = 10u128.checked_pow((to_decimals - from_decimals) as u32)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let result = (amount as u128)
            .checked_mul(factor)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        to_u64(result)
    } else {
        // Scale down: amount / 10^(from - to) (truncates)
        let factor = 10u64.checked_pow((from_decimals - to_decimals) as u32)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        checked_div(amount, factor)
    }
}

/// Scale a token amount between decimal precisions, rounding up.
///
/// Same as [`scale_amount`] but uses ceiling division when scaling down.
/// Use this for protocol-side calculations where truncating would
/// short-change the protocol (e.g., minimum collateral requirements).
///
/// ```rust,ignore
/// // 999 units at 9 decimals → 6 decimals, rounds up to 1
/// let scaled = scale_amount_ceil(999, 9, 6)?;
/// assert_eq!(scaled, 1);
/// ```
#[inline(always)]
pub fn scale_amount_ceil(amount: u64, from_decimals: u8, to_decimals: u8) -> Result<u64, ProgramError> {
    if from_decimals == to_decimals {
        return Ok(amount);
    }
    if to_decimals > from_decimals {
        // Scale up - same as floor (no rounding needed when multiplying)
        let factor = 10u128.checked_pow((to_decimals - from_decimals) as u32)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let result = (amount as u128)
            .checked_mul(factor)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        to_u64(result)
    } else {
        // Scale down with ceiling
        let factor = 10u64.checked_pow((from_decimals - to_decimals) as u32)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        checked_div_ceil(amount, factor)
    }
}
