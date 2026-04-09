//! Zero-copy Pyth V2 price feed readers.
//!
//! Read oracle prices directly from Pyth price account data at fixed byte
//! offsets. No `pyth-sdk-solana` dependency, no borsh, no alloc.
//!
//! Pyth V2 price account layout (first 240 bytes are the fixed header):
//! ```text
//!   0..4    magic         (u32 LE = 0xa1b2c3d4)
//!   4..8    version       (u32 LE = 2)
//!   8..12   account type  (u32 LE = 3 for price)
//!  20..24   exponent      (i32 LE, e.g. -8)
//!  48..56   ema_price     (i64 LE)
//!  72..80   ema_conf      (i64 LE, treat as u64)
//!  96..104  timestamp     (i64 LE, unix seconds)
//! 200..208  prev_timestamp(i64 LE, last TRADING timestamp)
//! 208..216  agg.price     (i64 LE, current aggregate price)
//! 216..224  agg.conf      (u64 LE, confidence interval)
//! 224..228  agg.status    (u32 LE, 1 = TRADING)
//! 232..240  agg.pub_slot  (u64 LE)
//! ```
//!
//! Works for both Solana and Pythnet price accounts (header is identical).

use hopper_runtime::ProgramError;

// ── Constants ────────────────────────────────────────────────────────────────

/// Pyth V2 magic number (`0xa1b2c3d4`).
pub const PYTH_MAGIC: u32 = 0xa1b2c3d4;

/// Pyth V2 version.
pub const PYTH_VERSION: u32 = 2;

/// Pyth account type for price accounts.
pub const PYTH_PRICE_TYPE: u32 = 3;

/// Pyth price status: valid aggregate price available.
pub const STATUS_TRADING: u32 = 1;

/// Minimum account data length for the fixed header (excludes component prices).
pub const PYTH_HEADER_LEN: usize = 240;

// ── Byte-offset helpers (private) ────────────────────────────────────────────

const OFF_MAGIC: usize = 0;
const OFF_VERSION: usize = 4;
const OFF_ATYPE: usize = 8;
const OFF_EXPO: usize = 20;
const OFF_EMA_PRICE: usize = 48;
const OFF_EMA_CONF: usize = 72;
const OFF_TIMESTAMP: usize = 96;
const OFF_AGG_PRICE: usize = 208;
const OFF_AGG_CONF: usize = 216;
const OFF_AGG_STATUS: usize = 224;
const OFF_AGG_PUB_SLOT: usize = 232;

// ── Internal readers ─────────────────────────────────────────────────────────

#[inline(always)]
fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

#[inline(always)]
fn read_i32(data: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

#[inline(always)]
fn read_u64(data: &[u8], off: usize) -> u64 {
    u64::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
        data[off + 4],
        data[off + 5],
        data[off + 6],
        data[off + 7],
    ])
}

#[inline(always)]
fn read_i64(data: &[u8], off: usize) -> i64 {
    i64::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
        data[off + 4],
        data[off + 5],
        data[off + 6],
        data[off + 7],
    ])
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Pyth price data extracted from the aggregate fields.
///
/// `price` and `conf` are raw integer values. Multiply by `10^expo` to get
/// the human-readable price. For example, price=12345678, expo=-6 means $12.345678.
pub struct PythPrice {
    /// Aggregate price (i64). Negative for inverse feeds.
    pub price: i64,
    /// Confidence interval (u64). Always non-negative.
    pub conf: u64,
    /// Price exponent (i32). Usually negative, e.g. -8.
    pub expo: i32,
    /// Publish timestamp (unix seconds). Uses `timestamp` if TRADING,
    /// otherwise `prev_timestamp`.
    pub publish_time: i64,
}

/// EMA (exponentially-weighted moving average) price data.
pub struct PythEma {
    /// EMA price (i64).
    pub price: i64,
    /// EMA confidence (u64).
    pub conf: u64,
    /// Price exponent (i32), same as the aggregate.
    pub expo: i32,
}

/// Validate and read the current aggregate price from a Pyth V2 price account.
///
/// Returns `InvalidAccountData` if:
/// - Data is shorter than 240 bytes
/// - Magic, version, or account type don't match Pyth V2
/// - Aggregate status is not `TRADING`
///
/// ```rust,ignore
/// let data = pyth_account.try_borrow()?;
/// let p = read_pyth_price(&data)?;
/// // p.price * 10^(p.expo) = human-readable price
/// ```
#[inline(always)]
pub fn read_pyth_price(data: &[u8]) -> Result<PythPrice, ProgramError> {
    if data.len() < PYTH_HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    check_pyth_header(data)?;

    let status = read_u32(data, OFF_AGG_STATUS);
    if status != STATUS_TRADING {
        return Err(ProgramError::InvalidAccountData);
    }

    let publish_time = read_i64(data, OFF_TIMESTAMP);

    Ok(PythPrice {
        price: read_i64(data, OFF_AGG_PRICE),
        conf: read_u64(data, OFF_AGG_CONF),
        expo: read_i32(data, OFF_EXPO),
        publish_time,
    })
}

/// Read the EMA (exponentially-weighted moving average) price from a Pyth V2
/// price account. Does not require TRADING status.
///
/// ```rust,ignore
/// let ema = read_pyth_ema(&data)?;
/// ```
#[inline(always)]
pub fn read_pyth_ema(data: &[u8]) -> Result<PythEma, ProgramError> {
    if data.len() < PYTH_HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    check_pyth_header(data)?;

    Ok(PythEma {
        price: read_i64(data, OFF_EMA_PRICE),
        conf: read_u64(data, OFF_EMA_CONF),
        expo: read_i32(data, OFF_EXPO),
    })
}

/// Read the aggregate publish slot from a Pyth V2 price account.
#[inline(always)]
pub fn pyth_agg_pub_slot(data: &[u8]) -> Result<u64, ProgramError> {
    if data.len() < PYTH_HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    check_pyth_header(data)?;
    Ok(read_u64(data, OFF_AGG_PUB_SLOT))
}

/// Check that a Pyth price is fresh (not stale).
///
/// Returns `InvalidAccountData` if the publish timestamp is older
/// than `max_age_seconds` from `current_time`.
///
/// ```rust,ignore
/// let (slot, ts) = read_clock(clock_account)?;
/// let p = read_pyth_price(&data)?;
/// check_pyth_price_fresh(p.publish_time, ts, 30)?; // max 30s stale
/// ```
#[inline(always)]
pub fn check_pyth_price_fresh(
    publish_time: i64,
    current_time: i64,
    max_age_seconds: i64,
) -> Result<(), ProgramError> {
    if current_time.wrapping_sub(publish_time) > max_age_seconds {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Check that a Pyth price's confidence interval is within acceptable bounds.
///
/// Returns `InvalidAccountData` if `conf * 100 / price.abs() > max_conf_pct`.
/// This prevents using a price feed when the oracle is too uncertain.
///
/// ```rust,ignore
/// let p = read_pyth_price(&data)?;
/// check_pyth_confidence(p.price, p.conf, 5)?; // max 5% confidence/price ratio
/// ```
#[inline(always)]
pub fn check_pyth_confidence(
    price: i64,
    conf: u64,
    max_conf_pct: u64,
) -> Result<(), ProgramError> {
    let abs_price = (price as i128).unsigned_abs();
    if abs_price == 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    // conf * 100 / abs_price > max_conf_pct => reject
    let ratio = (conf as u128)
        .checked_mul(100)
        .ok_or(ProgramError::ArithmeticOverflow)?
        / abs_price;
    if ratio > max_conf_pct as u128 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── Internal ─────────────────────────────────────────────────────────────────

/// Validate magic, version, and account type.
#[inline(always)]
fn check_pyth_header(data: &[u8]) -> Result<(), ProgramError> {
    let magic = read_u32(data, OFF_MAGIC);
    let ver = read_u32(data, OFF_VERSION);
    let atype = read_u32(data, OFF_ATYPE);
    if magic != PYTH_MAGIC || ver != PYTH_VERSION || atype != PYTH_PRICE_TYPE {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
