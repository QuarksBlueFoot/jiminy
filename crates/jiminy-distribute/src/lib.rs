#![no_std]
//! **jiminy-distribute** — Dust-safe proportional distribution helpers for the Jiminy ecosystem.
//!
//! N-way splits and fee extraction where `sum == total`.

mod distribute;
pub use distribute::*;
pub use pinocchio;
