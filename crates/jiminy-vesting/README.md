# jiminy-vesting

Linear + cliff vesting, stepped schedules, periodic unlock, claimable amount calculation.

```toml
jiminy-vesting = "0.16"
```

```rust,ignore
use jiminy_vesting::*;

let vested = vested_amount(total, start, cliff, end, now);
check_cliff_reached(cliff, now)?;
let claim = claimable(vested, already_claimed);
```

`vested_amount` is defensive against caller-provided timestamps: any schedule
where `start > cliff`, `cliff > end`, or `now < start` returns `0` rather than
wrapping `(now - start)` into a huge `u128` and silently releasing `total`.
Callers don't need a separate sanity check on the schedule shape.

Also has `unlocked_at_step` for discrete step schedules and `elapsed_steps` for counting complete periods.

`#![no_std]` / `no_alloc` / BPF-safe / Apache-2.0

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs)
