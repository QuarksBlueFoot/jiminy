# jiminy-lending

Lending protocol math. Collateralization ratios, health checks, liquidation
sizing, interest accrual, utilization rates. All basis-point denominated, all
overflow-safe. Drop it into your lending program and stop re-deriving the
formulas.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit.

## Install

```toml
[dependencies]
jiminy-lending = "0.11"
```

## What's inside

| Function | What it does |
|---|---|
| `collateralization_ratio_bps` | Collateral-to-debt ratio in basis points |
| `check_healthy` | Assert position is above minimum collateral threshold |
| `check_liquidatable` | Assert position is below liquidation threshold |
| `max_liquidation_amount` | Max repayable in a single liquidation (close-factor) |
| `liquidation_seize_amount` | Collateral to seize given repay amount + bonus BPS |
| `simple_interest` | Accrue interest over elapsed seconds at a per-second rate |
| `utilization_rate_bps` | Borrow utilization as basis points |

## Quick start

```rust,ignore
use jiminy_lending::*;

let ratio = collateralization_ratio_bps(collateral_value, debt_value)?;
check_healthy(collateral_value, debt_value, min_ratio_bps)?;
```

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0
