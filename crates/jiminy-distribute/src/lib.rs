#![no_std]
//! # jiminy-distribute
//!
//! Weighted splits and basis-point fee extraction.
//!
//! Split a token amount N ways by weight, extract protocol fees, and
//! guarantee that `sum(parts) == total` -- no dust left behind. If you've
//! ever had a distribution that silently loses 1 lamport per split, this
//! crate is why you don't have to debug that again.

mod distribute;
pub use distribute::*;
pub use hopper_runtime;
