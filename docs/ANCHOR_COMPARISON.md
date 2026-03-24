# Anchor vs Jiminy

Anchor and Jiminy are not the same kind of tool. Anchor is a full
framework: accounts, CPI, IDL, client codegen, proc macros, the works.
Jiminy is a focused account ABI standard and DeFi toolkit. Different
trade-offs, different strengths.

This document lays out the differences so you can pick the right tool
for the job, or use both.

## At a Glance

| Feature | Anchor | Jiminy |
|---------|--------|--------|
| Scope | Full framework (accounts, CPI, IDL, client codegen) | Focused account ABI standard |
| Account discriminator | 8-byte `sha256("account:TypeName")` prefix | 1-byte `u8` disc + 8-byte `layout_id` fingerprint |
| Header overhead | 8 bytes | 16 bytes |
| Serialization | Borsh (deserialize/serialize on every access) | Zero-copy overlay (`#[repr(C)]`, no copies) |
| Code generation | Proc macros (`#[account]`, `#[derive(Accounts)]`) | `macro_rules!` only (`zero_copy_layout!`) |
| Dependencies | ~40+ transitive crates | pinocchio + sha2-const-stable |
| `no_std` / `no_alloc` | No (requires `solana-program`, allocator) | Yes |
| Cross-program reads | Requires depending on source crate or manual byte parsing | `load_foreign()`: verify `layout_id` without source crate |
| Schema versioning | Not built-in (manual discriminator management) | First-class: version byte + deterministic `layout_id` |
| Safety model | Safe by default, `unchecked` variants available | Tiered: `load` (safe) → `unsafe load_unchecked` (friction) |
| Binary size | Typically 100–400 KB | Typically 20–30 KB |
| Compute cost | Higher (Borsh deser + allocator overhead) | Near hand-written Pinocchio (~7–14 CU overhead) |

## Discriminator Comparison

**Anchor** hashes the type name into 8 bytes
(`sha256("account:TypeName")[..8]`). Simple, unique per name, but you
pay 8 bytes and get no versioning.

**Jiminy** separates type identity from schema identity:

- **disc** (1 byte): cheap type tag, 256 account types per program.
- **layout_id** (8 bytes): deterministic ABI fingerprint derived from
  struct name, version, field names, types, and sizes. Changes when the
  schema changes.

This gives Jiminy both fast type dispatch (1-byte compare) and strong
schema validation (8-byte fingerprint) in a 16-byte header.

## Serialization

This is where the performance gap lives.

Anchor deserializes account data with Borsh on every instruction.
It copies every byte into a heap-allocated struct, then serializes it back
on exit. For large accounts or hot paths (AMM swaps, oracle reads), this
is real, measurable CU overhead.

Jiminy overlays a `#[repr(C)]` struct directly onto borrowed account
bytes. No copies, no allocation, no serialization. `vault.balance` is
a direct memory access.

```rust
// Anchor: deserializes entire account
let vault = &mut ctx.accounts.vault;
vault.balance += amount;

// Jiminy: zero-copy overlay
let mut raw = vault_account.try_borrow_mut()?;
let v = Vault::load_checked_mut(&mut raw)?;
v.balance = checked_add(v.balance, amount)?;
```

## Cross-Program Reads

This is Jiminy's strongest differentiator.

In Anchor, reading another program's account means either:

1. Depending on the source program's crate (tight coupling), or
2. Manual byte parsing with hardcoded offsets (fragile).

Jiminy's deterministic `layout_id` enables verified foreign reads without
any crate dependency:

```rust
// Program B reads Program A's Vault. No dependency on Program A's crate.
let verified = Vault::load_foreign(account, &PROGRAM_A_ID)?;
let vault = verified.get();
let balance = vault.balance;
```

The `layout_id` check proves the on-chain bytes match the expected
struct layout. If Program A changes its schema, the `layout_id` changes
and the read fails safely instead of misinterpreting bytes.

## When to Use Which

**Use Anchor when:**

- You want batteries-included: IDL generation, client codegen,
  built-in testing scaffolding.
- Rapid prototyping matters more than binary size or CU cost.
- Cross-program account reads aren't in the picture.

**Use Jiminy when:**

- CU budget is real money and you're optimizing for it.
- Cross-program account interoperability is a requirement.
- You want explicit schema versioning with upgrade safety.
- You're building on pinocchio and want a standardized data layer
  without buying into a full framework.
- You need `no_std` / `no_alloc` - embedded, WASM, or tests that
  don't pull in the entire Solana runtime.
- You need variable-length accounts with multiple dynamic arrays
  (`segmented_layout!`).
- You need alignment-safe types that pass Miri and work on all targets
  (Le* types instead of raw `&u64` pointer casts).

**Use both** when you want Anchor for orchestration and Jiminy for
the hot path. That's not a compromise. It's the architecture.

## Zero-Copy Feature Parity (v0.15)

| Feature | Anchor | Jiminy | Winner |
|---------|--------|--------|--------|
| Zero-copy overlay | `AccountLoader<T>` + `RefCell` | `overlay()`: direct pointer cast | Jiminy (no alloc, ~5 CU) |
| Discriminator | 8-byte `sha256("account:Name")` | 1-byte disc + 8-byte `layout_id` ABI fingerprint | Jiminy (stronger) |
| Schema versioning | Manual | `version` byte + `extends` + compile-time assertions | Jiminy |
| Cross-program reads | Requires source crate | `load_foreign` with `layout_id` proof | Jiminy |
| Variable-length data | `ZeroCopyVec` (single array) | `segmented_layout!`: N segments + push/swap_remove | Jiminy |
| Alignment safety | UB under strict alignment | Le* types: alignment 1, all targets | Jiminy |
| Borrow splitting | Not supported | `split_fields()` → `FieldRef`/`FieldMut` tuples | Jiminy |
| Close safety | No revival sentinel | Sentinel + `check_not_revived()` | Jiminy |
| Instruction IDL | Full IDL with args + accounts | Account layouts only (`jiminy-schema`) | Anchor |
| Declarative validation | `#[derive(Accounts)]` constraints | Explicit `check_*()` calls + `check_account!` | Anchor (ergonomics) |
| Event IDL | `#[event]` → IDL-indexed | `emit!`: zero-alloc, manual indexer config | Anchor (self-describing) |
| Client codegen | `@coral-xyz/anchor` TS SDK | `@jiminy/ts` + `ts_decoder()` codegen | Anchor (ecosystem) |

### What Jiminy has that Anchor does not

- **Le* alignment-safe types**: `LeU64`, `LeU128`, etc. Anchor's
  zero-copy uses `&u64` on `#[repr(C)]` structs, which is technically
  UB for unaligned access (SBF/SVM is alignment-1). Jiminy's Le* types
  are `#[repr(transparent)]` over `[u8; N]` - sound everywhere.

- **Borrow splitting**: `split_fields_mut()` gives you independent
  `FieldMut` handles to non-overlapping fields. Anchor's `RefMut<T>`
  gives you a single mutable reference to the whole struct.

- **Multi-segment dynamic accounts**: `segmented_layout!` supports
  N independent variable-length arrays in one account (order books,
  staking pools). Anchor's `ZeroCopyVec` is a single array.

- **Close revival protection**: `safe_close_with_sentinel` writes
  `[0xFF; 8]` to prevent account revival attacks. Anchor's `close`
  zeroes lamports but doesn't write a sentinel.

- **Compile-time PDA derivation**: `derive_address_const` computes
  PDAs at compile time for static seeds. Anchor always calls
  `find_program_address` at runtime.

- **Deterministic ABI fingerprint**: `LAYOUT_ID` is a SHA-256 hash
  of the full schema. If any field is renamed, retyped, resized, or
  reordered, the hash changes. Anchor's discriminator only tracks the
  type name. Silent schema changes go undetected.

## They're Not Mutually Exclusive

Jiminy is an account ABI standard, not a framework. The `jiminy-anchor`
crate bridges both worlds today:

- `anchor_disc("TypeName")`: compute Anchor's 8-byte discriminator at
  compile time.
- `check_anchor_disc(data, &disc)`: validate an Anchor account's
  discriminator on raw bytes.
- `anchor_body(data)`: extract the Borsh payload after the 8-byte
  discriminator.

This lets a Jiminy/pinocchio program read Anchor accounts without
depending on `anchor-lang`, and lets teams keep Anchor for orchestration
while moving performance-critical instructions to Jiminy. See
[MIGRATION_COOKBOOK.md](MIGRATION_COOKBOOK.md) for a step-by-step guide.
