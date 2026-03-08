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
//! # Ring 1: what lives here
//!
//! | Layer | Types / Functions |
//! |---|---|
//! | **Account grammar** | [`AccountHeader`], [`HEADER_LEN`], [`body`], [`body_mut`], [`check_header`] |
//! | **Zero-copy IO** | [`AccountReader`], [`AccountWriter`], [`SliceCursor`], [`DataWriter`] |
//! | **POD casting** | [`Pod`], [`FixedLayout`], [`pod_from_bytes`], [`pod_from_bytes_mut`] |
//! | **Validation** | [`check_signer`], [`check_owner`], [`check_account`], [`assert_pda`] … |
//! | **PDA utilities** | [`find_pda!`], [`derive_pda!`], [`derive_ata`], [`check_ata`] … |
//! | **Instruction access** | [`instruction::current_index`], [`instruction::program_id_at`] … |
//! | **Lifecycle** | [`safe_close`], [`safe_realloc`], [`zero_init`], [`check_not_revived`] |
//! | **Math** | [`checked_add`], [`checked_mul_div`], [`bps_of`], [`scale_amount`] … |
//! | **Time & state** | [`check_not_expired`], [`check_state_transition`], [`write_state`] |
//! | **Events** | [`emit!`], [`emit_slices`] |
//! | **Well-known IDs** | [`programs::TOKEN`], [`programs::SYSTEM`], [`programs::BPF_LOADER`] … |
//!
//! # What does NOT belong here
//!
//! Token/mint readers, Token-2022 screening, CPI guards, Ed25519, Merkle,
//! oracles, AMM math, lending, staking, vesting — see `jiminy-solana`,
//! `jiminy-finance`, and other optional crates.

// ── Modules ──────────────────────────────────────────────────────────────────

#[cfg(feature = "programs")]
pub mod programs;

mod accounts;
pub mod account_io;
mod asserts;
mod bits;
mod checks;
pub mod cursor;
pub mod event;
mod header;
pub mod instruction;
pub mod lifecycle;
#[cfg(feature = "log")]
pub mod log;
mod math;
mod pda;
pub mod pod;
pub mod prelude;
pub mod slippage;
pub mod state;
mod sysvar;
mod time;

// ── Re-exports at crate root ─────────────────────────────────────────────────

pub use accounts::AccountList;
pub use account_io::{AccountReader, AccountWriter};
pub use asserts::*;
pub use bits::*;
pub use checks::*;
pub use cursor::{write_discriminator, zero_init, DataWriter, SliceCursor};
pub use header::*;
pub use lifecycle::{
    safe_close, safe_close_with_sentinel, check_not_revived, check_alive,
    safe_realloc, safe_realloc_shrink, CLOSE_SENTINEL,
};
pub use math::*;
pub use pda::*;
pub use pod::{Pod, FixedLayout, pod_from_bytes, pod_from_bytes_mut, pod_write};
pub use sysvar::*;
pub use time::*;

// Re-export pinocchio so downstream crates and programs can depend on just jiminy-core.
pub use pinocchio;

// Re-export common types at crate root.
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
