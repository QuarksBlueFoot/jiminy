# jiminy-core

Account layout, zero-copy IO, validation, PDA, sysvar access, lifecycle,
math, time checks. Everything your pinocchio program needs before it touches
a token.

`#![no_std]` · `no_alloc` · BPF-safe

```toml
[dependencies]
jiminy-core = "0.11"
```

## Modules

| | |
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

## Usage

```rust,ignore
use jiminy_core::prelude::*;

let reader = AccountReader::new_checked(data, EXPECTED_DISC, EXPECTED_VER)?;
let owner = reader.pubkey_at(0)?;
let amount = reader.u64_at(32)?;
```

`programs` feature is on by default. `log` feature adds `sol_log` wrappers.

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

Donations: `solanadevdao.sol` (`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`)

## License

Apache-2.0
