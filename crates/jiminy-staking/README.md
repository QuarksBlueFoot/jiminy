# jiminy-staking

MasterChef-style staking math. Global reward-per-token accumulator, per-user
pending rewards, reward debt snapshots, emission rates. The standard pattern,
done once, done right.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit.

## Install

```toml
[dependencies]
jiminy-staking = "0.11"
```

## What's inside

| Function | What it does |
|---|---|
| `update_reward_per_token` | Advance the global accumulator given elapsed time and total staked |
| `pending_rewards` | Unclaimed rewards for a user position |
| `update_reward_debt` | Snapshot reward debt after claim or stake change |
| `emission_rate` | Per-second rate from total rewards and duration |
| `rewards_earned` | Total rewards emitted over an elapsed period |

Precision constant: `REWARD_PRECISION = 1_000_000_000_000` (10^12)

## Quick start

```rust,ignore
use jiminy_staking::*;

let new_rpt = update_reward_per_token(
    current_rpt, last_update, now, rate_per_second, total_staked,
)?;
let pending = pending_rewards(user_staked, new_rpt, user_reward_debt)?;
```

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0
