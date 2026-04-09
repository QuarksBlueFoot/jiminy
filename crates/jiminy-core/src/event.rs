//! Zero-alloc event emission via `sol_log_data`.
//!
//! Solana programs can emit structured events by writing raw byte slices
//! to the transaction log. Indexers (Helius, Triton, etc.) pick these up.
//! Anchor uses borsh-serialized events behind proc macros. We skip all that.
//!
//! `emit_slices` writes one or more `&[u8]` segments in a single syscall.
//! Stack only, no heap, no alloc, no serialization framework.
//!
//! The `emit!` macro packs up to 8 slices into a stack array and calls
//! `sol_log_data` directly. Use it for discriminators, pubkeys, u64s,
//! whatever your indexer expects.
//!
//! ```rust,ignore
//! // Emit a "Deposit" event: 1-byte discriminator + pubkey + amount
//! let disc = [0x01u8];
//! let amount_bytes = amount.to_le_bytes();
//! emit!(&disc, user.address().as_ref(), &amount_bytes);
//! ```
//!
//! Raw bytes, zero overhead, no serialization framework.

/// Emit one or more byte slices as a single `sol_log_data` entry.
///
/// Each slice becomes a separate data segment in the log. Indexers can
/// parse these however they want. Zero alloc, stack only.
///
/// ```rust,ignore
/// emit_slices(&[&[0x01], authority.address().as_ref(), &amount.to_le_bytes()]);
/// ```
#[inline(always)]
pub fn emit_slices(segments: &[&[u8]]) {
    #[cfg(target_os = "solana")]
    {
        // sol_log_data expects a pointer to an array of (ptr, len) pairs.
        // On BPF, a Rust slice &[u8] is (ptr: *const u8, len: usize) which
        // is exactly 16 bytes. We pass the segments slice directly.
        // SAFETY: segments is a valid &[&[u8]] slice whose repr matches the
        // (ptr, len) pair layout expected by sol_log_data.  The pointer and
        // length are derived from a live borrow so they remain valid for the
        // duration of the syscall.
        unsafe {
            hopper_runtime::syscalls::sol_log_data(
                segments.as_ptr() as *const u8,
                segments.len() as u64,
            );
        }
    }
    #[cfg(not(target_os = "solana"))]
    {
        let _ = segments;
    }
}

/// Emit byte slices as a structured event log entry.
///
/// Packs up to 8 slices on the stack and calls `sol_log_data` in a single
/// syscall. For indexer-friendly event emission without borsh, proc macros,
/// or an allocator.
///
/// ```rust,ignore
/// // Deposit event: discriminator + user pubkey + amount
/// let disc = [0x01u8];
/// let amt = deposit_amount.to_le_bytes();
/// emit!(&disc, user.address().as_ref(), &amt);
///
/// // Withdraw event: discriminator + vault + user + amount
/// let disc = [0x02u8];
/// let amt = withdraw_amount.to_le_bytes();
/// emit!(&disc, vault.address().as_ref(), user.address().as_ref(), &amt);
/// ```
#[macro_export]
macro_rules! emit {
    ($($segment:expr),+ $(,)?) => {{
        $crate::event::emit_slices(&[$($segment),+]);
    }};
}
