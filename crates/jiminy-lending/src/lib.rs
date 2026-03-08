#![no_std]
//! **jiminy-lending** — Lending protocol primitives for the Jiminy ecosystem.
//!
//! Collateralization ratios, health checks, liquidation math, interest
//! calculations, and utilization rates. All basis-point denominated.

mod lending;
pub use lending::*;
pub use pinocchio;
