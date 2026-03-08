# jiminy-vesting

Token vesting math. Linear + cliff, stepped schedules, periodic unlock,
claimable calculations. Handles the unlock curve so you can focus on the
instruction logic.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit.

## Install

```toml
[dependencies]
jiminy-vesting = "0.11"
```

## What's inside

| Function | What it does |
|---|---|
| `vested_amount` | Linear vesting with cliff. Tokens unlocked at timestamp `now` |
| `check_cliff_reached` | Assert the cliff time has passed |
| `unlocked_at_step` | Stepped schedule. Tokens unlocked after N of M steps |
| `claimable` | Vested minus already-claimed |
| `elapsed_steps` | Count complete step periods since start |

## Quick start

```rust,ignore
use jiminy_vesting::*;

let vested = vested_amount(total, start, cliff, end, now);
check_cliff_reached(cliff, now)?;
let claim = claimable(vested, already_claimed);
```

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0
