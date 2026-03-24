# Cross-Program ABI Read Demo

This example demonstrates Jiminy's zero-copy cross-program read
capability. **Program B reads Program A's account with no deserialization
and no dependency on Program A's crate.**

## Setup

### Program A: defines and owns a `Vault` account

```rust
// program_a/state.rs
use jiminy::prelude::*;

zero_copy_layout! {
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}
```

Program A creates and manages Vault accounts. Each account gets a
16-byte header with Jiminy's deterministic `LAYOUT_ID`.

### Program B: reads Program A's Vault, no crate dependency

```rust
// program_b/reader.rs
use jiminy::prelude::*;

// Program B declares its own view struct. Same fields, same sizes.
// It does NOT import program_a::Vault. It does NOT add program_a as
// a Cargo dependency. The only contract is the byte layout.
zero_copy_layout! {
    pub struct VaultView, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

pub fn read_vault_balance(
    vault_account: &AccountView,
    program_a_id: &Address,
) -> Result<u64, ProgramError> {
    // Tier 2: cross-program read.
    // Validates: owner == program_a_id, layout_id matches, exact size.
    // Does NOT check discriminator or version (foreign program's convention).
    let verified = VaultView::load_foreign(vault_account, program_a_id)?;
    let vault = verified.get();

    Ok(vault.balance)
}
```

## Why this works

1. **layout_id matching.** Both `Vault` and `VaultView` have identical
   field names, types, sizes, and version, so they produce the same
   `LAYOUT_ID` hash. `load_foreign` verifies this at runtime.

2. **#[repr(C)] determinism.** Field offsets are the running sum of
   preceding field sizes. Same fields → same offsets. No alignment
   surprises.

3. **Zero deserialization.** The `overlay` call returns a typed reference
   directly into the borrowed account data. No copies, no allocations,
   no borsh.

## What happens when Program A changes its layout?

| Change | Effect |
|--------|--------|
| Add a field (new version) | `LAYOUT_ID` changes → `load_foreign` rejects until Program B updates `VaultView` |
| Rename a field | `LAYOUT_ID` changes → rejected |
| Reorder fields | `LAYOUT_ID` changes → rejected |
| Change field type | `LAYOUT_ID` changes → rejected |
| No change | `LAYOUT_ID` matches → reads succeed |

The ABI fingerprint makes silent schema drift impossible. Program B
either reads the exact layout it expects, or it gets a clear error.

## Trust model

`load_foreign` proves **ABI identity** (the bytes match your struct).
It does **not** prove the foreign program is trustworthy. The caller
must independently decide whether they trust the program at
`program_a_id`.

See [docs/ACCOUNT_ABI_CONTRACT.md](../../docs/ACCOUNT_ABI_CONTRACT.md)
for the full cross-program read contract and failure modes.
