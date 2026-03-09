# jiminy-core

Account layout, zero-copy IO, validation, PDA, sysvar access, lifecycle, math, time checks. Everything your pinocchio program needs before it touches a token.

`#![no_std]` / `no_alloc` / BPF-safe

```toml
[dependencies]
jiminy-core = "0.13"
```

## What's in here

| | |
|---|---|
| `account` | 8-byte header, `AccountReader`, `AccountWriter`, `SliceCursor`, lifecycle (close/realloc), pod, `zero_copy_layout!`, `ZeroCopySlice`, list, bitflags |
| `check` | Owner / signer / key checks, PDA derivation and verification |
| `instruction` | Transaction introspection via the Instructions sysvar |
| `math` | Checked arithmetic, BPS helpers, scaling with u128 intermediates |
| `sysvar` | Clock and Rent readers (syscall-based + account-based) |
| `state` | State-machine transition validation |
| `time` | Deadline, cooldown, slot-staleness checks |
| `event` | Zero-alloc event emission via `sol_log_data` |
| `programs` | Well-known program IDs *(feature-gated)* |

## New in 0.13

- `zero_copy_layout!` macro: declare `#[repr(C)]` structs that overlay directly onto account bytes. No proc macros.
- `ZeroCopySlice` / `ZeroCopySliceMut`: length-prefixed arrays in account data. Zero-copy iteration, indexing, mutation.
- `pod_read<T>()`: alignment-safe owned copy via `read_unaligned`. Works everywhere.
- `clock_timestamp()`, `clock_slot()`, `clock_epoch()`: syscall-based sysvar access. No account slot needed.
- `rent_lamports_per_byte_year()`: same, for Rent.

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
