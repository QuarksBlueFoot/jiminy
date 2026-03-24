# Safety Model

Jiminy enforces safety through mechanical guardrails -- not documentation,
not "best practices," not hoping developers read the README. If a safety
property matters, the code enforces it. This document describes every
invariant Jiminy maintains and exactly how each one is enforced.

## Trust Tiers

Every account load in Jiminy runs through one of five named trust tiers.
Higher tiers validate more; lower tiers trade safety for flexibility.

| Tier | Name | Method | Validation | Use When |
|------|------|--------|------------|----------|
| 1 | **Verified** | `load()` / `load_mut()` | owner + disc + version + layout_id + exact size | Loading your own program's accounts |
| 2 | **Foreign Verified** | `load_foreign()` | owner + layout_id + exact size | Reading another program's accounts (cross-program) |
| 3 | **Compatibility** | `validate_version_compatible()` | owner + disc + version + min size (no layout_id) | Version migration, explicitly weaker |
| 4 | **Unsafe** | `load_unchecked()` | none (`unsafe`) | Hot path — caller assumes all risk |
| 5 | **Unverified Overlay** | `load_unverified_overlay()` | header + layout_id if present, fallback to overlay | Indexers, explorers, diagnostic tooling |

## 1. Zero-Init Before Header Write

**Invariant:** All Jiminy accounts MUST be zero-initialized before the
header is written.

**Why:** Solana does not guarantee that newly created account data is
zeroed. Stale bytes from a previous account at the same address can be
misread as valid flags, layout_id fragments, or field values.

**Enforcement:**

- `init_account!` calls `zero_init()` automatically before
  `write_header()`. This is the recommended creation path.
- `zero_init()` fills the entire data slice with zeroes.
- Manual creation (without `init_account!`) requires the developer to
  call `zero_init()` explicitly.

## 2. Close Sentinel

**Invariant:** Closed accounts are marked so they cannot be reused.

**Enforcement:**

- `safe_close()` and `close_account!` transfer all lamports to the
  destination, then write a close sentinel to the first bytes of the
  account data. This prevents the account from passing header validation
  if the runtime recycles the address.

## 3. Unsafe Load Friction

**Invariant:** The safe loading path (`load`, `load_foreign`) is the
path of least resistance. Skipping validation requires `unsafe`.

**Enforcement:**

The tiered loading API in `zero_copy_layout!` generates:

| Tier | Name | Method | Safety | Validation |
|------|------|--------|--------|------------|
| 1 | Verified | `Layout::load(account, program_id)` | safe | owner + disc + version + layout_id + exact size |
| 1m | Verified Mut | `Layout::load_mut(account, program_id)` | safe | owner + disc + version + layout_id + exact size |
| 2 | Foreign Verified | `Layout::load_foreign(account, owner)` | safe | owner + layout_id + exact size |
| 3 | Compatibility | `validate_version_compatible(...)` | safe | owner + disc + version + min size (no layout_id) |
| 4 | Unsafe | `Layout::load_unchecked(data)` | **unsafe** | none |
| 5 | Unverified Overlay | `Layout::load_unverified_overlay(data)` | safe | header + layout_id if present, fallback to overlay |

`load_unchecked` is `unsafe` by design. This creates syntactic friction
that pushes developers toward the validated `load()` path.

A separate utility, `validate_version_compatible(account, program_id, disc,
min_version, min_size)`, checks owner + discriminator + `version >=
min_version` + size **without** verifying `layout_id`. It is a
**migration/versioning helper**, not a full trust tier, and must not be
treated as equivalent to `load()` or `load_foreign()`. Because it skips
the layout fingerprint, it cannot guarantee that the on-chain bytes
match the expected struct layout -- it is not a proof of ABI identity.
Use it only for backward-compatible loading during version transitions
(see ABI_VERSIONING.md).

## 4. Pod Safety

**Invariant:** Overlay types accept all bit patterns as valid.

**Enforcement:**

- `Pod` is an `unsafe trait`. The `zero_copy_layout!` macro implements
  it for `#[repr(C)]` structs whose fields are all `Pod`.
- All primitive integer types and `Address` implement `Pod`.
- `AccountHeader` implements `Pod`.
- Overlay methods (`overlay`, `overlay_mut`, `pod_from_bytes`) perform
  a size check before transmuting.

## 5. Deterministic Layout ID

**Invariant:** The layout_id changes if and only if the account schema
changes.

**Enforcement:**

- `LAYOUT_ID` is a `const` computed at compile time from the struct
  name, version, and ordered field descriptors.
- Renaming a field, changing its type, changing its size, or reordering
  fields all change the hash.
- The hash prefix `"jiminy:v1:"` is itself versioned, so the hashing
  scheme can evolve without collisions.

## 6. Header Validation

**Invariant:** Account data is never interpreted without first validating
the header against the expected discriminator, version, and layout_id.

**Enforcement:**

- `load_checked` / `load_checked_mut` call `check_header()` before
  overlaying.
- `check_header()` verifies disc, version, and layout_id in one call.
- The `check_account!` macro provides composable constraint checks
  (owner, writable, signer, disc, version, layout_id, size) that
  compile down to inline comparisons.

## 7. Identity vs. Authorization

**Rule:** `layout_id` proves ABI identity -- it proves an account's byte
layout matches a specific Rust struct. It does **not** prove the account
is trustworthy, authorized, or owned by a trusted program.

**Corollary:** An owner check is never optional when ownership matters.
`load_foreign` proves ABI compatibility with the foreign account's
layout. It does **not** prove the foreign program is honest, audited, or
safe. The caller must independently verify trust in the program that
owns the account.

## 8. Compatibility Validation Is Weaker Than ABI Validation

**Rule:** `validate_version_compatible` is a migration helper, not a proof of
ABI identity. It does not validate `layout_id` and must not be treated
as equivalent to `load()` or `load_foreign()`.

**Why it exists:** During version transitions, a program may need to
accept accounts at version N or N+1 without knowing the exact
`layout_id` of each. `validate_version_compatible` covers this case.

**Why it is weaker:** Without `layout_id` verification, there is no
mechanical proof that the on-chain bytes match the overlaid Rust struct.
The caller accepts full responsibility for byte compatibility.

## 9. Padding Discipline

**Rule:** Implicit compiler padding must never carry semantic meaning.
Any reserved space in an account layout must be declared explicitly as
a named field (e.g., `_reserved: [u8; N]`).

**Why:** `#[repr(C)]` guarantees field ordering but allows inter-field
padding for alignment. If a program stores data in padding bytes, a
future `rustc` version or target could silently break it. Explicitly
declared reserved fields are part of the ABI contract and survive
across compilers.

## 10. Layout ID Collision Resistance

**Invariant:** The 8-byte `layout_id` provides sufficient collision
resistance for all practical Solana deployments.

**Analysis:** The birthday bound for a 64-bit hash is $\approx 2^{32}$
(4.3 billion) distinct layouts. At realistic ecosystem scale
(millions of layouts), collision probability is below $10^{-8}$.
See `LAYOUT_ID_COLLISION_ANALYSIS.md` for the full probability table.

**Defense in depth:**

- **Owner check (second factor):** Every `load()` and `load_foreign()`
  call verifies account ownership independently of `layout_id`.
  A false-positive load requires both a hash collision AND a matching
  owner -- a combined probability that is negligible.
- **Off-chain full hash:** `LayoutManifest::hash_input()` exposes the
  full canonical string. CI pipelines and tooling can compute the
  full 32-byte SHA-256 for 256-bit collision resistance during
  deployment verification.
- **Build-time registry:** Programs with many layouts can maintain a
  compile-time map of `(layout_id, hash_input)` pairs and assert no
  duplicates via `const` assertions.

## Non-Goals

Jiminy does **not** provide:

- **Runtime schema reflection.** There is no on-chain schema registry.
  Layout information is encoded at compile time.
- **Automatic migration.** Version bumps require explicit migration
  logic. Jiminy gives you the version byte and layout_id to detect
  mismatches; the migration itself is your responsibility.
- **Proc macros.** All code generation is via `macro_rules!`. This keeps
  the dependency tree minimal and compile times fast.
