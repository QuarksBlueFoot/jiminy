# Safe Composition Patterns

How to safely read accounts from other programs and compose with
untrusted data.

---

## The problem

A `layout_id` match proves the account has the right byte layout.
It does not prove the account is honest. A malicious program can deploy
accounts with the same `layout_id` (same struct shape) but contain
fabricated data.

`layout_id` proves ABI identity. It does not prove trust.

---

## Composition checklist

When reading an account from another program, always combine:

1. **Owner check** - the account is owned by the expected program
2. **`layout_id` check** - the bytes match the expected struct layout
3. **Authority field validation** - a stored authority/admin address
   matches the expected signer or PDA

Missing any one of these opens an attack vector.

```rust
// Good: all three checks
let data = ForeignVault::load_foreign(account, &TRUSTED_PROGRAM)?;
let vault = ForeignVault::overlay(&data)?;
check_has_one(&vault.authority(), signer)?;

// Bad: layout_id alone (malicious program could match the shape)
let data = ForeignVault::load_foreign(account, &ANY_OWNER)?; // don't do this
```

---

## Program allowlists

When your program accepts accounts from a known set of partner programs,
use `check_program_allowed` instead of a single owner check:

```rust
const ALLOWED_VAULTS: &[Address] = &[PROGRAM_A, PROGRAM_B];

check_program_allowed(vault_account, ALLOWED_VAULTS)?;
let data = vault_account.try_borrow()?;
// ... read data ...
```

This is better than chaining `if owner == A || owner == B` because the
allowlist is a const slice you can update in one place.

---

## Strict account validation

Use `check_account_strict!` instead of `check_account!` for any account
that touches funds or authority. The strict variant requires `owner`,
`disc`, and `layout_id` as mandatory arguments. Forgetting any of them
is a compile error:

```rust
// Strict: owner + disc + layout_id required, compile error if missing
check_account_strict!(vault,
    owner = program_id,
    disc = Vault::DISC,
    layout_id = &Vault::LAYOUT_ID,
    writable,
    size >= Vault::LEN,
)?;

// Flexible: any subset of constraints (use for less critical accounts)
check_account!(metadata, owner = program_id)?;
```

---

## PDA verification

Always verify PDA derivation for accounts that should be program-derived.
`require_pda!` combines seed construction and assertion in one call:

```rust
let bump = require_pda!(vault_account, program_id, b"vault", user.address())?;
```

This is equivalent to manually building a seed slice and calling
`assert_pda`, but harder to get wrong.

---

## Rules of thumb

- `load_foreign()` is safe for reading shape. Pair it with authority
  validation for reading meaning.
- Never trust `layout_id` alone across program boundaries.
- Use `check_program_allowed` over ad-hoc owner comparisons.
- Use `check_account_strict!` for anything security-critical.
- Verify PDA derivation for every PDA account, every time.
