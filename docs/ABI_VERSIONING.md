# ABI Versioning

Jiminy account layouts follow **append-only** versioning. You can add
fields. You can never remove, reorder, or shrink them. This document
describes how to evolve account schemas without breaking existing
on-chain data -- or anyone reading it.

## Core Principles

1. **Append only.** New fields are appended after existing ones. Existing
   field offsets never change within a discriminator.

2. **New layout_id per version.** Changing any field (name, type, size,
   order) or bumping `version` produces a new `LAYOUT_ID`. Readers can
   reject accounts whose layout doesn't match their expectations.

3. **Discriminator is permanent.** Once an account type is assigned a
   discriminator value, that value is never reused for a different type.

4. **Account size never shrinks.** If fields are removed in a new
   version, their bytes become reserved padding.

## Version Bumps

Bump `version` in `zero_copy_layout!` whenever:

- A field is added, removed, renamed, retyped, or resized.
- Field order changes.
- The account is extended with trailing fields.

```rust
// v1
zero_copy_layout! {
    pub struct Pool, discriminator = 3, version = 1 {
        header:    AccountHeader = 16,
        reserve_a: u64           = 8,
        reserve_b: u64           = 8,
    }
}

// v2 -- new field appended
zero_copy_layout! {
    pub struct PoolV2, discriminator = 3, version = 2 {
        header:    AccountHeader = 16,
        reserve_a: u64           = 8,
        reserve_b: u64           = 8,
        fee_bps:   u16           = 2,
    }
}
```

`Pool::LAYOUT_ID != PoolV2::LAYOUT_ID` -- the hash changes automatically
because the version and fields differ.

## Layout Inheritance (`extends`)

Use `extends = ParentType` to make the compiler enforce that a new
version is a byte-compatible superset of an older version:

```rust
zero_copy_layout! {
    pub struct PoolV2, discriminator = 3, version = 2, extends = Pool {
        header:    AccountHeader = 16,
        reserve_a: u64           = 8,
        reserve_b: u64           = 8,
        fee_bps:   u16           = 2,
    }
}
```

The macro asserts at compile time:

1. `PoolV2::DISC == Pool::DISC` -- same discriminator (same account type)
2. `PoolV2::LEN >= Pool::LEN` -- child is at least as large (append-only)
3. `PoolV2::VERSION > Pool::VERSION` -- child is a strictly newer version

If any assertion fails, the build fails with a clear error message. This
catches accidental discriminator mismatches or shrinking layouts before
they reach mainnet.

## Reading Older Versions

When a program encounters an account with a lower version than expected,
it can:

1. **Read the common prefix.** V1 fields occupy the same offsets in V2.
   Overlay the smaller struct to read just the fields you know.

2. **Migrate in-place.** Realloc the account (if needed), write new
   default values for appended fields, bump the version byte, and update
   the layout_id.

```rust
let data = account.try_borrow()?;
let ver = data[1]; // version byte

match ver {
    1 => {
        let pool = PoolV1::overlay(&data)?;
        // Use v1 fields; fee_bps doesn't exist yet.
    }
    2 => {
        let pool = PoolV2::overlay(&data)?;
        // Full access to all fields.
    }
    _ => return Err(ProgramError::InvalidAccountData),
}
```

## Backward-Compatible Loading

> **⚠ Compatibility validation is a migration helper, not a proof of ABI
> identity.** `validate_version_compatible` intentionally skips the `layout_id`
> check. It verifies owner + discriminator + `version >= min_version` +
> minimum size, but it does **not** confirm that the account's field
> layout matches your struct. It does **not** validate `layout_id` and
> must not be treated as equivalent to `load()` or `load_foreign()`.
>
> This is a **versioned migration utility** for use during version
> transitions only. For all other paths, use `load()` (Tier 1) or
> `load_foreign()` (Tier 2), which verify the full ABI fingerprint.

Use `validate_version_compatible` when you want to accept any version at or
above a minimum -- for example, during an in-place migration from V1 to
V2:

```rust
validate_version_compatible(account, program_id, DISC, /*min_version=*/1, MIN_SIZE)?;
```

Because no `layout_id` is checked, the caller is responsible for
ensuring the overlaid struct is compatible with the actual on-chain
bytes. Prefer `load()` or `load_checked()` in all non-migration paths.

## Layout ID Determinism

The layout_id is computed at compile time:

```
sha256("jiminy:v1:<Name>:<version>:<field>:<type>:<size>,...")[..8]
```

The `"jiminy:v1:"` prefix is versioned so the hashing scheme itself can
evolve in future major releases without colliding with existing IDs.

Field order in the hash matches declaration order -- reordering fields
produces a different layout_id even if the byte layout is identical.
This is intentional: field semantics depend on position.
