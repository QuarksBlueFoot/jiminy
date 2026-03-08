#![no_std]
//! **jiminy-staking** - MasterChef-style reward accumulators.
//!
//! MasterChef-style reward-per-token accumulator, emission rates, pending
//! rewards, and reward debt tracking.

mod staking;
pub use staking::*;
pub use pinocchio;
