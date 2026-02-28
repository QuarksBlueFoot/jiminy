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

Every function is `#[inline(always)]`. Designed to inline away in BPF builds;
the [benchmark suite](#benchmarks) shows 3–16 CU of overhead per instruction
with a smaller binary than hand-written Pinocchio.

---

## Install

```toml
[dependencies]
jiminy = "0.2"
```

## Quick Start

```rust
use jiminy::prelude::*;
```

The prelude re-exports all check functions, macros, cursors, header helpers,
math utilities, `AccountList`, and the pinocchio core types (`AccountView`,
`Address`, `ProgramResult`, `ProgramError`). One import, everything you need.

> **No proc macros** is both an advantage and a conscious tradeoff. Less surface
> area = fewer build surprises = fully auditable. The tradeoff: no auto-generated
> IDL or client code. For the teams that care about CU budgets and binary size,
> that's the right call.

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

| Function | Anchor equivalent | What it does |
| --- | --- | --- |
| `check_signer(account)` | `signer` | Must be a transaction signer |
| `check_writable(account)` | `mut` | Must be marked writable |
| `check_owner(account, program_id)` | `owner` | Must be owned by your program |
| `check_pda(account, expected)` | `seeds + bump` | Address must match the derived PDA |
| `check_system_program(account)` | `Program<System>` | Must be the system program |
| `check_executable(account)` | `executable` | Must be an executable program |
| `check_uninitialized(account)` | `init` | Data must be empty — prevents reinit attacks |
| `check_has_one(stored, account)` | `has_one` | Stored address field must match account key |
| `check_rent_exempt(account)` | `rent_exempt` | Must hold enough lamports to be rent-exempt |
| `check_lamports_gte(account, min)` | `constraint` | Must hold at least `min` lamports |
| `check_closed(account)` | `close` | Must have zero lamports and empty data |
| `check_size(data, min_len)` | — | Raw slice is at least N bytes |
| `check_discriminator(data, expected)` | `discriminator` | First byte must match type tag |
| `check_account(account, id, disc, len)` | composite | Owner + size + discriminator in one call |

### Macros

| Macro | Anchor equivalent | What it does |
| --- | --- | --- |
| `require!(cond, err)` | `require!` | Return error if condition is false |
| `require_eq!(a, b, err)` | `require_eq!` | `a == b` (scalars) |
| `require_neq!(a, b, err)` | `require_neq!` | `a != b` (scalars) |
| `require_gt!(a, b, err)` | `require_gt!` | `a > b` |
| `require_gte!(a, b, err)` | `require_gte!` | `a >= b` |
| `require_lt!(a, b, err)` | `require_lt!` | `a < b` |
| `require_lte!(a, b, err)` | `require_lte!` | `a <= b` |
| `require_keys_eq!(a, b, err)` | `require_keys_eq!` | Two `Address` values must be equal |
| `require_keys_neq!(a, b, err)` | `require_keys_neq!` | Two `Address` values must differ |
| `require_accounts_ne!(a, b, err)` | — | Two accounts must have different addresses |
| `require_flag!(byte, n, err)` | — | Bit `n` must be set in `byte` |

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
| --- | --- | --- | --- |
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

Jiminy is being used in [SHIPyard](https://github.com/BluefootLabs/SHIPyard) —
a platform for building, deploying, and sharing Solana programs. The on-chain
program registry is built with Jiminy's check functions and layout convention,
and the code generator targets Jiminy as a framework option.

---

## Account Layout Convention

Jiminy ships an opinionated [Account Layout v1](docs/LAYOUT_CONVENTION.md)
convention — an 8-byte header with discriminator, version, flags, and
optional `data_len`. Use `write_header` / `check_header` / `header_payload`
for versioned, evolvable account schemas without proc macros.

See [docs/LAYOUT_CONVENTION.md](docs/LAYOUT_CONVENTION.md) for the full spec
and a copy-pasteable layout lint test.

---

## Benchmarks

Comparing a vault program (deposit / withdraw / close) written in raw
Pinocchio vs the same logic using Jiminy. Measured via
[Mollusk SVM](https://github.com/anza-xyz/mollusk) on Agave 2.3.

### Compute Units

| Instruction | Pinocchio | Jiminy | Delta |
|-------------|-----------|--------|-------|
| Deposit     | 146 CU    | 149 CU | +3    |
| Withdraw    | 253 CU    | 266 CU | +13   |
| Close       | 214 CU    | 230 CU | +16   |

### Binary Size (release SBF)

| Program | Size |
|---------|------|
| Pinocchio vault | 18.7 KB |
| Jiminy vault    | 17.4 KB |

Jiminy adds **3–16 CU** of overhead per instruction (a single `sol_log` costs
~100 CU). The binary is actually **1.3 KB smaller** thanks to pattern
deduplication from `AccountList` and the check functions.

See [BENCHMARKS.md](BENCHMARKS.md) for full details and instructions to run
them yourself.

---

## Reference Programs

| Program | What it demonstrates |
|---------|---------------------|
| [`examples/jiminy-vault`](examples/jiminy-vault) | Init/deposit/withdraw/close with `AccountList`, cursors, `safe_close` |
| [`examples/jiminy-escrow`](examples/jiminy-escrow) | Two-party escrow, flag-based state, `check_closed`, ordering guarantees |

Both use the Jiminy Header v1 layout. Fork them as starting templates.

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
