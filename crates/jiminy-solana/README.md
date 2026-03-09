# jiminy-solana

Token/mint readers, Token-2022 screening, CPI guards, Ed25519, Merkle proofs, Pyth oracles, authority rotation, TWAP, compute guards. The Solana platform layer on top of `jiminy-core`.

`#![no_std]` / `no_alloc` / BPF-safe

```toml
[dependencies]
jiminy-solana = "0.13"
```

Pulls in `jiminy-core`, `pinocchio-token`, and `pinocchio-system` for you.

## What's in here

| | |
|---|---|
| `token` | SPL Token account readers, mint readers, Token-2022 extension screening |
| `cpi` | Safe CPI wrappers, reentrancy guards, return-data readers |
| `crypto` | Ed25519 precompile verification, Merkle proof verification |
| `authority` | Two-step authority rotation (propose + accept) |
| `balance` | Pre/post CPI balance-delta guards |
| `compute` | Compute-budget guards |
| `compose` | Transaction-composition guards (flash-loan detection) |
| `introspect` | Raw transaction introspection |
| `oracle` | Pyth V2 price-feed readers |
| `twap` | TWAP accumulators and deviation checks |
| `upgrade` | Program upgrade-authority verification *(feature-gated)* |

```rust,ignore
use jiminy_solana::prelude::*;

let balance = token_account_amount(token_account)?;
let valid = verify_merkle_proof(&proof, &root, &leaf);
```

---

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs) / Apache-2.0
