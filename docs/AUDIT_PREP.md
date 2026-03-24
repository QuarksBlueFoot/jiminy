# Audit Preparation Guide

> **Status:** v1.0 readiness checklist

## Scope

The external audit should cover the following crates in priority order:

| Crate | Priority | Reason |
|-------|----------|--------|
| `jiminy-core` | **Critical** | Account header, Pod casting, zero-copy overlay, validation |
| `jiminy-solana` | **High** | CPI wrappers, token readers, reentrancy guards |
| `jiminy-anchor` | Medium | Anchor discriminator computation, interop |
| `jiminy-layouts` | Medium | External account struct sizes, Pod safety |
| `jiminy-schema` | Low | Off-chain tooling, no on-chain safety impact |

## Critical Invariants to Verify

### 1. Pod Safety (jiminy-core/src/account/pod.rs)

- `pod_from_bytes` must never produce a reference to misaligned memory
- `pod_from_bytes_mut` must never return a mutable reference to misaligned memory
- All types implementing `Pod` must be `#[repr(C)]`, `Copy`, and valid for all bit patterns
- `zero_copy_layout!` macro must generate correct `unsafe impl Pod` only for qualifying types

### 2. Header Integrity (jiminy-core/src/account/header.rs)

- `HEADER_LEN = 16` is correct and matches struct size
- `write_header()` writes all 16 bytes atomically
- `check_header()` validates disc + version + layout_id correctly
- No partial header writes are possible through the public API

### 3. Zero-Init Invariant (jiminy-core/src/account/cursor.rs)

- `zero_init()` zeroes all account data bytes, not just the header
- `init_account!` always calls `zero_init()` before `write_header()`
- No code path exists that writes a header without prior zeroing

### 4. Layout ID Determinism (jiminy-core/src/account/overlay.rs)

- `LAYOUT_ID` is computed from `sha256("jiminy:v1:<Name>:<version>:<fields>")[..8]`
- Field order is declaration order (reordering fields changes the hash)
- Canonical type mapping is bijective (each Rust type maps to exactly one canonical string)
- The hash input format matches `LAYOUT_CONVENTION.md` specification

### 5. Tiered Loading Trust Levels (jiminy-core/src/account/view.rs)

- `load_checked` (Tier 1): verifies owner + disc + version + layout_id + size
- `load_foreign` (Tier 2): verifies owner + layout_id (NOT disc/version)
- `load_unchecked` (Tier 3): is `unsafe` - requires caller to justify safety
- `load_best_effort` (Tier 4): never panics, gracefully degrades
- No tier provides more access than documented

### 6. CPI Safety (jiminy-solana/src/cpi/)

- `check_no_cpi_caller()` correctly detects CPI context
- Safe transfer wrappers validate token program ID before invoke
- No CPI wrapper allows arbitrary program invocation

### 7. Compile-Time Assertions (jiminy-core/src/account/overlay.rs)

- `const _: () = assert!(size_of::<T>() == LEN)` prevents size mismatch
- `extends` arm verifies DISC equality, LEN growth, VERSION increment

## Test Coverage

### Existing Test Suite

| Test File | Tests | Coverage |
|-----------|-------|----------|
| `account_abi.rs` | 107 | Header v2, overlay, loading, extends, validation, foreign, malformed, edge cases, const offsets, split_fields, Le* types |
| `proptest_abi.rs` | 13 | Fuzz: roundtrip, rejection, mutation, best-effort, compatibility |
| `segment_tests.rs` | 57 | Segmented layouts, push, swap_remove, named accessors, validation, init, compute_size |
| `jiminy-schema` | 33 | Manifest, codegen, indexer, FieldRef/FieldMut, verify_account, verify_hash |
| `jiminy-anchor` | 18 | Discriminator, check, body extraction |
| `jiminy-layouts` | 25 | Struct size assertions, Le* roundtrips |
| Other crates | 7 | Staking, vesting, vault examples |
| **Total** | **260** | **All passing, 0 failures** |

### Recommended Additional Testing

1. **Miri**: CI runs `cargo miri test -p jiminy-core` (full test suite) for undefined behavior detection
2. **Kani**: model-check `pod_from_bytes` for all possible inputs
3. **Manual review**: every `unsafe` block in the codebase (grep for `unsafe`)
4. **Fuzzing**: cargo-fuzz targets in `crates/jiminy-core/fuzz/`:
   - `fuzz_header`: header validation with arbitrary bytes
   - `fuzz_overlay`: `pod_from_bytes` / `pod_from_bytes_mut` with random slices
   - `fuzz_segment_table`: segment table parsing, descriptor reads, validation
   - `fuzz_zero_copy_slice`: `ZeroCopySlice` length-prefix parsing, element access, iteration
   - `fuzz_best_effort`: `load_best_effort` permissive loading with arbitrary data

   Run with: `cd crates/jiminy-core && cargo +nightly fuzz run fuzz_header`

## `unsafe` Inventory

Run this to find all unsafe blocks:

```sh
grep -rn "unsafe" crates/jiminy-core/src/ crates/jiminy-solana/src/ crates/jiminy-anchor/src/ crates/jiminy-layouts/src/
```

Each `unsafe` block should have a `// SAFETY:` comment explaining why
it is sound. Verify each justification.

### Known `unsafe` usage:

| File | Function | Justification |
|------|----------|---------------|
| pod.rs | `pod_from_bytes` | Pointer cast after size+alignment check |
| pod.rs | `pod_from_bytes_mut` | Mutable pointer cast after size+alignment check |
| pod.rs | `pod_read` | `read_unaligned` on checked-size slice |
| overlay.rs | `impl Pod for <Layout>` | Generated by macro for `#[repr(C)]` + `Copy` types |
| view.rs | `load_unchecked` | Explicitly unsafe - caller must verify |
| header.rs | pointer reads | Read LE values from checked-length slices |

## Documentation Checklist

| Document | Status | Auditor Notes |
|----------|--------|---------------|
| LAYOUT_CONVENTION.md | Complete | Header format, tiered loading, zero-init |
| ABI_VERSIONING.md | Complete | Append-only, extends, migration |
| SAFETY_MODEL.md | Complete | 10 invariants + padding discipline, collision resistance |
| ACCOUNT_ABI_CONTRACT.md | Complete | Cross-program read contract |
| LAYOUT_ID_COLLISION_ANALYSIS.md | Complete | Birthday paradox math, 8-byte sufficiency, escape hatches |
| SEGMENTED_ABI.md | Complete | Design spec with frozen decisions (v0.15.0) |
| WHY_JIMINY.md | Complete | Motivation, comparison |
| ANCHOR_COMPARISON.md | Complete | Feature comparison table |
| HOT_PATH_COOKBOOK.md | Complete | Performance recipes |
| MIGRATION_COOKBOOK.md | Complete | pinocchio/Anchor/borsh migration recipes |
| ON_CHAIN_MANIFEST.md | Deferred | Phase 4 design spec (not yet implemented) |

## Pre-Audit Checklist

- [ ] All tests pass: `cargo test --workspace`
- [ ] No clippy warnings: `cargo clippy --workspace`
- [ ] Miri clean: `cargo +nightly miri test -p jiminy-core`
- [ ] Every `unsafe` block has a `// SAFETY:` comment
- [ ] CHANGELOG.md updated with all changes since last release
- [ ] Version bumped to release candidate
- [ ] All documentation reviewed for accuracy
- [ ] No `TODO` or `FIXME` comments in auditable crates
- [ ] Dependencies pinned to exact versions
- [ ] `cargo audit` shows no known vulnerabilities
