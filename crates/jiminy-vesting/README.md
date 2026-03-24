# jiminy-vesting

Linear + cliff vesting, stepped schedules, periodic unlock, claimable amount calculation.

```toml
jiminy-vesting = "0.15"
```

```rust,ignore
use jiminy_vesting::*;

let vested = vested_amount(total, start, cliff, end, now);
check_cliff_reached(cliff, now)?;
let claim = claimable(vested, already_claimed);
```

Also has `unlocked_at_step` for discrete step schedules and `elapsed_steps` for counting complete periods.

`#![no_std]` / `no_alloc` / BPF-safe / Apache-2.0

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs)
