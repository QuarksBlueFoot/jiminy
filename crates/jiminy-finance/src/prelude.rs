//! Convenience re-exports for jiminy-finance.
//!
//! ```rust,ignore
//! use jiminy_finance::prelude::*;
//! ```

pub use crate::amm::{
    isqrt, constant_product_out, constant_product_in, check_k_invariant,
    price_impact_bps, initial_lp_amount, proportional_lp_amount,
};

pub use crate::slippage::{
    check_max_amount, check_max_input, check_min_amount, check_nonzero,
    check_price_bounds, check_slippage, check_within_bps,
};
