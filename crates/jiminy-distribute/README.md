# jiminy-distribute

Split a token amount across N recipients by weight. Remainder goes to the last recipient so the sum always equals the input. Also does basis-point fee extraction.

```toml
jiminy-distribute = "0.16"
```

Two functions:

```rust,ignore
use jiminy_distribute::*;

// Weighted split, dust goes to last recipient
let mut amounts = [0u64; 4];
proportional_split(total_amount, &weights, &mut amounts)?;

// Fee extraction
let (net, fee) = extract_fee(gross_amount, fee_bps)?;
```

`#![no_std]` / `no_alloc` / BPF-safe / Apache-2.0

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs)
