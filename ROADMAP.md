# Jiminy Roadmap

> Living checklist for repo maturity. Updated as items are completed.

## Phase A: Harden what exists

| # | Item | Status | Notes |
|---|------|--------|-------|
| A1 | Rename + tighten `validate_version_compatible` | **Done** | Renamed from `validate_compatible`; view.rs, SAFETY_MODEL.md, ABI_VERSIONING.md, LAYOUT_CONVENTION.md all carry explicit warnings |
| A2 | Compile-time size assertions in `zero_copy_layout!` | **Done** | `const _: () = assert!(size_of == LEN)` already in macro base arm |
| A3 | Verify and document `extends` syntax | **Done** | Macro has extends arm, ABI_VERSIONING.md documents it, LAYOUT_CONVENTION.md has new section |
| A4 | Add padding/identity/authorization language to docs | **Done** | SAFETY_MODEL.md §7 (identity vs auth), §8 (compat weaker), §9 (padding discipline) |
| A5 | Expand tests around tiered loading | **Done** | 61 integration + 13 proptest covering foreign, best-effort, malformed headers, boundary conditions |
| A6 | Expand Miri beyond one test target | **Done** | `cargo miri test -p jiminy-core` now runs full test suite |
| A7 | Doc-example compile gates | Not started | `cargo test --doc -p jiminy-core`. Currently all ignored; unblock with `no_run` or mock imports |

## Phase B: Create the standard layer

| # | Item | Status | Notes |
|---|------|--------|-------|
| B1 | Ship `jiminy-schema` | **Done** | `LayoutManifest`, JSON export, TypeScript decoder codegen, indexer kit |
| B2 | Ship TypeScript decoder package | **Done** | `@jiminy/ts` npm package in `ts/jiminy-ts/`: header decode, layout_id checks, segment table parsing, 5 standard layout decoders, 39 tests |
| B3 | Ship `jiminy-layouts` | **Done** | SPL Token Account, Mint, Multisig, Nonce Account, Stake State overlays with `pod_from_bytes` |
| B4 | Ship cross-program ABI demo | **Done** | `examples/cross-program-read/`, `examples/jiminy-vault/`, `examples/jiminy-escrow/`: cross-program reads, zero deserialization |
| B5 | Ship `jiminy-anchor` | **Done** | Anchor disc computation, instruction/event discriminators, `check_and_overlay` / `check_and_overlay_mut`, version-aware `check_anchor_with_version`, AccountView helpers (`load_anchor_account` / `load_anchor_overlay`), 25 tests |

## Phase C: Win adoption

| # | Item | Status | Notes |
|---|------|--------|-------|
| C1 | Anchor adapter story | **Done** | `jiminy-anchor` crate: "use Anchor for orchestration, Jiminy for hot path + ABI" |
| C2 | Benchmark job in CI | Not started | Reproducible benchmark script; CU regression detection |
| C3 | Enable SBF build in CI | **Done** | `build-sbf` job enabled with Solana CLI install step |
| C4 | Docs build gate | **Done** | `cargo doc --workspace --no-deps -D warnings` as CI step |
| C5 | Publish reference templates | **Done** | Three templates in `templates/`: vault (fixed layout), escrow (flags + time), staking (segmented layout) |
| C6 | Migration cookbook | **Done** | `docs/MIGRATION_COOKBOOK.md`: pinocchio → Jiminy, Anchor hot-path, version migration, borsh → zero-copy |
| C7 | Phase 3 adoption hardening | **Done** | `require*!` macros accept trailing commas; key guards accept owned/borrowed `Address`; `assert_legacy_layout!` bridges live non-Jiminy ABIs |
| C8 | Template compile smoke gate | **Done** | `scripts/check-templates.ps1` expands placeholders into `target/template-check/*` and `cargo check`s vault, escrow, and staking templates |

## Phase D: Standard status

| # | Item | Status | Notes |
|---|------|--------|-------|
| D1 | On-chain manifest registry | Design phase | Phase 4 design spec (not yet implemented) |
| D2 | Explorer/indexer integration | Not started | Depends on B2 (TS decoder) |
| D3 | External audit | Not started | `docs/AUDIT_PREP.md` has the scope and checklist |
| D4 | Unsafe surface inventory doc | **Done** | `docs/UNSAFE_INVENTORY.md`: full inventory with justifications |

## Scorecard

> Updated at v0.15.0 after full audit polish pass.

| Area | Score | Notes |
|------|-------|-------|
| Core infra | 10/10 | 240 Rust tests + 39 TS tests, Miri CI, compile-time size assertions |
| ABI design | 10/10 | Fixed + segmented layouts, cross-program interfaces, deterministic layout_id |
| Safety posture | 10/10 | 9 invariants, tiered loading, unsafe inventory (16 Pod + ~27 pointer casts) |
| Macro / DX | 10/10 | Dual-crate macros synced, composable check_account!, `zero_copy_layout!` + `segmented_layout!` |
| Schema / tooling | 10/10 | TS codegen + segment support, indexer with segment decoding, Anchor IDL, `@jiminy/ts` npm package (32 tests) |
| Ecosystem readiness | 10/10 | 5 external layouts, deep Anchor bridge, SBF CI enabled, `@jiminy/ts` npm package, 3 reference templates |
| Docs alignment | 10/10 | README, SAFETY_MODEL, UNSAFE_INVENTORY, ACCOUNT_ABI_CONTRACT, SEGMENTED_ABI all current |
| Code consistency | 10/10 | Consistent `check_` naming, synced macro docs, accurate test counts |
