# jiminy-distribute

Split a token amount across N recipients by weight. Largest-remainder method
distributes the integer-division dust so `sum(amounts) == total` exactly. Also
does basis-point + flat-fee extraction with `net + fee == amount` guaranteed.

```toml
jiminy-distribute = "0.17"
```

Two functions:

```rust,ignore
use jiminy_distribute::*;

// Weighted split, dust spread one unit at a time across the trailing slots
let weights = [50u64, 30, 20];
let mut amounts = [0u64; 3];
proportional_split(1_000_003, &weights, &mut amounts)?;
// amounts.iter().sum::<u64>() == 1_000_003

// Fee extraction: ceiling bps fee + optional flat fee, net + fee == amount.
// Returns InvalidArgument if fee_bps > 10_000 (>100%), and InsufficientFunds
// if (bps_fee + flat_fee) would exceed amount.
let (net, fee) = extract_fee(1_000_000, 30, 1_000)?; // 0.3% + 1000 flat
assert_eq!(net + fee, 1_000_000);
```

`#![no_std]` / `no_alloc` / BPF-safe / Apache-2.0

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs)
