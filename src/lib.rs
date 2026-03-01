#![no_std]
//! **Jiminy** - Anchor-style safety abstractions for [pinocchio](https://docs.rs/pinocchio) programs.
//!
//! Pinocchio is the engine. Jiminy keeps it honest.
//!
//! Zero-copy, no alloc, BPF-safe. Check functions, zero-copy data cursors,
//! bit flag helpers, well-known program IDs, and iterator-style account
//! validation - so you can focus on program logic, not boilerplate.
//!
//! # Quick-start
//!
//! ```rust,ignore
//! use jiminy::{
//!     check_account, check_signer, check_writable, check_executable,
//!     check_lamports_gte, safe_close, checked_add,
//!     require, require_accounts_ne, require_keys_neq,
//!     SliceCursor, DataWriter, zero_init, write_discriminator,
//!     AccountList, programs,
//!     read_bit, set_bit, clear_bit,
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
//! | `check_executable` | account is an executable program |
//! | `check_uninitialized` | account has no data yet (anti-reinit) |
//! | `check_has_one` | stored address field matches account key |
//! | `check_keys_eq` | two addresses are equal |
//! | `check_lamports_gte` | account holds at least N lamports |
//! | `check_rent_exempt` | account holds enough lamports for rent exemption |
//! | `check_closed` | account has zero lamports and empty data |
//! | `check_size` | raw data slice is at least N bytes |
//! | `check_discriminator` | first byte matches expected type tag |
//! | `check_account` | owner + size + discriminator in one call |
//! | `rent_exempt_min` | compute minimum lamports for rent exemption |
//!
//! # Assert functions
//!
//! | Function | What it does |
//! |---|---|
//! | `assert_pda` | derive PDA, verify match, return bump |
//! | `assert_pda_with_bump` | verify PDA with known bump (cheaper) |
//! | `assert_pda_external` | same as `assert_pda` for external programs |
//! | `assert_token_program` | account is SPL Token or Token-2022 |
//! | `assert_address` | account address matches expected key |
//! | `assert_program` | address matches + account is executable |
//! | `assert_not_initialized` | lamports == 0 (account doesn't exist yet) |
//!
//! # Token account readers
//!
//! | Function | What it reads |
//! |---|---|
//! | `token_account_owner` | owner field (bytes 32..64) |
//! | `token_account_amount` | amount field (bytes 64..72) |
//! | `token_account_mint` | mint field (bytes 0..32) |
//! | `token_account_delegate` | delegate field (Option, bytes 72..108) |
//!
//! # PDA utilities
//!
//! | Macro / Function | What it does |
//! |---|---|
//! | `find_pda!` | find canonical PDA + bump via syscall |
//! | `derive_pda!` | derive PDA with known bump (cheap, ~100 CU) |
//! | `derive_pda_const!` | derive PDA at compile time |
//! | `derive_ata` | derive ATA address for wallet + mint |
//! | `derive_ata_with_program` | derive ATA with explicit token program |
//! | `derive_ata_with_bump` | derive ATA with known bump (cheap) |
//! | `derive_ata_const!` | derive ATA at compile time |
//!
//! # Zero-copy cursors
//!
//! [`SliceCursor`] reads typed fields sequentially from account data.
//! [`DataWriter`] writes them when initializing a new account.
//! [`zero_init`] zero-fills account data before the first write.
//!
//! # Account iteration
//!
//! [`AccountList`] provides iterator-style account consumption with
//! inline constraint checks, replacing manual index arithmetic.
//!
//! # Well-known program IDs
//!
//! [`programs`] module: `SYSTEM`, `TOKEN`, `TOKEN_2022`, `ASSOCIATED_TOKEN`,
//! `METADATA`, `BPF_LOADER`, `COMPUTE_BUDGET`, `SYSVAR_CLOCK`, `SYSVAR_RENT`,
//! `SYSVAR_INSTRUCTIONS`.

#[cfg(feature = "programs")]
pub mod programs;

mod accounts;
mod asserts;
mod bits;
mod checks;
mod close;
mod cursor;
mod header;
mod math;
mod pda;
pub mod prelude;
mod token;

pub use accounts::AccountList;
pub use asserts::*;
pub use bits::*;
pub use checks::*;
pub use close::*;
pub use cursor::{write_discriminator, zero_init, DataWriter, SliceCursor};
pub use header::*;
pub use math::*;
pub use pda::*;
pub use token::*;

// Re-export pinocchio core types so users only need one import.
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

// ── Macros ───────────────────────────────────────────────────────────────────

/// Require a boolean condition: return `$err` (converted via `Into`) if false.
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

/// Require two [`Address`] values to be **different**.
///
/// The counterpart to `require_keys_eq!`. Useful for authority rotation
/// checks, multi-hop validations, or any time two program-owned addresses
/// must not collide.
///
/// ```rust,ignore
/// require_keys_neq!(old_authority, new_authority, MyError::SameKey);
/// ```
#[macro_export]
macro_rules! require_keys_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a == *$b {
            return Err($err.into());
        }
    };
}

/// Require two accounts to have **different** addresses.
///
/// Prevents source == destination attacks common in token and escrow
/// programs. Anchor has no built-in constraint for this case.
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
///
/// The counterpart to `require_eq!`. Use `require_keys_neq!` for addresses.
///
/// ```rust,ignore
/// require_neq!(new_value, current_value, MyError::NoChange);
/// ```
#[macro_export]
macro_rules! require_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a == $b {
            return Err($err.into());
        }
    };
}

/// Require bit `n` to be set in `$byte`, else return `$err`.
///
/// ```rust,ignore
/// require_flag!(state.flags, 0, MyError::AccountLocked);
/// ```
#[macro_export]
macro_rules! require_flag {
    ($byte:expr, $n:expr, $err:expr) => {
        if ($byte >> $n) & 1 == 0 {
            return Err($err.into());
        }
    };
}
