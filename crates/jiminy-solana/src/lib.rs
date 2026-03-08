#![no_std]
//! **jiminy-solana** — Optional Solana platform helpers for the Jiminy ecosystem.
//!
//! This crate groups utilities that depend on SPL Token, Token-2022, or other
//! Solana platform features. Import it when your program needs token/mint readers,
//! CPI wrappers, Ed25519 verification, Pyth oracles, or other platform-specific
//! functionality.
//!
//! ```rust,ignore
//! use jiminy_solana::prelude::*;
//! ```
//!
//! Depends on [`jiminy_core`] for validation, math, and account IO.

// ── Modules ──────────────────────────────────────────────────────────────────

pub mod authority;
pub mod balance;
pub mod compute;
pub mod compose;
pub mod cpi;
pub mod cpi_guard;
pub mod cpi_return;
pub mod ed25519;
pub mod introspect;
pub mod merkle;
pub mod mint;
pub mod oracle;
pub mod prelude;
pub mod token;
pub mod token_2022;
pub mod twap;
#[cfg(feature = "programs")]
pub mod upgrade;

// ── Re-exports ───────────────────────────────────────────────────────────────

pub use jiminy_core;
pub use pinocchio;
pub use pinocchio_system;
pub use pinocchio_token;
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
