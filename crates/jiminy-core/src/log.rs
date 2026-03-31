//! Lightweight logging helpers for debugging on-chain programs.
//!
//! Gated behind the `log` feature to keep production builds tiny.
//! Wraps the raw `sol_log_` syscall. Zero alloc, zero deps.
//!
//! These are diagnostic aids. They print a label + value so you can see
//! which check failed or what value was computed without manually
//! constructing log strings.
//!
//! ```rust,ignore
//! // In your Cargo.toml:
//! // jiminy = { version = "0.7", features = ["log"] }
//!
//! use jiminy::log::*;
//!
//! log_msg("processing deposit");
//! log_val("amount", amount);
//! log_addr("authority", authority.address());
//! ```

/// Write a string to the program log via the sol_log syscall.
#[inline(always)]
fn sol_log(msg: &str) {
    #[cfg(target_os = "solana")]
    // SAFETY: sol_log_ expects a valid pointer and length. msg is a valid &str.
    unsafe {
        pinocchio::syscalls::sol_log_(msg.as_ptr(), msg.len() as u64);
    }
    #[cfg(not(target_os = "solana"))]
    {
        let _ = msg;
    }
}

/// Log a static message string.
///
/// ```rust,ignore
/// log_msg("init vault");
/// ```
#[inline(always)]
pub fn log_msg(msg: &str) {
    sol_log(msg);
}

/// Log a label + u64 value.
///
/// Prints: `"label: <value>"`. Uses a raw byte buffer to avoid alloc.
///
/// ```rust,ignore
/// log_val("balance", 42_000_000);
/// // prints: "balance: 42000000"
/// ```
#[inline(always)]
pub fn log_val(label: &str, value: u64) {
    // Build "label: <digits>" in a fixed buffer.
    // Max u64 is 20 digits. Label + ": " + digits = label.len() + 22.
    let mut buf = [0u8; 128];
    let label_bytes = label.as_bytes();
    let label_len = label_bytes.len().min(100); // cap label to prevent overflow

    buf[..label_len].copy_from_slice(&label_bytes[..label_len]);
    buf[label_len] = b':';
    buf[label_len + 1] = b' ';
    let mut pos = label_len + 2;

    if value == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        // Write digits in reverse, then reverse them.
        let start = pos;
        let mut v = value;
        while v > 0 {
            buf[pos] = b'0' + (v % 10) as u8;
            v /= 10;
            pos += 1;
        }
        // Reverse the digits.
        let mut i = start;
        let mut j = pos - 1;
        while i < j {
            buf.swap(i, j);
            i += 1;
            j -= 1;
        }
    }

    // SAFETY: buf[..pos] contains only ASCII digits, ':', ' ', and label bytes
    // (validated as bytes from &str). All are valid UTF-8.
    let msg = unsafe { core::str::from_utf8_unchecked(&buf[..pos]) };
    sol_log(msg);
}

/// Log a label + i64 value (signed).
///
/// ```rust,ignore
/// log_signed("timestamp", timestamp);
/// ```
#[inline(always)]
pub fn log_signed(label: &str, value: i64) {
    if value < 0 {
        // For negative values, log as "-<abs_value>"
        let mut buf = [0u8; 128];
        let label_bytes = label.as_bytes();
        let label_len = label_bytes.len().min(100);

        buf[..label_len].copy_from_slice(&label_bytes[..label_len]);
        buf[label_len] = b':';
        buf[label_len + 1] = b' ';
        buf[label_len + 2] = b'-';

        let abs_val = (value as i128).unsigned_abs() as u64;
        let mut pos = label_len + 3;
        let start = pos;
        let mut v = abs_val;
        if v == 0 {
            buf[pos] = b'0';
            pos += 1;
        } else {
            while v > 0 {
                buf[pos] = b'0' + (v % 10) as u8;
                v /= 10;
                pos += 1;
            }
            let mut i = start;
            let mut j = pos - 1;
            while i < j {
                buf.swap(i, j);
                i += 1;
                j -= 1;
            }
        }
        // SAFETY: buf contains only ASCII digits, '-', ':', ' ', and label bytes.
        let msg = unsafe { core::str::from_utf8_unchecked(&buf[..pos]) };
        sol_log(msg);
    } else {
        log_val(label, value as u64);
    }
}

/// Log a label + address (base58 is too expensive; prints raw bytes as hex-ish).
///
/// Prints the first and last 4 bytes of the address for quick identification
/// without the CU cost of full base58 encoding.
///
/// ```rust,ignore
/// log_addr("vault", vault.address());
/// // prints: "vault: [1a2b3c4d..f1e2d3c4]"
/// ```
#[inline(always)]
pub fn log_addr(label: &str, addr: &pinocchio::Address) {
    let bytes = addr.as_array();
    let mut buf = [0u8; 128];
    let label_bytes = label.as_bytes();
    let label_len = label_bytes.len().min(90);

    buf[..label_len].copy_from_slice(&label_bytes[..label_len]);
    buf[label_len] = b':';
    buf[label_len + 1] = b' ';
    buf[label_len + 2] = b'[';
    let mut pos = label_len + 3;

    // First 4 bytes as hex.
    for i in 0..4 {
        let hi = bytes[i] >> 4;
        let lo = bytes[i] & 0x0f;
        buf[pos] = if hi < 10 { b'0' + hi } else { b'a' + hi - 10 };
        buf[pos + 1] = if lo < 10 { b'0' + lo } else { b'a' + lo - 10 };
        pos += 2;
    }

    buf[pos] = b'.';
    buf[pos + 1] = b'.';
    pos += 2;

    // Last 4 bytes as hex.
    for i in 28..32 {
        let hi = bytes[i] >> 4;
        let lo = bytes[i] & 0x0f;
        buf[pos] = if hi < 10 { b'0' + hi } else { b'a' + hi - 10 };
        buf[pos + 1] = if lo < 10 { b'0' + lo } else { b'a' + lo - 10 };
        pos += 2;
    }

    buf[pos] = b']';
    pos += 1;

    // SAFETY: buf contains only ASCII hex digits, ':', ' ', '[', ']', '.', and label bytes.
    let msg = unsafe { core::str::from_utf8_unchecked(&buf[..pos]) };
    sol_log(msg);
}

/// Log a label + boolean value.
///
/// ```rust,ignore
/// log_bool("is_frozen", is_frozen);
/// ```
#[inline(always)]
pub fn log_bool(label: &str, value: bool) {
    if value {
        let mut buf = [0u8; 128];
        let label_bytes = label.as_bytes();
        let label_len = label_bytes.len().min(120);
        buf[..label_len].copy_from_slice(&label_bytes[..label_len]);
        buf[label_len] = b':';
        buf[label_len + 1] = b' ';
        buf[label_len + 2] = b'Y';
        // SAFETY: buf contains only label bytes (valid UTF-8) plus ASCII ':', ' ', 'Y'.
        let msg = unsafe { core::str::from_utf8_unchecked(&buf[..label_len + 3]) };
        sol_log(msg);
    } else {
        let mut buf = [0u8; 128];
        let label_bytes = label.as_bytes();
        let label_len = label_bytes.len().min(120);
        buf[..label_len].copy_from_slice(&label_bytes[..label_len]);
        buf[label_len] = b':';
        buf[label_len + 1] = b' ';
        buf[label_len + 2] = b'N';
        // SAFETY: buf contains only label bytes (valid UTF-8) plus ASCII ':', ' ', 'N'.
        let msg = unsafe { core::str::from_utf8_unchecked(&buf[..label_len + 3]) };
        sol_log(msg);
    }
}
