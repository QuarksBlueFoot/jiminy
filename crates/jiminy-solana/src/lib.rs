#![no_std]
//! **jiminy-solana** - Token readers, CPI guards, Ed25519, Merkle proofs,
//! Pyth oracles, authority rotation, and the rest of the Solana platform
//! helpers that depend on SPL Token / Token-2022.
//!
//! ```rust,ignore
//! use jiminy_solana::prelude::*;
//! ```
//!
//! # Modules
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
//! Depends on [`jiminy_core`] for validation, math, and account IO.

// ── Domain modules ───────────────────────────────────────────────────────────

pub mod token;
pub mod cpi;
pub mod crypto;

pub mod authority;
pub mod balance;
pub mod compute;
pub mod compose;
pub mod introspect;
pub mod oracle;
pub mod prelude;
pub mod twap;

#[cfg(feature = "programs")]
pub mod upgrade;

// ── Re-exports ───────────────────────────────────────────────────────────────

pub use jiminy_core;
pub use pinocchio;
pub use pinocchio_system;
pub use pinocchio_token;
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
