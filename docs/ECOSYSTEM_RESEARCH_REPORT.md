# Ecosystem Research Report: Zero-Copy Innovation Opportunities for Jiminy

> Research date: 2026-03-26
> Sources: Pinocchio v0.11 (febo/pinocchio), solana-zero-copy v1.0 (anza-xyz/solana-sdk), Quasar v0.0 (blueshift-gg/quasar), spl-list-view v0.1 (solana-program/libraries)

---

## Executive Summary

Jiminy is already the most comprehensive zero-copy ABI standard for Solana. No competing project matches its combination of deterministic `layout_id`, tiered trust model, cross-program interfaces, and segmented layouts. However, research reveals **five high-impact innovation opportunities** drawn from patterns in the ecosystem that Jiminy doesn't yet address:

1. **Lazy account loading** (from Pinocchio v0.11)
2. **Inline dynamic fields with offset caching** (from Quasar)
3. **Raw encoded CPI pass-through** (from Quasar)
4. **Segmented interface views** (novel, combining Jiminy + Quasar patterns)
5. **Account introspection trait** (novel, gap in entire ecosystem)

---

## 1. Pinocchio v0.11 -- Jiminy's Runtime Layer

### Sources Investigated
- `sdk/src/lib.rs` -- crate-level docs, feature flags, re-exports
- `sdk/src/entrypoint/lazy.rs` -- `InstructionContext`, `MaybeAccount`, lazy parsing
- `sdk/src/entrypoint/mod.rs` -- `process_entrypoint`, input buffer format
- `solana-sdk/account-view/src/lib.rs` -- `AccountView`, `RuntimeAccount`, borrow tracking

### Key Findings

#### 1a. Lazy Entrypoint (`lazy_program_entrypoint!`)

Pinocchio v0.11 promotes `InstructionContext` as a first-class lazy entrypoint. Instead of parsing all accounts upfront, accounts are parsed on-demand via `next_account()`:

```rust
pub fn process_instruction(mut context: InstructionContext) -> ProgramResult {
    let authority = context.next_account()?.assume_account();
    let vault = context.next_account()?.assume_account();
    // remaining accounts never parsed -- saves CU
}
```

**What Pinocchio does well:** Zero-cost for unused accounts. A program with 8 declared accounts but only 3 used saves ~5 × parsing-cost CU.

**Relevance to Jiminy:** Jiminy's `load()` / `load_foreign()` always operates on an already-parsed `AccountView`. But jiminy could provide a **lazy account validator** that wraps `InstructionContext` and provides typed, validated access incrementally:

```rust
// Innovation sketch: jiminy lazy loader
let mut ctx = jiminy_core::lazy::LazyAccounts::new(context);
let vault: VerifiedAccount<Vault> = ctx.next_verified(program_id)?;
let config: VerifiedAccount<Config> = ctx.next_verified(program_id)?;
// remaining accounts never touched
```

**Impact: MEDIUM.** Reduces CU for programs with many accounts where not all are used. Particularly valuable for instructions with optional trailing accounts.

#### 1b. AccountView Borrow Tracking

Pinocchio's `AccountView` uses a `borrow_state` byte (the reused duplicate marker) for RefCell-like borrow tracking. Key observations:

- `borrow_state = 255 (NOT_BORROWED)` -- no borrows
- `borrow_state = 0` -- mutably borrowed
- `borrow_state ∈ [2, 254]` -- immutable borrow count (decrements)
- `borrow_unchecked()` bypasses tracking entirely

**What Jiminy does:** `load()` calls `try_borrow()` (safe, tracked). `load_unchecked()` uses `borrow_unchecked()` (unsafe, untracked). Good layering.

**What could improve:** Jiminy's `load_mut()` should document that it prevents re-borrowing during the `VerifiedAccountMut` lifetime via pinocchio's borrow tracking, and that this is *the* mechanism preventing data races in CPI scenarios. Currently this is implicit.

#### 1c. Account Resize Feature

Pinocchio v0.11 has `account-resize` and `unsafe-account-resize` features. The `account-resize` feature stores original data length in the padding field for bounds checking. `unsafe-account-resize` skips validation.

**Relevance:** Jiminy's segmented layouts need `realloc` for capacity growth. Jiminy should detect and require the `account-resize` feature when segmented layouts use `push` near capacity, and provide a safe realloc path that preserves segment table integrity.

---

## 2. solana-zero-copy v1.0 -- Unaligned Primitives

### Sources Investigated
- `zero-copy/src/lib.rs` -- crate structure
- `zero-copy/src/unaligned.rs` -- `Bool`, `U16`, `U32`, `U64`, `I16`, `I64`, `U128`

### Key Findings

`solana-zero-copy` is **minimal by design**: only unaligned primitive wrappers. No dynamic arrays, no segments, no collections, no account patterns. Its value prop is "official Anza-blessed primitive types" with `bytemuck`, `borsh`, `serde`, and `wincode` feature gates.

**What they do well:**
- Ecosystem standardization -- used by `spl-list-view`, `spl-type-length-value`
- Feature coverage: bytemuck, borsh, serde, wincode derive support
- `impl_usize_conversion` -- clean `TryFrom<usize>` for all types (used by spl-list-view's `PodLength`)

**What Jiminy does better:**
- Jiminy's `Le*` types have arithmetic operators (`get()`, `set()`, `checked_add()` etc.)
- Jiminy's types have `Display`, `PartialOrd` impls
- Jiminy's types are integrated with `FixedLayout` and `Pod` traits, enabling `zero_copy_layout!` use
- Jiminy already has a bridge in `compat::szc` for bidirectional conversion

**What's missing:**
- `solana-zero-copy` has `wincode` (Solana's new serialization format) support. Jiminy's `Le*` types don't.
- Jiminy's `LeI32`, `LeI128` have no `solana-zero-copy` counterpart (SZC doesn't define them)

**Innovation opportunity:** None directly from solana-zero-copy itself. The crate is a strict subset of what Jiminy already provides.

**However:** The `PodLength` trait pattern from `spl-list-view` (which uses solana-zero-copy types) is relevant -- see Section 4 below.

---

## 3. Quasar -- Dynamic Account Framework

### Sources Investigated
- `lang/src/dynamic.rs` -- `String<P,N>`, `Vec<T,P,N>`, `RawEncoded`, `MAX_DYNAMIC_TAIL`
- `lang/src/traits.rs` -- `Owner`, `Discriminator`, `Space`, `AccountCheck`, `ZeroCopyDeref`
- `lang/src/context.rs` -- `Context`, `Ctx`, `CtxWithRemaining`
- `lang/src/accounts/interface.rs` -- `Interface<T>`, `ProgramInterface` trait
- `derive/src/account/mod.rs` -- field classification, `DynKind`, validation order
- `derive/src/account/dynamic.rs` -- codegen: ZC companion, offset caching, set_inner, realloc
- `derive/src/account/accessors.rs` -- O(1) field access via cached offsets, `_raw()` methods
- `pod/src/lib.rs` -- `PodU64`, `PodU32` etc. with wrapping arithmetic

### Key Findings

#### 3a. Inline Dynamic Fields with Offset Caching ★★★

Quasar's most innovative pattern. Dynamic accounts have:
- Fixed fields first → compiled into a `FooZc` companion struct (pointer-cast access)
- Dynamic fields after → `String<P,N>` / `Vec<T,P,N>` with inline length prefixes
- At parse time, a **single prefix-walk** produces `__off: [u32; N-1]` -- cached byte offsets
- All subsequent field access is O(1) via the cached offsets

```
Wire layout:
[disc][fixed_field_1][fixed_field_2][u32:len1][str_bytes_1][u16:count2][elem_bytes_2]
                                    ↑                      ↑
                                    __off[0]               __off[1]  (cached)
```

**Why this matters:** Jiminy's `segmented_layout!` uses a segment table (12 bytes × N descriptors) between the fixed prefix and data. Quasar's approach is **more space-efficient** for 1–3 dynamic fields because there's no segment table overhead -- the length prefix IS the metadata.

Trade-offs:
| | Jiminy Segments | Quasar Inline |
|---|---|---|
| **Space per dynamic field** | 12 bytes descriptor | 1–4 bytes prefix |
| **Cross-program readable** | Yes (segment table is self-describing) | No (offsets require walking) |
| **Capacity tracking** | Explicit capacity field | No capacity -- realloc on write |
| **Multiple segments** | Up to 8 | Unlimited (walk cost grows) |
| **Tooling decodability** | Segment table is parseable | Requires codec knowledge |

**Innovation opportunity (HIGH IMPACT):**

Jiminy should add a **lightweight inline dynamic field** pattern alongside `segmented_layout!` for the common case of 1–2 variable-length fields:

```rust
// Proposed: inline_dynamic_layout!
inline_dynamic_layout! {
    pub struct Profile, discriminator = 3, version = 1 {
        // Fixed region (zero-copy overlay)
        header:    AccountHeader = 16,
        authority: Address       = 32,
        score:     LeU64         = 8,
        // Dynamic region (prefix + data)
        name:      DynString<u16, 64>,    // u16 prefix, max 64 bytes
        tags:      DynVec<Tag, u8, 16>,   // u8 prefix, max 16 elems
    }
}
```

This generates:
- `ProfileFixed` -- `#[repr(C)]` overlay for the fixed portion
- `Profile<'a>` -- parsed view with cached offsets (like Quasar)
- `LAYOUT_ID` -- deterministic hash including dynamic field metadata
- `load()` -- validates header + walks prefixes once
- `name()`, `tags()` -- O(1) accessors via cached offsets

**Key differentiator from Quasar:** Include the dynamic field metadata (prefix sizes, max lengths, element types) in the `LAYOUT_ID` hash. This makes inline dynamic layouts **cross-program verifiable** -- something Quasar can't do.

#### 3b. RawEncoded CPI Pass-Through ★★

Quasar generates `_raw()` accessor methods that return `RawEncoded<'a, PREFIX_BYTES>` -- a zero-copy view of prefix + data bytes for CPI:

```rust
// Quasar pattern:
let raw_name = profile.name_raw();  // RawEncoded<'_, 2>
// Pass raw_name.bytes directly to CPI data -- no decode/re-encode
```

**Innovation for Jiminy:** For segmented accounts doing CPI, provide a raw segment view:

```rust
// Proposed: SegmentRaw
let raw_orders = order_book.segment_raw(0)?;  // &[u8] of prefix+data
// Forward to CPI without decode
```

**Impact: MEDIUM.** Saves CU in CPI-heavy programs that forward dynamic data.

#### 3c. ProgramInterface Trait ★

Quasar's `Interface<T>` and `ProgramInterface` trait allow a single account slot to accept accounts from multiple programs (e.g., Token OR Token-2022):

```rust
pub trait ProgramInterface {
    fn matches(address: &Address) -> bool;
}
```

**Comparison with Jiminy:** Jiminy's `jiminy_interface!` binds to a single `OWNER` address. There's no "multi-program interface" concept.

**Innovation opportunity (MEDIUM):**

```rust
// Proposed: multi-program interface
jiminy_interface! {
    pub struct TokenAccount for [TOKEN_PROGRAM, TOKEN_2022_PROGRAM] {
        // ...fields...
    }
}
```

Or a trait-based approach:
```rust
trait ProgramSet {
    fn contains(addr: &Address) -> bool;
}

// load_foreign checks against the program set instead of a single address
```

#### 3d. MIN_SPACE / MAX_SPACE Constants ★

Quasar computes compile-time `MIN_SPACE` (disc + fixed + prefixes) and `MAX_SPACE` (min + all dynamic maxes). Useful for:
- `create_account` CPI size calculation
- Rent estimation
- Off-chain tooling

**Innovation for Jiminy:** For `inline_dynamic_layout!`, generate:
```rust
impl Profile {
    const MIN_LEN: usize = 56 + 2 + 1;  // fixed + prefixes
    const MAX_LEN: usize = MIN_LEN + 64 + 16 * size_of::<Tag>();
}
```

---

## 4. spl-list-view -- Standard Collection Pattern

### Sources Investigated
- `list-view/src/lib.rs` -- crate structure
- `list-view/src/list_view.rs` -- `ListView<T, L>`, layout calculation, alignment padding
- `list-view/src/list_view_mut.rs` -- `ListViewMut`, push, remove

### Key Findings

`spl-list-view` is a zero-copy `Vec` over `&[u8]` with these design choices:
- Generic length prefix via `PodLength` trait (`U16`, `U32`, `U64`)
- Alignment padding between prefix and data (for native-aligned `Pod` types)
- **No capacity field** -- capacity derived from buffer size
- Shift-left `remove()` (not swap-remove)
- Uses `bytemuck` for type safety

**What they do well:**
- Clean `size_of(num_items)` calculation including padding
- `PodLength` trait is generic -- supports u16/u32/u64 prefixes
- Standard `push/remove` semantics

**What Jiminy does better:**
- Jiminy's `SegmentDescriptor` has explicit capacity tracking -- critical for deterministic rent calculation
- Jiminy's `ZeroCopySlice` is simpler (no alignment padding needed because all Jiminy types are align-1)
- Jiminy's segments support multiple arrays per account

**Gap in spl-list-view:**
- No capacity awareness -- buffer must be pre-sized perfectly
- No multi-array support -- single list per buffer
- Depends on `bytemuck` (not pinocchio-native)

**Innovation opportunity (LOW):** Jiminy's collection story is already stronger. Minor win: adopt the `PodLength` trait pattern for `ZeroCopySlice` to support u8/u16/u32 prefixes:

```rust
// Current: always u32 prefix
ZeroCopySlice::<Address>::from_bytes(data)?;

// Proposed: generic prefix
ZeroCopySlice::<Address, LeU16>::from_bytes(data)?;  // u16 prefix saves 2 bytes
```

---

## 5. Cross-Ecosystem Gap Analysis

### What Nobody Does Well (Opportunities for Jiminy to Lead)

#### 5a. Self-Describing Account Introspection ★★★

**The gap:** No project provides a universal way to answer "what IS this account?" from raw bytes without knowing the program. Pinocchio gives you raw bytes. Quasar checks a discriminator. Jiminy checks layout_id. But none provide account introspection.

**Innovation (HIGH IMPACT):**

```rust
/// Inspect a raw account and determine its type.
pub struct AccountIntrospection<'a> {
    pub header: Option<&'a AccountHeader>,
    pub layout_id: Option<[u8; 8]>,
    pub is_jiminy: bool,
    pub is_segmented: bool,
    pub segment_count: Option<u8>,
    pub data_len: usize,
}

impl AccountIntrospection<'_> {
    /// Best-effort inspection of raw account bytes.
    /// Does NOT validate -- for tooling/explorers only.
    pub fn inspect(data: &[u8]) -> Self { /* ... */ }
}
```

This enables:
- Explorers showing "this is a Jiminy Vault v2 with 3 segments"
- Indexers auto-detecting account types without program-specific decoders
- Migration tools validating old vs. new account formats

#### 5b. Segmented Layout Foreign Interface ★★★

**The gap:** Jiminy's `jiminy_interface!` only works for fixed-size layouts. There's no way to define a cross-program interface for a segmented account.

**Innovation (HIGH IMPACT):**

```rust
// Proposed: segmented_interface!
segmented_interface! {
    pub struct OrderBook for DEX_PROGRAM {
        // Fixed portion (overlay)
        header:     AccountHeader = 16,
        market:     Address       = 32,
        // Segments (read-only)
        bids:       SegmentView<Order>,   // maps to segment 0
        asks:       SegmentView<Order>,   // maps to segment 1
    }
}

// Usage:
let ob = OrderBook::load_foreign(account)?;
let bids = ob.bids()?;  // SegmentSlice<Order> -- zero-copy view
for bid in bids.iter() { /* ... */ }
```

The `LAYOUT_ID` for segmented interfaces includes segment metadata (element types, expected positions), so cross-program verification works.

#### 5c. Account View Projections ★★

**The gap:** When reading a foreign account, you often only need 2-3 fields. But `load_foreign` validates and overlays the entire struct. Quasar's `Deref` to the ZC companion also gives you everything.

**Innovation (MEDIUM):**

```rust
// Proposed: field projection interface
jiminy_projection! {
    pub struct VaultBalance for DEX_PROGRAM {
        // Only overlay the fields you need:
        header:    AccountHeader = 16,
        balance:   LeU64         = 8,
        // total expected size = 56 (full Vault size)
    }
    total_size = 56,
}

let proj = VaultBalance::load_projection(account)?;
let balance = proj.get().balance.get();
// Only header + balance are accessed; authority field never touched
```

This is more efficient for cross-program reads where you only need one or two fields.

---

## Innovation Rankings

### Tier 1: Build These (High Impact, Sound, Auditable)

| # | Innovation | Source Inspiration | Effort | Impact |
|---|---|---|---|---|
| 1 | **`inline_dynamic_layout!`** -- inline dynamic fields with offset caching and layout_id verification | Quasar's dynamic codegen | Large | ★★★★★ |
| 2 | **`segmented_interface!`** -- cross-program views for segmented accounts | Novel (gap in ecosystem) | Medium | ★★★★★ |
| 3 | **Account Introspection** -- `AccountIntrospection::inspect()` for tooling | Novel (gap in ecosystem) | Small | ★★★★ |
| 4 | **Multi-program interfaces** -- `jiminy_interface!` with program sets | Quasar's `ProgramInterface` | Small | ★★★★ |

### Tier 2: Strong Opportunities (Medium Impact)

| # | Innovation | Source Inspiration | Effort | Impact |
|---|---|---|---|---|
| 5 | **Raw segment pass-through** -- `SegmentRaw` for zero-copy CPI forwarding | Quasar's `RawEncoded` | Small | ★★★ |
| 6 | **Generic ZeroCopySlice prefix** -- `ZeroCopySlice<T, L>` with u8/u16/u32 prefix | spl-list-view's `PodLength` | Small | ★★★ |
| 7 | **Lazy account loader** -- `LazyAccounts` wrapping pinocchio's `InstructionContext` | Pinocchio v0.11 lazy entrypoint | Medium | ★★★ |
| 8 | **MIN_LEN / MAX_LEN** for dynamic layouts | Quasar's `MIN_SPACE`/`MAX_SPACE` | Small | ★★ |

### Tier 3: Nice-to-Have (Low Impact or Low Urgency)

| # | Innovation | Source Inspiration | Effort | Impact |
|---|---|---|---|---|
| 9 | **Field projection interfaces** -- partial struct overlays | Novel | Medium | ★★ |
| 10 | **wincode feature gate** for Le* types | solana-zero-copy's wincode support | Small | ★ |
| 11 | **Realloc-safe segment growth** -- detect pinocchio resize feature | Pinocchio account-resize | Small | ★ |

---

## Detailed Design: Top 3 Innovations

### Innovation 1: `inline_dynamic_layout!`

**Problem:** `segmented_layout!` has 12 bytes overhead per segment. For 1–2 dynamic fields (the common case -- e.g., a name string + a tags array), the segment table is heavy.

**Design:**

```rust
inline_dynamic_layout! {
    pub struct Profile, discriminator = 3, version = 1 {
        // Fixed portion (standard zero_copy_layout)
        header:    AccountHeader = 16,
        authority: Address       = 32,
        score:     LeU64         = 8,
        // Dynamic portion (inline prefix + data)
        name:      DynString<LeU16, 64>,    // 2-byte LE prefix, max 64 bytes
        avatar:    DynBytes<LeU16, 256>,     // 2-byte LE prefix, max 256 bytes
        tags:      DynSlice<Tag, LeU8, 16>,  // 1-byte prefix, max 16 elems
    }
}
```

**Generated types:**
- `ProfileFixed` -- `#[repr(C)]` overlay for bytes 0..56
- `Profile<'a>` -- parsed view borrowing `&'a [u8]` with `__off: [u32; 2]`
- `ProfileMut<'a>` -- mutable variant with write accessors
- `Profile::LAYOUT_ID` -- hash includes `"DynString:LeU16:64"` etc.
- `Profile::MIN_LEN = 56 + 2 + 2 + 1` (fixed + prefixes)
- `Profile::MAX_LEN = MIN_LEN + 64 + 256 + 16 * size_of::<Tag>()`

**layout_id computation:**
```
sha256("jiminy:v1:Profile:1:header:AccountHeader:16,authority:Address:32,score:LeU64:8,name:DynString:LeU16:64,avatar:DynBytes:LeU16:256,tags:DynSlice:Tag:LeU8:16,")[..8]
```

The dynamic field metadata is part of the hash, making cross-program verification possible.

**Key rules:**
1. Fixed fields must precede all dynamic fields
2. All dynamic field element types must be `Pod + FixedLayout` (alignment 1)
3. At most one `DynBytes` or `DynString` can be "tail" (no prefix, consumes rest)
4. `load()` walks prefixes once, caches N-1 offsets
5. Write operations compute total space and realloc if needed

**Correctness invariants:**
- Sum of (prefix_size + actual_data_len) for all dynamic fields + fixed_len = account data_len
- Each prefix value ≤ declared max
- Element types have alignment 1

### Innovation 2: `segmented_interface!`

**Problem:** `jiminy_interface!` only works for fixed-size accounts. Programs reading segmented accounts from other programs have no macro support.

**Design:**

```rust
segmented_interface! {
    /// Read-only view of DEX Program's OrderBook (segmented).
    pub struct OrderBook for DEX_PROGRAM, version = 1 {
        // Fixed prefix (overlaid)
        header:     AccountHeader = 16,
        market:     Address       = 32,
        base_mint:  Address       = 32,
        quote_mint: Address       = 32,
        // Segment declarations (read-only views)
        [0] bids: Order,    // segment index 0, element type Order
        [1] asks: Order,    // segment index 1, element type Order
    }
}
```

**Generated API:**
```rust
// Tier 2 loading with segment validation
let ob = OrderBook::load_foreign(account)?;

// Fixed portion access (O(1))
let market = ob.fixed().market;

// Segment access (reads descriptor, returns SegmentSlice)
let bids = ob.bids()?;   // SegmentSlice<Order>
let asks = ob.asks()?;    // SegmentSlice<Order>

for bid in bids.iter() {
    // zero-copy iteration over foreign account's segments
}
```

**Validation:**
1. Owner check against `DEX_PROGRAM`
2. Layout_id check on the fixed prefix (first 16 bytes)
3. Segment table validation: correct number of segments, element_size matches `Order::SIZE`
4. Bounds checking: segment data within account bounds

### Innovation 3: Account Introspection

**Problem:** No standard way to inspect raw account bytes and determine structure.

**Design:**

```rust
/// Best-effort account structure inspection.
/// For tooling only -- NOT a trust boundary.
pub struct AccountShape {
    /// True if bytes 0..16 look like a valid Jiminy header.
    pub has_jiminy_header: bool,
    /// Discriminator byte (if header present).
    pub discriminator: u8,
    /// Version byte (if header present).
    pub version: u8,
    /// Flags (if header present).
    pub flags: u16,
    /// Layout ID (if header present).
    pub layout_id: [u8; 8],
    /// True if flags indicate segmented layout.
    pub is_segmented: bool,
    /// Number of segments detected (if segmented).
    pub segment_count: u8,
    /// Total account data length.
    pub data_len: usize,
}

impl AccountShape {
    /// Inspect raw bytes. Does NOT validate ownership or correctness.
    /// Returns `None` if data is too short for a header.
    pub fn inspect(data: &[u8]) -> Option<Self> { ... }
    
    /// Check if the layout_id matches a known layout.
    pub fn matches_layout(&self, expected: &[u8; 8]) -> bool {
        self.has_jiminy_header && self.layout_id == *expected
    }
}
```

Integrates with `jiminy-schema`'s `LayoutManifest` for full account type resolution.

---

## What Jiminy Already Does Better Than Everyone

For context, here's what the research confirms Jiminy leads on:

| Capability | Jiminy | Pinocchio | solana-zero-copy | Quasar | spl-list-view |
|---|---|---|---|---|---|
| Deterministic layout_id | ✅ SHA-256 | ❌ | ❌ | ❌ (dev discriminator) | ❌ |
| Cross-program ABI | ✅ Tier 2 + interfaces | ❌ | ❌ | ❌ | ❌ |
| Tiered trust model | ✅ 5 tiers | ❌ | ❌ | ❌ (1 tier) | ❌ |
| Segmented accounts | ✅ capacity-aware | ❌ | ❌ | Inline only | Single list |
| Alignment-1 LE types | ✅ Le* + arithmetic | ❌ | ✅ (no arithmetic) | ✅ (wrapping arith) | ❌ |
| Schema tooling (TS/JSON) | ✅ jiminy-schema + @jiminy/ts | ❌ | ❌ | IDL-based | ❌ |
| Account versioning | ✅ extends + compat tier | ❌ | ❌ | ❌ | ❌ |
| No proc macros | ✅ declarative only | ✅ | ✅ | ❌ (proc macros) | ✅ |
| Inline dynamic fields | ❌ | ❌ | ❌ | ✅ | ❌ |
| Raw CPI pass-through | ❌ | ❌ | ❌ | ✅ | ❌ |

**Critical advantages to maintain:**
1. `layout_id` -- deterministic, schema-change-detecting ABI fingerprint. Nobody else has this.
2. `jiminy_interface!` -- the ONLY cross-program type-safe read system in the ecosystem.
3. Declarative-only macros -- auditable, no hidden codegen.
4. Tiered trust -- explicit security posture selection.
5. Segment capacity tracking -- deterministic rent, no surprise reallocs.

---

## Risk Assessment

| Innovation | Risk | Mitigation |
|---|---|---|
| `inline_dynamic_layout!` | Complexity -- two dynamic patterns (segmented + inline) | Clear docs: "use inline for 1-2 fields, segmented for 3+" |
| `inline_dynamic_layout!` | layout_id hash format change | New hash prefix `jiminy:v1:dyn:` to distinguish |
| `segmented_interface!` | Foreign segment table could be malformed | Strict bounds checking, element_size validation |
| Multi-program interface | Widens attack surface (more programs pass owner check) | Explicit program set, document trust implications |
| Introspection | Users might confuse inspection with verification | Name it `inspect()`, document "NOT a trust boundary" |

---

## Next Steps

1. **Prototype `inline_dynamic_layout!`** -- start with fixed-then-dynamic field ordering, u8/u16/u32 prefixes, offset caching. Verify layout_id determinism.
2. **Add `AccountShape::inspect()`** -- small, immediately useful for jiminy-schema and @jiminy/ts.
3. **Extend `jiminy_interface!`** with program set support -- backward-compatible addition.
4. **Design `segmented_interface!`** -- requires careful cross-program segment table validation spec.
5. **Generic `ZeroCopySlice<T, L>`** -- quick win, backward-compatible.
