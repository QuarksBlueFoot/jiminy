#![no_std]
//! **jiminy-distribute** - Weighted splits and basis-point fee extraction.
//!
//! N-way splits and fee extraction where `sum == total`.

mod distribute;
pub use distribute::*;
pub use pinocchio;
