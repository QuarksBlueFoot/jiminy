# jiminy-layouts

Standard zero-copy account layouts for well-known Solana programs. `#[repr(C)]` structs you can overlay directly onto account bytes. No deserialization, no allocation.

`#![no_std]` / `no_alloc` / BPF-safe

```toml
[dependencies]
jiminy-layouts = "0.15"
```

## Layouts

| Struct | Program | Size |
|--------|---------|------|
| `SplTokenAccount` | SPL Token | 165 bytes |
| `SplMint` | SPL Token | 82 bytes |
| `SplMultisig` | SPL Token | 355 bytes |
| `NonceAccount` | System program | 80 bytes |
| `StakeState` | Stake program | 200 bytes |

## Example

```rust,ignore
use jiminy_layouts::SplTokenAccount;
use jiminy_core::account::{pod_from_bytes, FixedLayout};

let data: &[u8] = &account.data;
let token = pod_from_bytes::<SplTokenAccount>(data)?;
let owner = token.owner;
let amount = u64::from_le_bytes(token.amount);
```

## Important

These are **external** (non-Jiminy) account layouts. They do NOT have the Jiminy 16-byte header. They are meant for reading accounts owned by other programs (SPL Token, System, Stake, etc.).

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

Donations: `solanadevdao.sol` (`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`)

## License

Apache-2.0
