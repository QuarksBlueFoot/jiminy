# jiminy

[![crates.io](https://img.shields.io/crates/v/jiminy.svg)](https://crates.io/crates/jiminy)
[![docs.rs](https://docs.rs/jiminy/badge.svg)](https://docs.rs/jiminy)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Pinocchio is the engine. Jiminy keeps it honest.**

You're writing Solana programs with [pinocchio](https://github.com/anza-xyz/pinocchio).
No allocator, no borsh, raw bytes, full control. Fastest thing on the network. But
every instruction ends up with the same wall of boilerplate: signer? owner?
discriminator? overflow math? PDA derivation? You copy-paste it, something slips,
you get rekt.

Jiminy is a complete safety toolkit that sits on top of pinocchio. Composable check
functions, PDA assertions that return bumps, zero-copy token + mint readers,
Token-2022 extension screening, CPI reentrancy guards, DeFi math with u128
intermediates, slippage protection, time/deadline checks, state machine validation,
and more. All `#[inline(always)]`, all `no_std`, all BPF-safe.

You're still writing pinocchio. You're just not writing the boring (and dangerous)
parts by hand anymore.

**No allocator. No borsh. No proc macros. No compromises.**

The [benchmarks](#benchmarks) show 7-14 CU overhead per instruction and a smaller
binary than hand-rolled pinocchio. Not a typo.

---

## Install

```toml
[dependencies]
jiminy = "0.4"
```

## Adding Jiminy to an existing Pinocchio project

Already using pinocchio directly? You have two options:

### Option 1: Keep both dependencies

```toml
[dependencies]
pinocchio = "0.10"
jiminy = "0.4"
```

This works fine. Cargo deduplicates the pinocchio crate as long as versions are
compatible. You keep your existing `use pinocchio::*` imports and add jiminy
imports alongside them.

### Option 2: Drop the direct pinocchio dependency (recommended)

```toml
[dependencies]
jiminy = "0.4"
```

Jiminy re-exports the entire pinocchio crate. Replace your pinocchio imports:

```rust
// Before
use pinocchio::{AccountView, Address, ProgramResult};
use pinocchio::{program_entrypoint, no_allocator, nostd_panic_handler};

// After
use jiminy::pinocchio::{AccountView, Address, ProgramResult};
use jiminy::pinocchio::{program_entrypoint, no_allocator, nostd_panic_handler};

// Or just use the prelude for the most common types
use jiminy::prelude::*;
```

The `pub use pinocchio;` re-export in `lib.rs` makes the **entire** pinocchio
API available under `jiminy::pinocchio`, so there's no need for a direct
dependency. One crate, one version, no duplication.

---

## Quick Start

```rust
use jiminy::prelude::*;
```

One import. You get everything: account checks, token readers, mint readers,
Token-2022 extension screening, CPI guards, DeFi math, slippage checks,
time validation, state machines, cursors, macros, `AccountList`, and the
pinocchio core types.

---

## A real example

```rust
use jiminy::{
    check_account, check_signer, check_writable,
    checked_sub, check_slippage, check_not_expired,
    require_accounts_ne, token_account_amount,
    AccountList, SliceCursor,
};

fn process_swap(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let user        = accs.next_signer()?;
    let pool        = accs.next_writable_account(program_id, POOL_DISC, POOL_LEN)?;
    let user_in     = accs.next_token_account(&token_a_mint, user.address())?;
    let user_out    = accs.next_token_account(&token_b_mint, user.address())?;
    let clock       = accs.next_clock()?;

    require_accounts_ne!(user_in, user_out, ProgramError::InvalidArgument);

    let mut ix = SliceCursor::from_instruction(instruction_data, 17)?;
    let amount_in   = ix.read_u64()?;
    let min_out     = ix.read_u64()?;
    let deadline    = ix.read_i64()?;  // user-set expiry

    // Time check: reject stale transactions
    let (_, now) = read_clock(clock)?;
    check_not_expired(now, deadline)?;

    // ... compute swap output ...
    let amount_out = compute_output(amount_in, &pool_data)?;

    // Slippage: the single most important DeFi check
    check_slippage(amount_out, min_out)?;

    // ... execute transfers ...
    Ok(())
}
```

---

## What's in the box

### Account validation

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
| `check_rent_exempt(account)` | `rent_exempt` | Must hold enough lamports for rent exemption |
| `check_lamports_gte(account, min)` | `constraint` | Must hold at least `min` lamports |
| `check_closed(account)` | `close` | Must have zero lamports and empty data |
| `check_account(account, id, disc, len)` | composite | Owner + size + discriminator in one call |
| `check_accounts_unique_2(a, b)` | -- | Two accounts have different addresses |
| `check_accounts_unique_3(a, b, c)` | -- | Three accounts all different (src != dest != fee) |
| `check_instruction_data_len(data, n)` | -- | Exact instruction data length |
| `check_instruction_data_min(data, n)` | -- | Minimum instruction data length |
| `check_version(data, min)` | -- | Header version byte >= minimum |

### Assert functions

These derive, compare, and return useful data. Not just pass/fail.

| Function | What it does |
| --- | --- |
| `assert_pda(account, seeds, program_id)` | Derive PDA, verify match, **return bump** |
| `assert_pda_with_bump(account, seeds, bump, id)` | Verify PDA with known bump (way cheaper) |
| `assert_pda_external(account, seeds, id)` | Same as `assert_pda` for external program PDAs |
| `assert_token_program(account)` | Must be SPL Token *or* Token-2022 |
| `assert_address(account, expected)` | Account address must match exactly |
| `assert_program(account, expected)` | Address match + must be executable |
| `assert_not_initialized(account)` | Lamports == 0 (account doesn't exist yet) |

### AccountList -- iterator-style account consumption

Stop hand-indexing `accounts[0]`, `accounts[1]`. `AccountList` gives you
named, validated accounts in order with inline constraint checks:

```rust
let mut accs = AccountList::new(accounts);
let payer         = accs.next_writable_signer()?;
let vault         = accs.next_writable_account(program_id, VAULT_DISC, VAULT_LEN)?;
let user_token    = accs.next_token_account(&usdc_mint, user.address())?;
let mint          = accs.next_mint(&programs::TOKEN)?;
let token_program = accs.next_executable()?;
let clock         = accs.next_clock()?;
let sysvar_ix     = accs.next_sysvar_instructions()?;
```

Each method consumes one account and runs the appropriate checks. Runs out of
accounts? You get `NotEnoughAccountKeys`, not a panic.

### Token account readers + checks

Zero-copy reads from the 165-byte SPL Token layout. No deserialization.

| Function | What it reads / checks |
| --- | --- |
| `token_account_owner(account)` | Owner address (bytes 32..64) |
| `token_account_amount(account)` | Token balance as u64 (bytes 64..72) |
| `token_account_mint(account)` | Mint address (bytes 0..32) |
| `token_account_delegate(account)` | Optional delegate address |
| `token_account_state(account)` | State byte (0=uninit, 1=init, 2=frozen) |
| `token_account_close_authority(account)` | Optional close authority |
| `token_account_delegated_amount(account)` | Delegated amount (u64) |
| `check_token_account_mint(account, mint)` | Mint matches expected -- **#1 most exploited missing check** |
| `check_token_account_owner(account, owner)` | Owner matches expected |
| `check_token_account_initialized(account)` | State == 1 |
| `check_no_delegate(account)` | No active delegate (prevents fund pulling) |
| `check_no_close_authority(account)` | No close authority set |
| `check_token_balance_gte(account, min)` | Token balance >= minimum |
| `check_token_program_match(account, prog)` | Account owned by the right token program |

### Mint account readers + checks

Same zero-copy approach for the 82-byte SPL Mint layout.

| Function | What it reads / checks |
| --- | --- |
| `mint_authority(account)` | Optional mint authority address |
| `mint_supply(account)` | Total supply (u64) |
| `mint_decimals(account)` | Decimals (u8) |
| `mint_is_initialized(account)` | Is initialized (bool) |
| `mint_freeze_authority(account)` | Optional freeze authority |
| `check_mint_owner(account, token_prog)` | Mint owned by expected token program |
| `check_mint_authority(account, expected)` | Mint authority matches |

### Token-2022 extension screening

Programs accepting Token-2022 tokens **must** screen for dangerous extensions.
Ignoring transfer fees, hooks, or permanent delegates is a critical vulnerability.
Jiminy gives you a full TLV extension reader and one-line safety guards:

```rust
let data = mint_account.try_borrow()?;

// Nuclear option: reject all dangerous extensions at once
check_safe_token_2022_mint(&data)?;

// Or check individually
check_no_transfer_fee(&data)?;
check_no_transfer_hook(&data)?;
check_no_permanent_delegate(&data)?;
check_not_non_transferable(&data)?;
check_no_default_account_state(&data)?;

// Need to actually handle transfer fees? Read the config.
if let Some(config) = read_transfer_fee_config(&data)? {
    let fee = calculate_transfer_fee(amount, &config.older_transfer_fee);
    let net = checked_sub(amount, fee)?;
}
```

Also: `find_extension`, `has_extension`, `check_no_extension`, `check_token_program_for_mint`,
and the full `ExtensionType` enum covering all 24 known extension types.

### CPI reentrancy protection

Reentrancy on Solana works differently than on EVM, but it's still real. A
malicious program can invoke your instruction via CPI to exploit intermediate
state. Neither Anchor nor Pinocchio ships a guard for this.

```rust
let sysvar_ix = accs.next_sysvar_instructions()?;

// Reject if we were called via CPI (top-level only)
check_no_cpi_caller(sysvar_ix, program_id)?;

// Or verify the CPI caller is a trusted router
check_cpi_caller(sysvar_ix, &TRUSTED_ROUTER)?;
```

Reads the Sysvar Instructions account to inspect the instruction stack.
Zero runtime overhead beyond the sysvar read.

### DeFi math

Every AMM, vault, and lending protocol needs the same math primitives.
Without u128 intermediates, `amount * price` overflows for any token amount
above ~4.2B. Jiminy handles it:

| Function | What it does |
|---|---|
| `checked_add(a, b)` | Overflow-safe u64 addition |
| `checked_sub(a, b)` | Underflow-safe u64 subtraction |
| `checked_mul(a, b)` | Overflow-safe u64 multiplication |
| `checked_div(a, b)` | Division with zero check |
| `checked_div_ceil(a, b)` | Ceiling division (fees should never round to zero) |
| `checked_mul_div(a, b, c)` | `(a * b) / c` with u128 intermediate (floor) |
| `checked_mul_div_ceil(a, b, c)` | Same, ceiling (protocol-side fee math) |
| `bps_of(amount, bps)` | Basis point fee: `amount * bps / 10_000` |
| `bps_of_ceil(amount, bps)` | Same, ceiling |
| `checked_pow(base, exp)` | Exponentiation via repeated squaring |
| `to_u64(val)` | Safe u128 -> u64 narrowing |

### Slippage + economic bounds

| Function | What it does |
|---|---|
| `check_slippage(actual, min_output)` | **The #1 DeFi check.** Reject sandwich attacks. |
| `check_max_input(actual, max_input)` | Exact-output swap: input doesn't exceed max |
| `check_min_amount(amount, min)` | Anti-dust: reject economically meaningless ops |
| `check_max_amount(amount, max)` | Exposure limit per operation |
| `check_nonzero(amount)` | Zero-amount transfers/swaps are always a bug |
| `check_within_bps(actual, expected, tol)` | Oracle deviation check (u128 intermediate) |
| `check_price_bounds(price, min, max)` | Circuit breaker for price feeds |

### Time + deadline checks

| Function | What it does |
|---|---|
| `check_not_expired(now, deadline)` | Current time <= deadline |
| `check_expired(now, deadline)` | Current time > deadline (for claims/settlements) |
| `check_within_window(now, start, end)` | Time is within [start, end] (auction windows) |
| `check_cooldown(last, cooldown, now)` | Rate limiting (oracle updates, admin changes) |
| `check_deadline(clock, deadline)` | Combined: read Clock sysvar + check not expired |
| `check_after(clock, deadline)` | Combined: read Clock sysvar + check expired |

### Sysvar readers

Zero-copy readers for Clock and Rent. No deserialization, just offset reads.

```rust
let clock = accs.next_clock()?;
let (slot, timestamp) = read_clock(clock)?;
let epoch = read_clock_epoch(clock)?;
```

### State machine validation

DeFi programs are state machines: orders go Open -> Filled -> Settled,
escrows go Pending -> Released -> Disputed. State transitions need to be
validated, not just checked.

```rust
const TRANSITIONS: &[(u8, u8)] = &[
    (ORDER_OPEN, ORDER_FILLED),
    (ORDER_OPEN, ORDER_CANCELLED),
    (ORDER_FILLED, ORDER_SETTLED),
];

let data = order.try_borrow()?;
check_state(&data, STATE_OFFSET, ORDER_OPEN)?;
check_state_transition(ORDER_OPEN, ORDER_FILLED, TRANSITIONS)?;

let mut data = order.try_borrow_mut()?;
write_state(&mut data, STATE_OFFSET, ORDER_FILLED)?;
```

Also: `check_state_not`, `check_state_in` (multiple valid states).

### PDA utilities

| Macro / Function | What it does |
| --- | --- |
| `find_pda!(program_id, seed1, seed2, ...)` | Find canonical PDA + bump via syscall |
| `derive_pda!(program_id, bump, seed1, ...)` | Derive PDA with known bump (~100 CU) |
| `derive_pda_const!(id_bytes, bump, seed1, ...)` | Compile-time PDA derivation |
| `derive_ata(wallet, mint)` | Derive ATA address + bump |
| `derive_ata_with_program(wallet, mint, token_prog)` | ATA with explicit token program |
| `derive_ata_with_bump(wallet, mint, bump)` | ATA with known bump (cheap) |
| `derive_ata_const!(wallet_bytes, mint_bytes, bump)` | Compile-time ATA derivation |

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
| `require_accounts_ne!(a, b, err)` | -- | Two accounts must have different addresses |
| `require_flag!(byte, n, err)` | -- | Bit `n` must be set in `byte` |

### Cursors

| Type / Function | What it does |
|---|---|
| `SliceCursor` | Position-tracked, bounds-checked reads from `&[u8]` |
| `DataWriter` | Position-tracked, bounds-checked writes to `&mut [u8]` |
| `SliceCursor::from_instruction(data, min_len)` | Cursor with upfront length validation |
| `zero_init(data)` | Zero-fill before writing (prevents stale-data bugs) |
| `write_discriminator(data, disc)` | Write type tag byte |

Supports: `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`,
`bool`, `Address`. All little-endian.

### Account lifecycle

| Function | What it does |
|---|---|
| `safe_close(account, destination)` | Move all lamports + close atomically |

### Well-known program IDs

```rust
use jiminy::programs;

programs::SYSTEM             // 11111111111111111111111111111111
programs::TOKEN              // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
programs::TOKEN_2022         // TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
programs::ASSOCIATED_TOKEN   // ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
programs::METADATA           // metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s
programs::SYSVAR_CLOCK       // SysvarC1ock11111111111111111111111111111111
programs::SYSVAR_RENT        // SysvarRent111111111111111111111111111111111
programs::SYSVAR_INSTRUCTIONS // Sysvar1nstructions1111111111111111111111111
```

---

## Things that don't exist in Anchor (or Pinocchio)

### CPI reentrancy guard

Neither Anchor nor Pinocchio ships a built-in reentrancy check. Jiminy reads the
Sysvar Instructions account to detect whether your instruction was invoked
directly by the transaction or via CPI from another program. One function call.

### Token-2022 extension screening

Anchor deserializes token accounts but doesn't screen extensions. A mint with a
permanent delegate can drain your vault. A transfer hook can make your CPI fail.
Jiminy's `check_safe_token_2022_mint` rejects all commonly dangerous extensions
in a single call, or you can check them individually.

### Slippage + economic guards

`check_slippage`, `check_within_bps`, `check_price_bounds`. DeFi primitives
that are missing from both Anchor and Pinocchio. Every swap needs slippage
protection. Every oracle read needs a staleness/deviation check. These should
be one-liners, not hand-rolled math with off-by-one risks.

### U128 intermediate math

`checked_mul_div` and `bps_of` use u128 intermediates to prevent overflow.
Without this, `amount * price` overflows at ~4.2 billion tokens. Anchor's
checked math doesn't do u128 promotion. This is the #1 numerical footgun
in DeFi programs.

### State machine transitions

`check_state_transition` validates (from, to) pairs against a transition table.
No more `if state == X && next_state == Y || state == X && next_state == Z`.
Define your transitions as a const table, validate in one call.

### Source != destination guard

`check_accounts_unique_2` and `check_accounts_unique_3`. Anchor doesn't have a
built-in for this. Same-account-as-source-and-dest is a classic token program
exploit vector.

---

## Compared to the alternatives

|  | Raw pinocchio | Anchor | **Jiminy** |
| --- | --- | --- | --- |
| Allocator required | No | Yes | **No** |
| Borsh required | No | Yes | **No** |
| Proc macros | No | Yes | **No** |
| Account validation | Manual | `#[account(...)]` | Functions + macros |
| Token account reads | Manual offsets | Borsh deser | Zero-copy readers |
| Mint account reads | Manual offsets | Borsh deser | Zero-copy readers |
| Token-2022 screening | Manual | Not built-in | `check_safe_token_2022_mint` |
| CPI reentrancy guard | Manual | Not built-in | `check_no_cpi_caller` |
| Slippage protection | Manual | Not built-in | `check_slippage` |
| DeFi math (u128) | Manual | Not built-in | `checked_mul_div` / `bps_of` |
| State machine checks | Manual | Not built-in | `check_state_transition` |
| Time/deadline checks | Manual | Not built-in | `check_not_expired` / `check_cooldown` |
| Source != dest guard | Manual | Not built-in | `check_accounts_unique_2` |
| PDA derivation + bump | Manual syscall | `seeds + bump` constraint | `assert_pda` / `find_pda!` / `derive_pda!` |
| Data reads/writes | Manual index math | Borsh | `SliceCursor` / `DataWriter` |

Anchor is great for what it does. But once you're at the pinocchio level, you
shouldn't have to give up safety primitives. Jiminy gives you more checks than
Anchor provides out of the box, with zero runtime overhead.

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
| Deposit     | 147 CU    | 154 CU | +7    |
| Withdraw    | 254 CU    | 266 CU | +12   |
| Close       | 215 CU    | 228 CU | +13   |
| Guarded Withdraw | 567 CU | 581 CU | +14 |

**Guarded Withdraw** exercises the new DeFi safety modules: `check_nonzero`,
`check_min_amount`, `check_accounts_unique_3`, `check_instruction_data_min`,
and `checked_mul_div` for a 0.3% fee calculation.

### Security Demo: Missing Signer Check

The benchmark includes a `vuln_withdraw` that "forgot" the `is_signer()` check.
An attacker reads a real user's vault on-chain, passes the stored authority pubkey
(unsigned) and the real vault, and calls withdraw. All other checks pass -- the
vault IS owned by the program. **2 SOL drained.**

| Program | CU | Result |
|---------|----|--------|
| Pinocchio | 211 CU | **EXPLOITED** -- attacker drains 2 SOL |
| Jiminy    |  78 CU | **SAFE** -- `next_signer()` rejects unsigned authority |

In Jiminy, the signer check is bundled into `accs.next_signer()` -- there's no
separate line to forget.

### Binary Size (release SBF)

| Program | Size |
|---------|------|
| Pinocchio vault | 27.4 KB |
| Jiminy vault    | 26.5 KB |

Jiminy adds **7-14 CU** of overhead per instruction (a single `sol_log` costs
~100 CU). The binary is **0.9 KB smaller** thanks to pattern deduplication
from `AccountList` and the check functions.

See [BENCHMARKS.md](BENCHMARKS.md) for full details and instructions to run
them yourself.

---

## Reference Programs

| Program | What it demonstrates |
|---------|---------------------|
| [`examples/jiminy-vault`](examples/jiminy-vault) | Init / deposit / withdraw / close with `AccountList`, cursors, `safe_close` |
| [`examples/jiminy-escrow`](examples/jiminy-escrow) | Two-party escrow, flag-based state, `check_closed`, ordering guarantees |

Both use the Jiminy Header v1 layout. Fork them as starting templates.

---

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some debugging time, donations welcome at `SolanaDevDao.sol`.

---

## License

Apache-2.0. See [LICENSE](LICENSE).

pinocchio is also Apache-2.0: [anza-xyz/pinocchio](https://github.com/anza-xyz/pinocchio).
