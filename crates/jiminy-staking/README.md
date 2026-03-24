# jiminy-staking

MasterChef-style staking math. Global reward-per-token accumulator, per-user pending rewards, reward debt snapshots, emission rates.

```toml
jiminy-staking = "0.15"
```

- `update_reward_per_token` - advance the global accumulator
- `pending_rewards` - unclaimed rewards for a user
- `update_reward_debt` - snapshot after claim or stake change
- `emission_rate` - per-second rate from total rewards and duration
- `rewards_earned` - total emitted over elapsed period

Precision: `REWARD_PRECISION = 1_000_000_000_000` (10^12)

```rust,ignore
use jiminy_staking::*;

let new_rpt = update_reward_per_token(
    current_rpt, last_update, now, rate_per_second, total_staked,
)?;
let pending = pending_rewards(user_staked, new_rpt, user_reward_debt)?;
```

`#![no_std]` / `no_alloc` / BPF-safe / Apache-2.0

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs)
