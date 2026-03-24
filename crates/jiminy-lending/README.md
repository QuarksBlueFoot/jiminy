# jiminy-lending

Collateralization ratios, health checks, liquidation sizing, interest accrual, utilization rates. All basis-point denominated, all overflow-safe.

```toml
jiminy-lending = "0.15"
```

Functions: `collateralization_ratio_bps`, `check_healthy`, `check_liquidatable`,
`max_liquidation_amount`, `liquidation_seize_amount`, `simple_interest`,
`utilization_rate_bps`

```rust,ignore
use jiminy_lending::*;

let ratio = collateralization_ratio_bps(collateral_value, debt_value)?;
check_healthy(collateral_value, debt_value, min_ratio_bps)?;
```

`#![no_std]` / `no_alloc` / BPF-safe / Apache-2.0

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs)
