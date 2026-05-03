# jiminy-schema

Layout Manifest v1 for Jiminy account schemas. Structured descriptions of account layouts for cross-language tooling, TypeScript decoder generation, indexer integration, and schema validation.

`#![no_std]` compatible (std/codegen/indexer features gated)

```toml
[dependencies]
jiminy-schema = "0.17"
```

## Why This Matters

Jiminy's account ABI is deterministic: every `zero_copy_layout!` struct has
a `layout_id`, field offsets, and canonical types defined at compile time.
`jiminy-schema` is the bridge that carries that information out of Rust and
into the rest of your stack. Without it, off-chain code has to guess at
account structure.

## How This Connects

```text
zero_copy_layout!            LayoutManifest            export_json()
     Rust struct  ──▶  structured description  ──▶  JSON manifest
                                                        │
                                              ┌─────────┼──────────┐
                                              ▼         ▼          ▼
                                        TypeScript   Indexer   Explorer
                                        decoders    matching   display
```

1. `zero_copy_layout!` defines your account struct and computes `LAYOUT_ID`.
2. You build a `LayoutManifest` describing the same struct (name, fields, types, sizes).
3. `export_json()` emits a JSON manifest. No serde dependency.
4. `@jiminy/ts`, indexers, and explorers consume the manifest to decode accounts.

## What's in here

| | |
|---|---|
| `LayoutManifest` | Describes one account type: name, version, discriminator, layout_id, field list |
| `FieldDescriptor` | Per-field metadata: name, canonical type, size |
| `CanonicalType` | Language-independent type identifiers (`U8`, `U64`, `Pubkey`, `Header`, etc.) |
| `codegen` | TypeScript decoder generation *(feature: `codegen`)* |
| `indexer` | Account matching and decoding for off-chain indexers *(feature: `std`)* |

## Example

```rust,ignore
use jiminy_schema::*;

let manifest = LayoutManifest {
    name: "Vault",
    version: 1,
    discriminator: 1,
    layout_id: [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89],
    fields: &[
        FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
        FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
        FieldDescriptor { name: "authority", canonical_type: CanonicalType::Pubkey, size: 32 },
    ],
};

assert_eq!(manifest.total_size(), 56);
assert_eq!(manifest.field_offset("balance"), Some(16));
```

## New in 0.17

- `LayoutManifest::min_size()` reports the fixed account size plus segment
    table bytes for segmented layouts.
- Schema verification rejects invalid segment metadata, duplicate segment
    names, and field/segment name collisions.

## New in 0.16

- `verify_account()` now enforces exact size matching.
- Documentation updates for cross-language tooling.

## New in 0.15

- `export_json()`: JSON manifest output for TypeScript decoders and indexers. No serde dependency.
- `verify()`: structural validation (header check, zero-size detection, duplicate field names).
- `anchor_idl_json()`: Anchor IDL v0.1.0 account fragment generation for explorer/wallet integration.

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

Donations: `solanadevdao.sol` (`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`)

## License

Apache-2.0
