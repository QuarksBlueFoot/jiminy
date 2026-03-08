#![no_std]
//! **jiminy-lending** - Collateralization, liquidation, interest, utilization.
//!
//! Collateralization ratios, health checks, liquidation math, interest
//! calculations, and utilization rates. All basis-point denominated.

mod lending;
pub use lending::*;
pub use pinocchio;
