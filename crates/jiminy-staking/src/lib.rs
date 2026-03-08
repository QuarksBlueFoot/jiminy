#![no_std]
//! **jiminy-staking** — Staking reward accumulators for the Jiminy ecosystem.
//!
//! MasterChef-style reward-per-token accumulator, emission rates, pending
//! rewards, and reward debt tracking.

mod staking;
pub use staking::*;
pub use pinocchio;
