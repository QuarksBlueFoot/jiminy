# Account ABI Contract

This is the cross-program interoperability contract for Jiminy accounts.
If your program follows this contract, any other Jiminy program can read
your accounts without touching your crate. No shared dependency, no
Borsh, no coordination. Just deterministic byte layouts.

## The Contract

Any Jiminy account satisfies three properties:

1. **Fixed 16-byte header** at offset 0.
2. **Deterministic layout_id** derived from the struct schema.
3. **Deterministic field offsets** via `#[repr(C)]` ordering.

Together, these properties mean: *any program can read any Jiminy
account without depending on the source crate.*

## Header Layout

```
Offset  Size  Field          Description
──────────────────────────────────────────────────────
0       1     discriminator  Account type (unique per program)
1       1     version        Schema version
2       2     flags          Application-defined bitfield (LE)
4       8     layout_id      ABI fingerprint
12      4     reserved       Must be zero
──────────────────────────────────────────────────────
```

Payload fields start at byte 16.

## Cross-Program Read Protocol

To read a Jiminy account owned by another program:

```rust
use jiminy::prelude::*;

// 1. Verify the account is owned by the expected program.
// 2. Verify the layout_id matches your expected schema.
// 3. Verify minimum size.
validate_foreign(account, &OTHER_PROGRAM_ID, &ExpectedLayout::LAYOUT_ID, ExpectedLayout::LEN)?;

// 4. Borrow and overlay.
let data = account.try_borrow()?;
let layout = ExpectedLayout::overlay(&data)?;
```

Or use the macro-generated method:

```rust
let verified = ExpectedLayout::load_foreign(account, &OTHER_PROGRAM_ID)?;
let layout = verified.get();
```

## Why This Works

### layout_id = ABI fingerprint

The layout_id is:

```
sha256("jiminy:v1:<Name>:<version>:<field>:<canonical_type>:<size>,...")[..8]
```

If Program A's `Vault` and Program B's local `VaultView` produce the
same layout_id, their field offsets are guaranteed to match, because
the hash input encodes the exact field names, types, sizes, and order.

### #[repr(C)] + compile-time enforcement = deterministic offsets

All Jiminy layouts use `#[repr(C)]`. This preserves field declaration
order, but `#[repr(C)]` alone does not eliminate padding. The compiler
may insert alignment padding between fields.

Jiminy eliminates this risk with a **compile-time assertion** inside
`zero_copy_layout!`:

```rust,ignore
const _: () = assert!(
    core::mem::size_of::<T>() == 0 + f1_size + f2_size + ...,
    "size_of does not match declared LEN - check field sizes"
);
```

If the actual `size_of` (which includes any compiler-inserted padding)
does not equal the sum of declared field sizes, the program fails to
compile. This guarantees that field offsets are the running sum of
preceding field sizes - no hidden gaps.

### No crate dependency needed

Program B doesn't need to `use program_a::Vault`. It declares its own
struct with the same fields:

```rust
zero_copy_layout! {
    pub struct VaultView, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}
```

If the layout_id matches at runtime, the overlay is safe.

## Failure Modes

| Scenario | Detection |
|----------|-----------|
| Program A changes a field name | layout_id changes → `validate_foreign` fails |
| Program A reorders fields | layout_id changes → `validate_foreign` fails |
| Program A adds a field (new version) | layout_id changes; old version's layout_id still works if Program B uses the old struct |
| Program A changes field type but not name | layout_id changes (canonical type differs) |
| Account not owned by expected program | `validate_foreign` checks `owned_by` → `IllegalOwner` |
| Account too small for expected layout | `validate_foreign` checks `min_size` → `AccountDataTooSmall` |

## Limitations

- **layout_id is not a full schema hash.** It is 8 bytes (64 bits).
  Collisions are theoretically possible but astronomically unlikely for
  reasonable numbers of distinct layouts.

- **Semantic compatibility is not guaranteed.** Two structs can have the
  same field types and sizes but assign different meanings to the same
  bytes. layout_id matching means *structural* compatibility, not
  *semantic* agreement.

- **Dynamic-length accounts use the segmented ABI.** The fixed-field
  ABI contract applies to the `#[repr(C)]` prefix region. For
  variable-length data (collections, extension zones), Jiminy provides
  the segmented ABI (`segmented_layout!`, `SegmentTable`,
  `SegmentSlice`). Each segment is described by a `SegmentDescriptor`
  (offset + count + capacity + element_size) in a region table immediately after the fixed prefix.
  See `SEGMENTED_ABI.md` for details.
