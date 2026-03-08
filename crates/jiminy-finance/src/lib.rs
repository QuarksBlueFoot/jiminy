#![no_std]
//! **jiminy-finance** - AMM math, slippage guards, price impact, economic bounds.
//!
//! AMM math (integer square root, constant-product swap formulas, k-invariant
//! verification, LP token minting), slippage/economic bounds, and balance-delta
//! guards.
//!
//! ```rust,ignore
//! use jiminy_finance::prelude::*;
//! ```

pub mod amm;
pub mod prelude;
pub mod slippage;

// ── Re-exports ───────────────────────────────────────────────────────────────

pub use amm::{
    isqrt, constant_product_out, constant_product_in, check_k_invariant,
    price_impact_bps, initial_lp_amount, proportional_lp_amount,
};
pub use slippage::{
    check_max_amount, check_max_input, check_min_amount, check_nonzero,
    check_price_bounds, check_slippage, check_within_bps,
};
pub use pinocchio;
