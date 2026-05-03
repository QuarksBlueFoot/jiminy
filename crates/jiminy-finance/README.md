# jiminy-finance

AMM math, slippage guards, price impact, economic bounds. u128 intermediates so your constant-product swaps don't overflow.

```toml
jiminy-finance = "0.17"
```

`amm` - `isqrt`, `constant_product_out`, `constant_product_in`, `check_k_invariant`, `price_impact_bps`, `initial_lp_amount`, `proportional_lp_amount`

`slippage` - `check_slippage`, `check_max_input`, `check_min_amount`, `check_max_amount`, `check_nonzero`, `check_within_bps`, `check_price_bounds`

```rust,ignore
use jiminy_finance::prelude::*;

// Signature: (reserve_in, reserve_out, amount_in, fee_bps).
// fee_bps must be < 10_000. A 100% fee would underflow the fee factor, so
// the function rejects anything >= 10_000 with InvalidArgument.
let out = constant_product_out(reserve_in, reserve_out, amount_in, 30)?; // 30 bps fee
check_slippage(actual_output, minimum_output)?;
```

`#![no_std]` / `no_alloc` / BPF-safe / Depends on `jiminy-core`

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs) / Apache-2.0
