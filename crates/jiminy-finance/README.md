# jiminy-finance

DeFi math that doesn't overflow. AMM constant-product swaps with u128
intermediates, slippage guards, economic bounds, price impact. The math you keep
re-deriving in every protocol.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit. Depends on `jiminy-core` for validation and math.

## Install

```toml
[dependencies]
jiminy-finance = "0.11"
```

## What's inside

| Module | What it does |
|---|---|
| `amm` | `isqrt`, `constant_product_out`, `constant_product_in`, `check_k_invariant`, `price_impact_bps`, `initial_lp_amount`, `proportional_lp_amount` |
| `slippage` | `check_slippage`, `check_max_input`, `check_min_amount`, `check_max_amount`, `check_nonzero`, `check_within_bps`, `check_price_bounds` |

## Quick start

```rust,ignore
use jiminy_finance::prelude::*;

// Constant-product swap
let out = constant_product_out(amount_in, reserve_in, reserve_out, fee_bps)?;

// Slippage guard
check_slippage(actual_output, minimum_output)?;
```

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0
