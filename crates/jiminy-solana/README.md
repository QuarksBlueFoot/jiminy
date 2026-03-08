# jiminy-solana

Everything your pinocchio program needs to talk to Solana. Token/mint readers,
Token-2022 extension screening, CPI guards, Ed25519 verification, Merkle proofs,
Pyth oracles, authority rotation, TWAP, compute guards. All the platform stuff
that doesn't belong in core.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit. Depends on `jiminy-core` for validation, math, and account IO.

## Install

```toml
[dependencies]
jiminy-solana = "0.11"
```

## What's inside

| Module | What it does |
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

## Quick start

```rust,ignore
use jiminy_solana::prelude::*;

// Read token account balance
let balance = token_amount(token_account)?;

// Verify a Merkle proof
let valid = crypto::merkle::verify_proof(&proof, &root, &leaf);
```

## Features

| Feature | Default | Description |
|---|---|---|
| `programs` | yes | Exposes upgrade-authority helpers that reference program addresses |

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0

## License

Apache-2.0
