#![no_std]
//! # jiminy
//!
//! **The zero-copy standard library for Solana programs.**
//!
//! *Every pinocchio needs a conscience.*
//!
//! You chose [pinocchio](https://docs.rs/pinocchio) because you wanted raw
//! performance and full control over your on-chain program. No allocator, no
//! borsh, no framework opinions. Just bytes.
//!
//! But you're still writing the same signer check for the tenth time. You're
//! still hand-rolling `amount * price / 10_000` and praying it doesn't overflow
//! at 4.2 billion tokens. And every check you forget to write is one exploit
//! away from draining your vault.
//!
//! jiminy is the standard library that pinocchio doesn't ship. Every guard,
//! check, reader, and piece of math that DeFi programs need -- packaged as
//! plain functions and declarative macros. `no_std`, `no_alloc`, no proc
//! macros, every function `#[inline(always)]`, pinocchio under the hood.
//!
//! ```rust,ignore
//! use jiminy::prelude::*;
//! ```
//!
//! # Modules
//!
//! ## Ring 1 -- `jiminy_core`
//!
//! | Module | |
//! |---|---|
//! | [`account`] | Header, reader, writer, cursor, lifecycle, pod, list, bits |
//! | [`check`] | Validation checks, asserts, PDA derivation & verification |
//! | [`math`] | Checked arithmetic, BPS, scaling |
//! | [`instruction`] | Transaction introspection (sysvar Instructions) |
//! | [`state`] | State machine transition checks |
//! | [`sysvar`] | Clock & Rent sysvar readers |
//! | [`time`] | Deadline, cooldown, staleness checks |
//! | [`event`] | Zero-alloc event emission via `sol_log_data` |
//! | [`programs`] | Well-known program IDs *(feature: `programs`)* |
//!
//! ## Ring 2 -- `jiminy_solana`
//!
//! | Module | |
//! |---|---|
//! | [`token`] | SPL Token account readers, mint readers, Token-2022 extension screening |
//! | [`cpi`] | Safe CPI wrappers, reentrancy guards, return data readers |
//! | [`crypto`] | Ed25519 precompile verification, Merkle proof verification |
//! | [`authority`] | Two-step authority rotation (propose + accept) |
//! | [`balance`] | Pre/post CPI balance delta guards |
//! | [`compute`] | Compute budget guards |
//! | [`compose`] | Transaction composition guards (flash-loan detection) |
//! | [`introspect`] | Raw transaction introspection |
//! | [`oracle`] | Pyth V2 price feed readers |
//! | [`twap`] | TWAP accumulators |
//! | [`upgrade`] | Program upgrade authority verification *(feature: `programs`)* |
//!
//! ## Ring 3+
//!
//! | Crate | |
//! |---|---|
//! | [`jiminy_finance`] | AMM math, slippage / economic bounds |
//! | [`jiminy_lending`] | Lending protocol primitives |
//! | [`jiminy_staking`] | Staking reward accumulators |
//! | [`jiminy_vesting`] | Vesting schedule helpers |
//! | [`jiminy_multisig`] | M-of-N multi-signer threshold |
//! | [`jiminy_distribute`] | Dust-safe proportional distribution |
//!
//! # Macros
//!
//! All macros are declarative (`macro_rules!`). No proc macros, no build
//! dependency, no compile-time surprises.
//!
//! | Macro | |
//! |---|---|
//! | [`require!`] | `if !cond { return Err(e) }` -- the universal guard |
//! | [`require_keys_eq!`] | Two addresses must match |
//! | [`require_keys_neq!`] | Two addresses must differ |
//! | [`require_gte!`] | `a >= b` |
//! | [`require_gt!`] | `a > b` |
//! | [`require_lt!`] | `a < b` |
//! | [`require_lte!`] | `a <= b` |
//! | [`require_eq!`] | Scalar equality |
//! | [`require_neq!`] | Scalar inequality |
//! | [`require_flag!`] | Bit must be set |
//! | [`check_accounts_unique!`] | Pairwise uniqueness for any N accounts |
//! | [`error_codes!`] | Sequential error constants + `Into<ProgramError>` |
//! | [`instruction_dispatch!`] | Byte-tag dispatch to handler functions |
//! | [`impl_pod!`] | Batch `unsafe impl Pod` for primitive types |

// ── Ring 1: systems layer (from jiminy-core) ─────────────────────────────────

pub use jiminy_core::{account, check, event, instruction, math, state, sysvar, time};

#[cfg(feature = "programs")]
pub use jiminy_core::programs;

#[cfg(feature = "log")]
pub use jiminy_core::log;

// ── Ring 2: platform helpers (from jiminy-solana) ────────────────────────────

pub use jiminy_solana::{
    authority, balance, compute, compose, cpi, crypto, introspect, oracle, token, twap,
};

#[cfg(feature = "programs")]
pub use jiminy_solana::upgrade;

// ── Ring 3+: protocol domain crates ──────────────────────────────────────────

pub mod amm {
    //! AMM math: integer square root, constant-product swap formulas, LP minting.
    pub use jiminy_finance::amm::*;
}

pub mod slippage {
    //! Slippage and economic bound checks.
    pub use jiminy_finance::slippage::*;
}

pub mod lending {
    //! Lending protocol primitives: collateralization, liquidation, interest.
    pub use jiminy_lending::*;
}

pub mod staking {
    //! Staking reward accumulators (MasterChef-style).
    pub use jiminy_staking::*;
}

pub mod vesting {
    //! Vesting schedule helpers: linear, cliff, stepped, periodic.
    pub use jiminy_vesting::*;
}

pub mod multisig {
    //! M-of-N multi-signer threshold checks.
    pub use jiminy_multisig::*;
}

pub mod distribute {
    //! Dust-safe proportional distribution and fee extraction.
    pub use jiminy_distribute::*;
}

// ── Subcrate re-exports (for direct crate access) ───────────────────────────

pub use jiminy_core;
pub use jiminy_solana;
pub use jiminy_finance;
pub use jiminy_lending;
pub use jiminy_staking;
pub use jiminy_vesting;
pub use jiminy_multisig;
pub use jiminy_distribute;

// ── Pinocchio ecosystem ──────────────────────────────────────────────────────

pub use pinocchio;
pub use pinocchio_system;
pub use pinocchio_token;
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

// ── Prelude ──────────────────────────────────────────────────────────────────

pub mod prelude;

// ── Macros ───────────────────────────────────────────────────────────────────
//
// These are defined here (not re-exported from jiminy_core) because
// `#[macro_export]` puts them in the root crate namespace. Having them in
// both crates is harmless; users get whichever crate they depend on.

/// Require a boolean condition: return `$err` (converted via `Into`) if false.
#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be equal.
#[macro_export]
macro_rules! require_keys_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a != *$b {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be **different**.
#[macro_export]
macro_rules! require_keys_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a == *$b {
            return Err($err.into());
        }
    };
}

/// Require two accounts to have **different** addresses.
#[macro_export]
macro_rules! require_accounts_ne {
    ($a:expr, $b:expr, $err:expr) => {
        if $a.address() == $b.address() {
            return Err($err.into());
        }
    };
}

/// Require `a >= b`.
#[macro_export]
macro_rules! require_gte {
    ($a:expr, $b:expr, $err:expr) => {
        if $a < $b {
            return Err($err.into());
        }
    };
}

/// Require `a > b`.
#[macro_export]
macro_rules! require_gt {
    ($a:expr, $b:expr, $err:expr) => {
        if $a <= $b {
            return Err($err.into());
        }
    };
}

/// Require `a < b`.
#[macro_export]
macro_rules! require_lt {
    ($a:expr, $b:expr, $err:expr) => {
        if $a >= $b {
            return Err($err.into());
        }
    };
}

/// Require `a <= b`.
#[macro_export]
macro_rules! require_lte {
    ($a:expr, $b:expr, $err:expr) => {
        if $a > $b {
            return Err($err.into());
        }
    };
}

/// Require `a == b` for scalar types.
#[macro_export]
macro_rules! require_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a != $b {
            return Err($err.into());
        }
    };
}

/// Require `a != b` for scalar types.
#[macro_export]
macro_rules! require_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a == $b {
            return Err($err.into());
        }
    };
}

/// Require bit `n` to be set in `$byte`, else return `$err`.
#[macro_export]
macro_rules! require_flag {
    ($byte:expr, $n:expr, $err:expr) => {
        if ($byte >> $n) & 1 == 0 {
            return Err($err.into());
        }
    };
}

/// Verify that all passed accounts have unique addresses.
///
/// Variadic -- works with 2, 3, 4, or more accounts. Expands to
/// pairwise `!=` checks at compile time. No heap, no loops.
///
/// ```rust,ignore
/// check_accounts_unique!(payer, vault, mint);
/// check_accounts_unique!(a, b, c, d);
/// ```
#[macro_export]
macro_rules! check_accounts_unique {
    ($a:expr, $b:expr) => {
        if $a.address() == $b.address() {
            return Err($crate::pinocchio::error::ProgramError::InvalidArgument);
        }
    };
    ($head:expr, $($tail:expr),+ $(,)?) => {
        $( if $head.address() == $tail.address() {
            return Err($crate::pinocchio::error::ProgramError::InvalidArgument);
        } )+
        $crate::check_accounts_unique!($($tail),+);
    };
}

/// Define numbered program error codes that map to `ProgramError::Custom`.
///
/// Replaces Anchor's `#[error_code]` proc macro. Each variant gets a
/// sequential `u32` discriminant offset from the base you provide.
///
/// ```rust,ignore
/// error_codes! {
///     base = 6000;
///     Undercollateralized,   // 6000
///     Expired,               // 6001
///     InvalidOracle,         // 6002
/// }
/// ```
#[macro_export]
macro_rules! error_codes {
    (
        base = $base:expr;
        $( $(#[$meta:meta])* $name:ident ),+ $(,)?
    ) => {
        #[allow(non_upper_case_globals)]
        pub mod errors {
            $crate::error_codes!(@count $base; $( $(#[$meta])* $name ),+ );
        }
        $(
            impl From<errors::$name> for $crate::pinocchio::error::ProgramError {
                #[inline(always)]
                fn from(_: errors::$name) -> Self {
                    $crate::pinocchio::error::ProgramError::Custom(errors::$name::CODE)
                }
            }
        )+
    };
    (@count $code:expr; $(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub struct $name;
        impl $name { pub const CODE: u32 = $code; }
    };
    (@count $code:expr; $(#[$meta:meta])* $name:ident, $( $(#[$rmeta:meta])* $rest:ident ),+ ) => {
        $(#[$meta])*
        pub struct $name;
        impl $name { pub const CODE: u32 = $code; }
        $crate::error_codes!(@count $code + 1; $( $(#[$rmeta])* $rest ),+ );
    };
}

/// Route instruction data to handler functions based on a single-byte tag.
///
/// Reads byte 0 as the discriminator and dispatches to the matching
/// handler. Returns `InvalidInstructionData` for unknown tags.
///
/// ```rust,ignore
/// instruction_dispatch! {
///     program_id, accounts, instruction_data;
///     0 => process_init(program_id, accounts, ix),
///     1 => process_deposit(program_id, accounts, ix),
/// }
/// ```
///
/// Inside each arm, `ix` is a [`account::SliceCursor`] positioned after the tag byte.
#[macro_export]
macro_rules! instruction_dispatch {
    (
        $pid:expr, $accs:expr, $data:expr;
        $( $tag:expr => $handler:expr ),+ $(,)?
    ) => {{
        let mut ix = $crate::account::SliceCursor::new($data);
        let tag = ix.read_u8()?;
        match tag {
            $( $tag => { let _ = &ix; $handler } )+
            _ => Err($crate::pinocchio::error::ProgramError::InvalidInstructionData),
        }
    }};
}

/// Batch `unsafe impl Pod` for a list of types.
///
/// ```rust,ignore
/// impl_pod!(u8, u16, u32, u64, MyStruct);
/// ```
#[macro_export]
macro_rules! impl_pod {
    ($($t:ty),+ $(,)?) => {
        $( unsafe impl $crate::account::Pod for $t {} )+
    };
}
