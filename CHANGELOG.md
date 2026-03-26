# Changelog

All notable changes to the Jiminy workspace are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.17.0] - 2025-06-XX

### Added

- **`segmented_interface!` macro**: Cross-program read-only views for
  segmented accounts. Generates `SEGMENTED_LAYOUT_ID`, segment table
  access, typed segment reads, and `load_foreign_segmented()` (Tier 2
  with min-size validation). Enables any program to read foreign
  segmented accounts without crate dependencies.
- **`validate_foreign_segmented()`**: Tier 2 loading function for
  segmented accounts using min-size checking instead of exact-size.
- **`min_size()` on `LayoutManifest`**: Returns the minimum account
  size for segmented layouts (`total_size + segments.len() * 12`).
  Emitted in `export_json()` output.
- **Schema `verify()` segment validation**: Detects zero element_size,
  duplicate segment names, and segment/field name collisions.
- **Schema integration test**: End-to-end lifecycle test covering
  manifest → verify → JSON → build bytes → verify account → decode.

### Changed

- **Capacity-aware `SegmentDescriptor` (breaking)**: Segment descriptors
  are now 12 bytes (was 8). Wire format:
  `[offset:u32][count:u16][capacity:u16][element_size:u16][flags:u16]`.
  The `capacity` field defines the reserved region per segment;
  `count` tracks live elements. Invariant: `count ≤ capacity`.
- **`SegmentDescriptor::new()` takes 4 arguments**: `(offset, count,
  capacity, element_size)` instead of 3.
- **`SegmentTableMut::init()` takes 3-tuples**: Specs are
  `(element_size, count, capacity)` instead of 2-tuples.
- **`init_segments()` sets `count == capacity`** (tight fit). Previously
  only set `count`; now both are equal, making pushed-after-init
  impossible. Use `init_segments_with_capacity()` for dynamic push.
- **`push` checks `count < capacity`** as its primary guard instead
  of computing overflow from adjacent segment offsets.
- **Validation uses reserved regions**: `validate()` checks
  `capacity × element_size` bounds and overlap, not just live data.

## [0.16.0] - 2025-03-24

### Added

- **`VerifiedAccount<T>` / `VerifiedAccountMut<T>`**: type-safe wrappers
  returned by `load()` / `load_mut()` / `load_foreign()`. Infallible
  `get()` / `get_mut()` access after construction, no raw bytes exposed.
- **`validate_account_mut()`**: mutable variant of Tier 1 validation,
  returns `RefMut` for write access.
- **`HEADER_FORMAT` constant**: tracks the header byte layout version.
- **`strict` feature**: production hardening mode. When enabled,
  `validate_version_compatible()` is compile-time disabled, forcing
  all loads through layout_id-verified tiers.
- **Compile-time alignment assertion**: `zero_copy_layout!` and
  `jiminy_interface!` now assert `align_of::<T>() <= 8` at compile time,
  preventing unsound layouts on Solana (which aligns program input to
  8 bytes). Raw `u128` fields are a compile error; use `LeU128`.
- **`jiminy_interface!` version parameter**: interfaces can now specify
  `version = N` to match foreign layouts at any version. Default remains
  `version = 1` for backward compatibility.
- **`init_segments_with_capacity()`**: new initializer for segmented
  layouts that spaces segment offsets by max capacity with counts
  starting at zero. Enables safe push/remove workflows.
- **Push overlap protection**: `segment_push` now checks the next
  segment's offset to prevent writes from overflowing into adjacent
  segments.

### Changed

- **Renamed** `load_best_effort()` to `load_unverified_overlay()` to
  communicate that no ABI guarantees are provided.
- **Exact size enforcement**: Tiers 1 and 2 now require
  `data.len() == expected_size` (was `<`). Prevents hidden trailing
  data attacks.
- **Alignment checks on all targets**: `pod_from_bytes` /
  `pod_from_bytes_mut` always check alignment, not just on native.
- **`load_mut()` no longer aliases**: backed by `RefMut` instead of
  casting from `Ref`. Eliminates UB from mutable aliasing.
- **Tier numbering**: `load_unchecked` is Tier 4, `load_unverified_overlay`
  is Tier 5 (was inconsistently Tier 3/4 in some docs).
- **Doc consistency**: all tier tables, doc comments, and safety model
  updated to reflect exact size checks and new API names.

## [0.15.0] - 2025-01-XX

### Added

- **Le\* types**: `LeU16`, `LeU32`, `LeU64`, `LeU128`, `LeI16`, `LeI32`,
  `LeI64`, `LeI128` for safe, alignment-1 little-endian field access.
- **FieldRef / FieldMut**: zero-copy typed references for individual
  fields without borrowing the entire struct.
- **`split_fields`**: macro-generated borrow-splitting that returns
  disjoint mutable references to individual fields.
- **Segmented ABI**: `segmented_layout!` macro for accounts with
  multiple variable-length dynamic arrays (up to `MAX_SEGMENTS = 8`).
  Includes `SegmentTable`, `SegmentSlice`, `push`, `swap_remove`,
  named segment accessors, and full bounds-checking.
- **`LayoutManifest`**: structured account schema description with
  `hash_input()`, `export_json()`, `verify()`, `verify_account()`,
  and `verify_hash()`. Manifest format version: `manifest-v1`.
- **`MANIFEST_VERSION` const**: frozen manifest format identifier.
- **Const field offsets**: `zero_copy_layout!` generates compile-time
  `OFFSET_<FIELD>` constants for every field.
- **TypeScript codegen**: `ts_decoder()` generates TS decoder
  functions from `LayoutManifest` (library API in jiminy-schema).
- **Anchor IDL fragments**: `to_anchor_idl_type()` for Anchor
  compatibility tooling.
- **Cross-program foreign read**: `load_foreign()` (Tier 2) validates
  owner + layout_id for safe cross-program account reads.
- **Unverified overlay loading**: `load_unverified_overlay()` (Tier 5) for indexers
  and tooling that read accounts without guaranteed headers.
- **Token-2022 screening**: `check_safe_token_2022_mint()` and
  extension safety checks in `jiminy-solana`.
- **Fuzz targets**: `fuzz_header`, `fuzz_overlay`, `fuzz_segment_table`,
  `fuzz_zero_copy_slice`, `fuzz_unverified_overlay` in `jiminy-core/fuzz/`.

### Documentation

- **LAYOUT_CONVENTION.md**: const offsets, `split_fields`, Le* types.
- **SEGMENTED_ABI.md**: complete design spec with frozen design
  decisions (max segments = 8, no auto-realloc, swap_remove semantics).
- **SAFETY_MODEL.md**: 10 invariants including layout_id collision
  resistance analysis.
- **LAYOUT_ID_COLLISION_ANALYSIS.md**: formal birthday paradox analysis,
  why 8-byte layout_id is sufficient, higher-assurance escape hatches.
- **ACCOUNT_ABI_CONTRACT.md**: cross-program ABI contract with failure
  mode table.
- **HOT_PATH_COOKBOOK.md**: 12 performance recipes.
- **MIGRATION_COOKBOOK.md**: 5 migration recipes including
  fixed-to-segmented migration.
- **ANCHOR_COMPARISON.md**: feature parity table and Jiminy advantages.
- **AUDIT_PREP.md**: scope, invariants, test counts, unsafe inventory.

### Changed

- Segmented ABI "Open Questions" section renamed to "Design Decisions"
  and locked. Max segments documented as 8 (matching `MAX_SEGMENTS`
  const). Swap-remove zeroing behavior specified. Push-when-full error
  behavior specified (`AccountDataTooSmall`).

## [0.14.0] - Prior release

See git history for earlier changes.
