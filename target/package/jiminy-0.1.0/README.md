# jiminy

[![crates.io](https://img.shields.io/crates/v/jiminy.svg)](https://crates.io/crates/jiminy)
[![docs.rs](https://docs.rs/jiminy/badge.svg)](https://docs.rs/jiminy)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Pinocchio is the engine. Jiminy keeps it honest.**

Writing Solana programs with [pinocchio](https://github.com/anza-xyz/pinocchio)
is fast — no allocator, no borsh, full control over every byte in every account.
The tradeoff is that safety checks end up scattered through your handlers. Signer
check here, owner check there, discriminator byte manually compared somewhere else,
and an overflow somewhere you forgot to look at.

Jiminy bundles those checks into composable functions and macros. It doesn't abstract
away pinocchio — you still work directly with `AccountView`, `Address`, and raw byte
slices. Jiminy just makes the guard-rail part less repetitive, and adds a few things
that neither pinocchio nor Anchor ever got around to building.

**No allocator. No borsh. No proc macros. BPF-safe.**

Every function is `#[inline(always)]`. At compile time it produces the same
instructions you'd write by hand, just without the copy-paste.

---

## Install

```toml
[dependencies]
jiminy = "0.1"
```

---

## A real example

```rust
use jiminy::{
    check_account, check_signer, check_writable, check_system_program,
    check_uninitialized, check_lamports_gte, safe_close, checked_add,
    require, require_accounts_ne,
    SliceCursor, DataWriter, write_discriminator,
};

const VAULT_DISC: u8 = 1;
const VAULT_LEN: usize = 41; // 1 disc + 8 balance + 32 authority

fn process_transfer(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let authority = &accounts[0];
    let from_vault = &accounts[1];
    let to_vault   = &accounts[2];

    check_signer(authority)?;
    check_writable(from_vault)?;
    check_writable(to_vault)?;

    // Same account passed twice? Catch it before anything happens.
    require_accounts_ne!(from_vault, to_vault, ProgramError::InvalidArgument);

    check_account(from_vault, program_id, VAULT_DISC, VAULT_LEN)?;
    check_account(to_vault,   program_id, VAULT_DISC, VAULT_LEN)?;

    // Read instruction args
    let mut ix = SliceCursor::new(instruction_data);
    let amount = ix.read_u64()?;

    // Read from_vault fields
    let from_data = from_vault.try_borrow()?;
    let mut cur = SliceCursor::new(&from_data[1..]);
    let balance   = cur.read_u64()?;
    let vault_auth = cur.read_address()?;
    drop(from_data);

    require_keys_eq!(authority.address(), &vault_auth, ProgramError::MissingRequiredSignature);
    require_gte!(balance, amount, ProgramError::InsufficientFunds);

    // ... update balances ...
    Ok(())
}
```

---

## What's in the box

### Account checks

| Function | What it does |
|---|---|
| `check_signer(account)` | Must be a transaction signer |
| `check_writable(account)` | Must be marked writable |
| `check_owner(account, program_id)` | Must be owned by your program |
| `check_pda(account, expected)` | Address must match the derived PDA |
| `check_system_program(account)` | Must be the system program |
| `check_uninitialized(account)` | Data must be empty — prevents reinit attacks |
| `check_lamports_gte(account, min)` | Must hold at least `min` lamports |
| `check_closed(account)` | Must have zero lamports and empty data |
| `check_size(data, min_len)` | Raw slice is at least N bytes |
| `check_discriminator(data, expected)` | First byte must match type tag |
| `check_account(account, id, disc, len)` | Owner + size + discriminator in one call |

### Macros

| Macro | What it does |
|---|---|
| `require!(cond, err)` | Return error if condition is false |
| `require_keys_eq!(a, b, err)` | Two `Address` values must be equal |
| `require_accounts_ne!(a, b, err)` | Two accounts must have **different** addresses |
| `require_gte!(a, b, err)` | `a >= b` |
| `require_gt!(a, b, err)` | `a > b` |
| `require_eq!(a, b, err)` | `a == b` (scalars) |

### Math

| Function | What it does |
|---|---|
| `checked_add(a, b)` | Overflow-safe u64 addition |
| `checked_sub(a, b)` | Underflow-safe u64 subtraction |
| `checked_mul(a, b)` | Overflow-safe u64 multiplication |

### Account lifecycle

| Function | What it does |
|---|---|
| `safe_close(account, destination)` | Move all lamports + close atomically |
| `write_discriminator(data, disc)` | Write type tag byte when initializing |

---

## What Anchor doesn't give you

Anchor is good at what it does. But once you step off the borsh treadmill and
go zero-copy, a few things fall off the table. These are the gaps we built
Jiminy to fill.

### `SliceCursor` — field reads without the arithmetic

Reading fields from raw account data in pinocchio usually means keeping byte
offsets in your head or in constants, then slicing manually. Fine for one or two
fields, annoying for five, and a footgun when you change the layout and forget
to update an offset three functions away.

`SliceCursor` tracks the position for you:

```rust
let data = account.try_borrow()?;
let mut cur = SliceCursor::new(&data[1..]); // skip discriminator
let balance   = cur.read_u64()?;
let authority = cur.read_address()?;
let is_locked = cur.read_bool()?;
let padding   = cur.skip(3)?;
```

No alloc. No schema. If you run off the end of the buffer you get
`AccountDataTooSmall`, not a panic or silent garbage.

Supported reads: `u8`, `u16`, `u32`, `u64`, `i64`, `bool`, `Address`, `skip`.

### `DataWriter` — field writes without the arithmetic

The write-side complement to `SliceCursor`. Use it when initializing account
data inside a create instruction. Same idea — position-tracked, bounds-checked,
every write little-endian.

```rust
let mut raw = new_account.try_borrow_mut()?;
write_discriminator(&mut raw, VAULT_DISC)?;
let mut w = DataWriter::new(&mut raw[1..]);
w.write_u64(0)?;               // initial balance
w.write_address(&authority)?;  // 32-byte authority key
```

The `write_discriminator` helper is separate so it's explicit that byte zero
is special — it's the type tag that every other check function looks at first.

### `require_accounts_ne!` — source ≠ destination

One of the oldest classes of token program bugs: pass the same account as both
source and destination, end up with corrupted state or a free mint. Anchor doesn't
have a built-in constraint for this. You'd need a custom constraint or an inline
`if source.key() == destination.key() { return err; }`.

```rust
require_accounts_ne!(source_vault, dest_vault, MyError::SameAccount);
```

One line. Runs before you touch any balances.

### `check_lamports_gte` — collateral and fee floors

```rust
// Verify the collateral account holds enough before accepting a position
check_lamports_gte(collateral, required_collateral_lamports)?;
```

Anchor's constraint system doesn't expose lamport checks directly. You'd write
a custom constraint or inline the comparison. Here it's a named function with
an obvious error return.

### `check_closed` — verify a previous close actually happened

In CPI-heavy programs you sometimes need to confirm that an account was fully
closed by an earlier instruction before you proceed — whether that's reusing the
address, completing a multi-step flow, or enforcing ordering guarantees.

```rust
// Confirm the escrow was already closed before releasing collateral
check_closed(old_escrow)?;
```

Zero lamports and empty data. If either condition isn't met, you get
`InvalidAccountData` and stop.

---

## Compared to the alternatives

|  | Raw pinocchio | Anchor | **Jiminy** |
|---|---|---|---|
| Allocator required | No | Yes | No |
| Borsh required | No | Yes | No |
| Proc macros | No | Yes | No |
| Account validation | Manual | `#[account(...)]` constraints | Functions + macros |
| Data reads | Manual index arithmetic | `Account<'info, T>` + borsh | `SliceCursor` |
| Data writes | Manual index arithmetic | Automatic via borsh | `DataWriter` |
| Overflow-safe math | Manual | Built-in | `checked_add/sub/mul` |
| Source ≠ dest guard | Manual | Not built-in | `require_accounts_ne!` |
| Lamport floor check | Manual | Not built-in | `check_lamports_gte` |
| Close verification | Manual | Not built-in | `check_closed` |

The point isn't that Anchor is bad — it's that once you're working at the
pinocchio level, you shouldn't have to give up composable safety primitives
to do it.

---

## Used in SHIPyard

Jiminy powers the on-chain program registry in [SHIPyard](https://github.com/BluefootLabs/SHIPyard) —
a platform for building, deploying, and sharing Solana programs. Every
instruction handler in the project registry uses Jiminy's check functions,
and SHIPyard's code generator can target Jiminy as a framework option
when generating programs from IR.

Real usage in a production program is the best test. If something doesn't
hold up, we'll find it.

---

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy has saved you some debugging time, donations are welcome
at `SolanaDevDao.sol` — it goes toward keeping development going.

---

## License

Apache-2.0. See [LICENSE](LICENSE).

pinocchio is also Apache-2.0 — [anza-xyz/pinocchio](https://github.com/anza-xyz/pinocchio).
Apache wrapping Apache, all the way down.
