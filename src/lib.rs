#![no_std]
//! **Jiminy** — Anchor-style safety abstractions for [pinocchio](https://docs.rs/pinocchio) programs.
//!
//! Pinocchio is the engine. Jiminy keeps it honest.
//!
//! Zero-copy, no alloc, BPF-safe. Provides composable check functions,
//! zero-copy data cursors, and macros for the most common instruction
//! guard-rails — so you can focus on program logic, not boilerplate.
//!
//! # Quick-start
//!
//! ```rust,ignore
//! use jiminy::{
//!     check_account, check_signer, check_writable, check_system_program,
//!     check_uninitialized, check_lamports_gte, safe_close,
//!     checked_add, checked_sub,
//!     require, require_accounts_ne,
//!     SliceCursor, DataWriter, write_discriminator,
//! };
//! ```
//!
//! # Check functions
//!
//! | Function | What it verifies |
//! |---|---|
//! | `check_signer` | account is a transaction signer |
//! | `check_writable` | account is marked writable |
//! | `check_owner` | account is owned by your program |
//! | `check_pda` | account address equals a derived PDA |
//! | `check_system_program` | account is the system program |
//! | `check_uninitialized` | account has no data yet (anti-reinit) |
//! | `check_lamports_gte` | account holds at least N lamports |
//! | `check_closed` | account has zero lamports and empty data |
//! | `check_size` | raw data slice is at least N bytes |
//! | `check_discriminator` | first byte matches expected type tag |
//! | `check_account` | owner + size + discriminator in one call |
//!
//! # Zero-copy cursors
//!
//! [`SliceCursor`] reads typed fields sequentially from account data.
//! [`DataWriter`] writes them when initializing a new account.
//! Both are bounds-checked and compile to the same byte reads you'd
//! write by hand.

mod checks;
mod close;
mod cursor;
mod math;

pub use checks::*;
pub use close::*;
pub use cursor::{write_discriminator, DataWriter, SliceCursor};
pub use math::*;

// Re-export pinocchio core types so users only need one import.
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

// ── Macros ───────────────────────────────────────────────────────────────────

/// Require a boolean condition — return `$err` (converted via `Into`) if false.
///
/// Equivalent to Anchor's `require!`.
///
/// ```rust,ignore
/// require!(amount > 0, MyError::ZeroAmount);
/// ```
#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be equal.
///
/// ```rust,ignore
/// require_keys_eq!(vault.owner(), &expected_owner, MyError::WrongOwner);
/// ```
#[macro_export]
macro_rules! require_keys_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a != *$b {
            return Err($err.into());
        }
    };
}

/// Require two accounts to have **different** addresses.
///
/// Prevents source == destination attacks that are common in token and
/// escrow programs. Anchor has no built-in constraint for this case —
/// you need a custom constraint or inline logic. Here it's one line.
///
/// ```rust,ignore
/// require_accounts_ne!(source_vault, dest_vault, MyError::SameAccount);
/// ```
#[macro_export]
macro_rules! require_accounts_ne {
    ($a:expr, $b:expr, $err:expr) => {
        if $a.address() == $b.address() {
            return Err($err.into());
        }
    };
}

/// Require `a >= b`.
///
/// ```rust,ignore
/// require_gte!(balance, amount, MyError::InsufficientFunds);
/// ```
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

/// Require `a == b` for non-Address scalar types.
#[macro_export]
macro_rules! require_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a != $b {
            return Err($err.into());
        }
    };
}
