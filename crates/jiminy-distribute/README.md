# jiminy-distribute

Dust-safe token distribution. Split a total across N recipients by weight where
the sum always equals the input. Extract basis-point fees without rounding
errors eating your lamports.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit.

## Install

```toml
[dependencies]
jiminy-distribute = "0.11"
```

## What's inside

| Function | What it does |
|---|---|
| `proportional_split` | Split `total` across N recipients by weight, dust-free (remainder goes to last) |
| `extract_fee` | Compute and subtract a basis-point fee, returning `(net, fee)` |

## Quick start

```rust,ignore
use jiminy_distribute::*;

let mut amounts = [0u64; 4];
proportional_split(total_amount, &weights, &mut amounts)?;

let (net, fee) = extract_fee(gross_amount, fee_bps)?;
```

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0
