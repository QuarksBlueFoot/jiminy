#![no_std]
//! **Jiminy** - the zero-copy standard library for Solana programs.
//!
//! Every pinocchio needs a conscience.
//!
//! [pinocchio](https://docs.rs/pinocchio) gives you raw bytes and full control.
//! jiminy adds every check, guard, and piece of math that keeps your program
//! honest. `no_std`, `no_alloc`, BPF-safe. One import gives you everything:
//!
//! ```rust,ignore
//! use jiminy::prelude::*;
//! ```
//!
//! # Modules
//!
//! ## Ring 1 - `jiminy_core`
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
//! ## Ring 2 - `jiminy_solana`
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
