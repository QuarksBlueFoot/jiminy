#![no_std]
//! **jiminy-core** — The zero-copy systems layer for low-level Solana programs.
//!
//! Account layout, validation, lifecycle, and transaction introspection
//! without framework bloat. `#![no_std]`, `no_alloc`, BPF-safe.
//!
//! One import gives you the core:
//!
//! ```rust,ignore
//! use jiminy_core::prelude::*;
//! ```
//!
//! # Module organisation
//!
//! | Module | Purpose |
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
//! # What does NOT belong here
//!
//! Token/mint readers, Token-2022 screening, CPI guards, Ed25519, Merkle,
//! oracles, AMM math, lending, staking, vesting — see `jiminy-solana`,
//! `jiminy-finance`, and other optional crates.

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
