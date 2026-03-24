# Why Jiminy

Every Solana program invents its own way to lay out account bytes,
validate ownership, version schemas, and read foreign accounts. This
ad-hoc duplication is the root cause of an entire class of on-chain bugs
 - corrupted cross-program reads, silent ABI drift, missing validation
checks, overflows in DeFi math. All preventable. All still happening.

Jiminy is the standard that makes this class of bug structurally
impossible.

## The Problem

### 1. No standard header

Without a shared header convention, every program picks a different
discriminator scheme, flags layout, and versioning strategy. Cross-program
reads require intimate knowledge of the target program's internal
serialization - knowledge that breaks silently when the target upgrades.

### 2. Foreign accounts are dangerous

Reading an account owned by another program is common (oracles, AMMs,
lending markets). Without a standard layout_id or version byte, the
reader has no way to verify that the data it's interpreting matches the
schema it expects. A layout change in the source program can cause the
reader to misinterpret bytes, potentially leading to fund loss.

### 3. Serialization overhead

Borsh deserialization copies every byte. For large accounts or hot paths
(AMM swaps, oracle reads), this overhead is measurable in compute units.
Zero-copy overlays eliminate it, but without a standard overlay
convention, each program rolls its own unsafe code.

### 4. Version drift

Programs evolve. Fields get added. Without a versioned header, there is
no cheap way to detect whether an account was created by v1 or v2 of a
program. Developers resort to heuristics (account size, magic bytes) that
are fragile and error-prone.

## Jiminy's Answer

### Standardized 16-byte header

Every Jiminy account starts with the same header:

```
[disc:u8][version:u8][flags:u16][layout_id:[u8;8]][reserved:[u8;4]]
```

- **disc**: cheap type tag (256 types per program).
- **version**: schema version, bumped on any layout change.
- **layout_id**: deterministic ABI fingerprint. Any reader can verify
  the layout without depending on the source crate.

### Deterministic layout_id

```
sha256("jiminy:v1:Vault:1:header:header:16,balance:u64:8,authority:pubkey:32,")[..8]
```

Rename a field, change a type, reorder - the hash changes. Programs get
a compile-time constant they can compare at runtime in a single 8-byte
memcmp.

### Zero-copy overlays

`zero_copy_layout!` generates `#[repr(C)]` structs with typed field
access. No deserialization, no copies. Read `vault.balance` directly from
borrowed account data.

### Tiered loading with safety defaults

The safe path (`load`) validates everything. Skipping validation requires
`unsafe`. This inverts the usual Solana pattern where unsafe raw access
is the default and validation is opt-in.

### Cross-program interop

Any program can read any Jiminy account:

```rust
// Program B reads Program A's Vault account.
validate_foreign(account, &PROGRAM_A_ID, &Vault::LAYOUT_ID, Vault::LEN)?;
let data = account.try_borrow()?;
let vault = Vault::overlay(&data)?;
let balance = vault.balance;
```

No dependency on Program A's crate. The layout_id guarantees the bytes
match the expected schema.

### No proc macros, no alloc, no std

Jiminy uses only `macro_rules!` and runs in `no_std` + `no_alloc`
environments. Two runtime dependencies: pinocchio and
sha2-const-stable. That's it. No framework opinions, no allocator,
no proc-macro compilation tax. Your build stays fast because there's
nothing to slow it down.

## What Jiminy Does Not Do

- **Full IDL generation.** `jiminy-schema` exports a `LayoutManifest`
  for TypeScript decoders and indexers, but Jiminy does not generate
  complete Anchor-style IDL files or client SDKs. The schema is a
  building block, not a framework.
- **Anchor replacement.** Jiminy and Anchor solve different problems.
  Anchor is a full framework with client codegen. Jiminy is a focused
  account ABI standard and DeFi toolkit that works with any Solana
  framework, or none at all. Use both together via `jiminy-anchor`.
