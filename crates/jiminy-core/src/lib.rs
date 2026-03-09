#![no_std]
//! # jiminy-core
//!
//! Account layout, validation, math, PDA, and all the zero-copy primitives
//! your pinocchio program needs before it touches a token.
//!
//! This is the systems layer. Everything here works with raw `AccountView`
//! bytes and has zero dependencies beyond pinocchio itself. If your program
//! never calls SPL Token, this crate is all you need.
//!
//! ```rust,ignore
//! use jiminy_core::prelude::*;
//! ```
//!
//! # Modules
//!
//! | Module | |
//! |---|---|
//! | [`account`] | Header, reader, writer, cursor, lifecycle, pod, list, bits |
//! | [`check`] | Validation checks, asserts, PDA derivation & verification |
//! | [`instruction`] | Transaction introspection (sysvar Instructions) |
//! | [`math`] | Checked arithmetic, BPS, scaling |
//! | [`sysvar`] | Clock & Rent sysvar readers |
//! | [`state`] | State machine transition checks |
//! | [`time`] | Deadline, cooldown, staleness checks |
//! | [`event`] | Zero-alloc event emission via `sol_log_data` |
//! | [`programs`] | Well-known program IDs *(feature: `programs`)* |
//!
//! # Macros
//!
//! All macros are declarative (`macro_rules!`). No proc macros.
//!
//! | Macro | |
//! |---|---|
//! | [`require!`] | `if !cond { return Err(e) }` -- the universal guard |
//! | [`require_keys_eq!`] | Two addresses must match |
//! | [`check_accounts_unique!`] | Pairwise uniqueness for any N accounts |
//! | [`error_codes!`] | Define numbered error codes without a proc macro |
//! | [`instruction_dispatch!`] | Byte-tag instruction routing |
//! | [`impl_pod!`] | Batch `unsafe impl Pod` |
//!
//! # What does NOT belong here
//!
//! Token/mint readers, Token-2022 screening, CPI guards, Ed25519, Merkle,
//! oracles, AMM math, lending, staking, vesting -- see `jiminy-solana`,
//! `jiminy-finance`, and other domain crates.

// ── Domain modules ───────────────────────────────────────────────────────────

pub mod account;
pub mod check;
pub mod event;
pub mod instruction;
pub mod math;
pub mod prelude;
pub mod state;
pub mod sysvar;
pub mod time;

#[cfg(feature = "log")]
pub mod log;

#[cfg(feature = "programs")]
pub mod programs;

// ── Pinocchio re-exports ─────────────────────────────────────────────────────
//
// Downstream crates depend on just jiminy-core; they get pinocchio for free.

pub use pinocchio;
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

// ── Macros ───────────────────────────────────────────────────────────────────

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
/// Variadic — works with 2, 3, 4, or more accounts. Expands to
/// pairwise `!=` checks at compile time. No heap, no loops.
///
/// Replaces `check_accounts_unique_2`, `check_accounts_unique_3`, etc.
///
/// ```rust,ignore
/// check_accounts_unique!(payer, vault, mint);
/// check_accounts_unique!(a, b, c, d);
/// ```
#[macro_export]
macro_rules! check_accounts_unique {
    // Base case: two accounts.
    ($a:expr, $b:expr) => {
        if $a.address() == $b.address() {
            return Err($crate::pinocchio::error::ProgramError::InvalidArgument);
        }
    };
    // Recursive: compare head against every tail, then recurse on tail.
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
/// `u32` discriminant offset from a base you provide. The macro emits
/// constants and an `Into<ProgramError>` conversion.
///
/// ```rust,ignore
/// error_codes! {
///     base = 6000;
///     Undercollateralized,   // 6000
///     Expired,               // 6001
///     InvalidOracle,         // 6002
/// }
///
/// // Use in require! or return Err(...)
/// require!(collateral >= min, Undercollateralized);
/// ```
#[macro_export]
macro_rules! error_codes {
    (
        base = $base:expr;
        $( $(#[$meta:meta])* $name:ident ),+ $(,)?
    ) => {
        /// Program-specific error codes.
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
    // Internal counter arm — assigns sequential codes.
    (@count $code:expr; $(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub struct $name;
        impl $name {
            pub const CODE: u32 = $code;
        }
    };
    (@count $code:expr; $(#[$meta:meta])* $name:ident, $( $(#[$rmeta:meta])* $rest:ident ),+ ) => {
        $(#[$meta])*
        pub struct $name;
        impl $name {
            pub const CODE: u32 = $code;
        }
        $crate::error_codes!(@count $code + 1; $( $(#[$rmeta])* $rest ),+ );
    };
}

/// Route instruction data to handler functions based on a single-byte tag.
///
/// Replaces Anchor's `#[program]` proc macro. Reads byte 0 as the
/// discriminator and dispatches to the matching handler. Returns
/// `InvalidInstructionData` for unknown tags.
///
/// ```rust,ignore
/// instruction_dispatch! {
///     program_id, accounts, instruction_data;
///     0 => process_init(program_id, accounts, ix),
///     1 => process_deposit(program_id, accounts, ix),
///     2 => process_withdraw(program_id, accounts, ix),
/// }
/// ```
///
/// Inside each arm, `ix` is a `SliceCursor` positioned after the tag byte.
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
