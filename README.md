# jiminy

[![crates.io](https://img.shields.io/crates/v/jiminy.svg)](https://crates.io/crates/jiminy)
[![docs.rs](https://docs.rs/jiminy/badge.svg)](https://docs.rs/jiminy)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Pinocchio is the engine. Jiminy keeps it honest.**

If you're writing Solana programs with [pinocchio](https://github.com/anza-xyz/pinocchio),
you already know the deal. No allocator, no borsh, raw bytes, full control. Insanely
fast. But every handler ends up with the same boilerplate: signer check, owner check,
discriminator comparison, overflow math, PDA derivation. Copy paste it enough times
and something slips through.

Jiminy gives you composable check functions, PDA assertions that return bumps,
zero-copy token account readers, safe math, and data cursors. All `#[inline(always)]`,
all `no_std`, all BPF-safe. You're still writing pinocchio. You're just not writing
the boring parts by hand anymore.

**No allocator. No borsh. No proc macros.**

The [benchmarks](#benchmarks) show 3-16 CU overhead per instruction and a smaller
binary than hand-rolled pinocchio. Not a typo.

---

## Install

```toml
[dependencies]
jiminy = "0.3"
```

## Quick Start

```rust
use jiminy::prelude::*;
```

The prelude gives you everything: check functions, assert functions, token
account readers, macros, cursors, math, `AccountList`, and the pinocchio
core types. One import line.

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
| `check_uninitialized(account)` | `init` | Data must be empty (anti-reinit) |
| `check_has_one(stored, account)` | `has_one` | Stored address field must match account key |
| `check_rent_exempt(account)` | `rent_exempt` | Must hold enough lamports to be rent-exempt |
| `check_lamports_gte(account, min)` | `constraint` | Must hold at least `min` lamports |
| `check_closed(account)` | `close` | Must have zero lamports and empty data |
| `check_size(data, min_len)` | | Raw slice is at least N bytes |
| `check_discriminator(data, expected)` | `discriminator` | First byte must match type tag |
| `check_account(account, id, disc, len)` | composite | Owner + size + discriminator in one call |

### Assert functions

These do more than just check a condition. They derive, compare, and return useful data.

| Function | What it does |
| --- | --- |
| `assert_pda(account, seeds, program_id)` | Derive PDA, verify match, **return bump** |
| `assert_pda_with_bump(account, seeds, bump, id)` | Verify PDA with known bump (way cheaper) |
| `assert_pda_external(account, seeds, id)` | Same as `assert_pda` for external program PDAs |
| `assert_token_program(account)` | Must be SPL Token *or* Token-2022 |
| `assert_address(account, expected)` | Account address must match exactly |
| `assert_program(account, expected)` | Address match + must be executable |
| `assert_not_initialized(account)` | Lamports == 0 (account doesn't exist yet) |

### Token account readers

Zero-copy field reads from SPL Token accounts. No deserialization, no borsh,
just pointer math into the 165-byte layout.

| Function | What it reads |
| --- | --- |
| `token_account_owner(account)` | Owner address (bytes 32..64) |
| `token_account_amount(account)` | Token balance as u64 (bytes 64..72) |
| `token_account_mint(account)` | Mint address (bytes 0..32) |
| `token_account_delegate(account)` | Optional delegate address |

```rust
let owner = token_account_owner(user_token)?;
require_keys_eq!(owner, authority.address(), ProgramError::InvalidArgument);

let amount = token_account_amount(user_token)?;
require_gte!(amount, min_collateral, MyError::Undercollateralized);
```

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
| `require_accounts_ne!(a, b, err)` | | Two accounts must have different addresses |
| `require_flag!(byte, n, err)` | | Bit `n` must be set in `byte` |

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

## Things you can't do in Anchor

Not a knock on Anchor. Different tool, different abstraction level. But once
you're at the pinocchio level, you lose some things. Jiminy puts them back.

### `assert_pda` - derive and verify with bump returned

Anchor derives PDAs behind proc macros. In pinocchio you're calling syscalls
manually and managing bumps yourself. `assert_pda` does the derivation, checks
the match, and hands you the bump for storage or CPI signing.

```rust
let bump = assert_pda(vault, &[b"vault", authority.as_ref()], program_id)?;
// bump is ready for CPI or storage
```

If you already have the bump (read it from account data), use `assert_pda_with_bump`
to skip the search and save ~1500 CU per bump iteration avoided.

### Token account reads without borsh

Need to check a token account's owner or balance? In Anchor you deserialize
the whole thing. Here you just read the bytes you need.

```rust
let owner = token_account_owner(user_token)?;
let amount = token_account_amount(user_token)?;
let mint = token_account_mint(user_token)?;
```

Zero-copy reads from the 165-byte SPL layout. No alloc, no schema.

### `SliceCursor` - field reads without the arithmetic

Reading fields from raw account data in pinocchio means keeping byte offsets
in your head. `SliceCursor` tracks the position for you:

```rust
let data = account.try_borrow()?;
let mut cur = SliceCursor::new(&data[1..]); // skip discriminator
let balance   = cur.read_u64()?;
let authority = cur.read_address()?;
let is_locked = cur.read_bool()?;
```

Run off the end of the buffer and you get `AccountDataTooSmall`, not a panic.

### `DataWriter` - same thing for writes

```rust
let mut raw = new_account.try_borrow_mut()?;
write_discriminator(&mut raw, VAULT_DISC)?;
let mut w = DataWriter::new(&mut raw[1..]);
w.write_u64(0)?;               // initial balance
w.write_address(&authority)?;  // 32-byte authority key
```

### `require_accounts_ne!` - source != destination

Classic token program bug: same account as source and dest. Anchor doesn't
have a built-in for this.

```rust
require_accounts_ne!(source_vault, dest_vault, MyError::SameAccount);
```

### `assert_not_initialized` - the account shouldn't exist yet

Different from `check_uninitialized` (which checks empty data). This checks
lamports == 0, meaning the account hasn't been funded on-chain. Useful for
create-if-not-exists patterns.

```rust
assert_not_initialized(new_vault)?;
```

---

## Compared to the alternatives

|  | Raw pinocchio | Anchor | **Jiminy** |
| --- | --- | --- | --- |
| Allocator required | No | Yes | No |
| Borsh required | No | Yes | No |
| Proc macros | No | Yes | No |
| Account validation | Manual | `#[account(...)]` constraints | Functions + macros |
| PDA derivation + bump | Manual syscall | `seeds + bump` constraint | `assert_pda` returns bump |
| Token account reads | Manual offset math | `Account<'info, TokenAccount>` | `token_account_owner/amount/mint` |
| Data reads | Manual index arithmetic | `Account<'info, T>` + borsh | `SliceCursor` |
| Data writes | Manual index arithmetic | Automatic via borsh | `DataWriter` |
| Overflow-safe math | Manual | Built-in | `checked_add/sub/mul` |
| Source != dest guard | Manual | Not built-in | `require_accounts_ne!` |
| Token program check | Manual | `Program<Token>` | `assert_token_program` (handles both SPL + 2022) |
| Existence check | Manual | Not built-in | `assert_not_initialized` |

Anchor isn't bad. But once you're at the pinocchio level, you shouldn't have to
give up composable safety primitives.

---

## Used in SHIPyard

Jiminy powers the on-chain program registry in
[SHIPyard](https://github.com/BluefootLabs/SHIPyard), a platform for building,
deploying, and sharing Solana programs. The code generator targets Jiminy as a
framework option.

---

## Account Layout Convention

Jiminy ships an opinionated [Account Layout v1](docs/LAYOUT_CONVENTION.md)
convention: an 8-byte header with discriminator, version, flags, and
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
at `SolanaDevDao.sol` - it goes toward keeping development going.

---

## License

Apache-2.0. See [LICENSE](LICENSE).

pinocchio is also Apache-2.0: [anza-xyz/pinocchio](https://github.com/anza-xyz/pinocchio).
Apache wrapping Apache, all the way down.
