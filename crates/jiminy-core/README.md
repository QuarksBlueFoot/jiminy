# jiminy-core

The foundation. Account layout, zero-copy IO, validation, PDA utilities, sysvar
access, lifecycle helpers, math, time checks. Everything your pinocchio program
needs before it touches a token.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit.

## Install

```toml
[dependencies]
jiminy-core = "0.11"
```

## What's inside

| Module | What it does |
|---|---|
| `account` | 8-byte header, `AccountReader`, `AccountWriter`, `SliceCursor`, lifecycle (close/realloc), pod, list, bitflags |
| `check` | Owner / signer / key assertions, PDA derivation & verification |
| `instruction` | Transaction introspection via the Instructions sysvar |
| `math` | Checked arithmetic, BPS helpers, scaling with u128 intermediates |
| `sysvar` | Clock & Rent sysvar readers |
| `state` | State-machine transition validation |
| `time` | Deadline, cooldown, slot-staleness checks |
| `event` | Zero-alloc event emission via `sol_log_data` |
| `programs` | Well-known program IDs *(feature-gated)* |

## Quick start

```rust,ignore
use jiminy_core::prelude::*;

// Read a validated account
let reader = AccountReader::new_checked(data, EXPECTED_DISC, EXPECTED_VER)?;
let owner = reader.pubkey_at(0)?;
let amount = reader.u64_at(32)?;
```

## Features

| Feature | Default | Description |
|---|---|---|
| `programs` | yes | Exposes well-known Solana program addresses |
| `log` | no | Structured `sol_log` wrappers |

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0
