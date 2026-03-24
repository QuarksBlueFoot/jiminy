//! Optional `solana-zero-copy` integration.
//!
//! When the `solana-zero-copy` feature is enabled, this module provides:
//!
//! - Blanket [`Pod`](crate::account::Pod) implementations for `solana-zero-copy` unaligned
//!   primitives (`U16`, `U32`, `U64`, `I16`, `I64`, `U128`, `Bool`)
//! - [`FixedLayout`](crate::account::FixedLayout) implementations for the same types
//! - Re-exports for convenient use in `zero_copy_layout!` structs
//! - Bidirectional `From` conversions between Jiminy `Le*` types and `solana-zero-copy` types
//!
//! This allows overlay structs to use alignment-1 field types natively,
//! which is the recommended approach for maximum portability between SBF
//! and native targets.
//!
//! ## Type mapping
//!
//! | Jiminy | solana-zero-copy | Inner |
//! |--------|-----------------|-------|
//! | `LeU16` | `U16` | `[u8; 2]` |
//! | `LeU32` | `U32` | `[u8; 4]` |
//! | `LeU64` | `U64` | `[u8; 8]` |
//! | `LeU128` | `U128` | `[u8; 16]` (non-BPF) |
//! | `LeI16` | `I16` | `[u8; 2]` |
//! | `LeI64` | `I64` | `[u8; 8]` |
//! | `LeBool` | `Bool` | `u8` |
//!
//! Note: `solana-zero-copy` v1.0.0 does not define `I32`, `I128`, or `U8`
//! types. Jiminy's `LeI32`, `LeI128` have no direct counterpart.
//!
//! ## Example
//!
//! ```rust,ignore
//! use jiminy_core::compat::szc::{U64, U128};
//!
//! zero_copy_layout! {
//!     Pool {
//!         disc = 3,
//!         version = 1,
//!         liquidity: U128 = 16,
//!         sqrt_price: U128 = 16,
//!         fee_rate: U64 = 8,
//!         authority: Address = 32,
//!     }
//! }
//! ```
//!
//! ## Feature gate
//!
//! ```toml
//! [dependencies]
//! jiminy-core = { version = "0.15", features = ["solana-zero-copy"] }
//! ```

// Re-export the unaligned primitives for use in overlay structs.
pub use solana_zero_copy::unaligned::{Bool, U16, U32, U64, I16, I64};

// U128 is only available on non-BPF targets.
#[cfg(not(target_arch = "bpf"))]
pub use solana_zero_copy::unaligned::U128;

use crate::account::{Pod, FixedLayout};

// SAFETY: All solana-zero-copy unaligned types are #[repr(transparent)]
// wrappers around byte arrays (or u8 for Bool). They are Copy, all bit
// patterns are valid, and they have no interior references.
unsafe impl Pod for U16 {}
unsafe impl Pod for U32 {}
unsafe impl Pod for U64 {}
unsafe impl Pod for I16 {}
unsafe impl Pod for I64 {}
unsafe impl Pod for Bool {}

#[cfg(not(target_arch = "bpf"))]
unsafe impl Pod for U128 {}

impl FixedLayout for U16 { const SIZE: usize = 2; }
impl FixedLayout for U32 { const SIZE: usize = 4; }
impl FixedLayout for U64 { const SIZE: usize = 8; }
impl FixedLayout for I16 { const SIZE: usize = 2; }
impl FixedLayout for I64 { const SIZE: usize = 8; }
impl FixedLayout for Bool { const SIZE: usize = 1; }

#[cfg(not(target_arch = "bpf"))]
impl FixedLayout for U128 { const SIZE: usize = 16; }

// ── Bidirectional bridges: Le* ↔ solana-zero-copy types ─────────────────────
//
// Jiminy-native Le* types are the canonical ABI surface. These conversions
// let users interop with solana-zero-copy types when needed.

use crate::abi::{LeBool, LeI16, LeU128, LeU16, LeU32, LeU64};

// Integer types share identical inner representation: [u8; N].
macro_rules! impl_le_szc_bridge {
    ($le:ty, $szc:ty) => {
        impl From<$le> for $szc {
            #[inline(always)]
            fn from(v: $le) -> Self {
                Self(v.0)
            }
        }
        impl From<$szc> for $le {
            #[inline(always)]
            fn from(v: $szc) -> Self {
                Self(v.0)
            }
        }
    };
}

impl_le_szc_bridge!(LeU16, U16);
impl_le_szc_bridge!(LeU32, U32);
impl_le_szc_bridge!(LeU64, U64);
impl_le_szc_bridge!(LeI16, I16);

// I64 has a private inner field — convert via bytes.
impl From<crate::abi::LeI64> for I64 {
    #[inline(always)]
    fn from(v: crate::abi::LeI64) -> Self {
        Self::from_primitive(v.get())
    }
}
impl From<I64> for crate::abi::LeI64 {
    #[inline(always)]
    fn from(v: I64) -> Self {
        Self::new(i64::from(v))
    }
}

// Bool: LeBool([u8; 1]) ↔ Bool(u8)
impl From<LeBool> for Bool {
    #[inline(always)]
    fn from(v: LeBool) -> Self {
        Self(v.0[0])
    }
}
impl From<Bool> for LeBool {
    #[inline(always)]
    fn from(v: Bool) -> Self {
        Self([v.0])
    }
}

// U128 bridge only on non-BPF.
#[cfg(not(target_arch = "bpf"))]
impl From<LeU128> for U128 {
    #[inline(always)]
    fn from(v: LeU128) -> Self {
        Self(v.0)
    }
}
#[cfg(not(target_arch = "bpf"))]
impl From<U128> for LeU128 {
    #[inline(always)]
    fn from(v: U128) -> Self {
        Self(v.0)
    }
}
