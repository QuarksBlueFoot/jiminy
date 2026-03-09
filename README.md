# jiminy

[![crates.io](https://img.shields.io/crates/v/jiminy.svg)](https://crates.io/crates/jiminy)
[![docs.rs](https://docs.rs/jiminy/badge.svg)](https://docs.rs/jiminy)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**The zero-copy standard library for Solana programs.**

*Every pinocchio needs a conscience.*

You chose pinocchio because you wanted raw performance and full control over
your on-chain program. No allocator, no borsh, no framework opinions. Just
bytes.

But here's the thing: you're still writing the same signer check for the tenth
time. You're still hand-rolling `amount * price / 10_000` and praying it doesn't
overflow at 4.2 billion tokens. You're still manually walking the Token-2022 TLV
to screen for permanent delegates. And every check you forget to write is one
exploit away from draining your vault.

jiminy is the standard library that pinocchio doesn't ship.

It's every guard, check, reader, and piece of math that DeFi programs need,
packaged as plain functions and declarative macros. `no_std`, `no_alloc`,
no proc macros, every function `#[inline(always)]`, pinocchio under the hood.
The [benchmarks](#benchmarks) show 7-14 CU overhead per instruction and a
binary that's actually *smaller* than hand-rolled pinocchio. Not a typo.

One import and you get:

- **Account validation** -- signer, owner, discriminator, PDA, rent, uniqueness
- **Zero-copy IO** -- cursors, readers, writers, pod overlays, `zero_copy_layout!`
- **Token reads** -- SPL Token account/mint fields without deserialization
- **Token-2022 screening** -- reject transfer fees, hooks, permanent delegates in one call
- **CPI wrappers** -- system, token, mint-to, burn, close, with built-in validation
- **Reentrancy guards** -- detect CPI callers via sysvar introspection
- **DeFi math** -- u128 intermediates, basis points, decimal scaling, overflow-safe
- **AMM curves** -- constant-product swaps, LP math, K-invariant checks
- **Slippage + bounds** -- the single most important DeFi check, as a one-liner
- **Time + deadlines** -- expiry, cooldowns, staleness, slot-based checks
- **State machines** -- transition tables, not nested `if` chains
- **Oracles** -- Pyth V2 zero-copy reads, TWAP accumulators
- **Lending** -- collateralization, liquidation, interest rate math
- **Staking** -- reward-per-token accumulators with u128 precision
- **Vesting** -- linear, cliff, stepped, periodic unlock schedules
- **Multisig** -- M-of-N threshold with duplicate-signer prevention
- **Distribution** -- dust-safe splits where `sum(parts) == total` is guaranteed
- **Merkle proofs** -- on-chain verification via `sol_sha256` syscall
- **Ed25519** -- precompile signature verification from sysvar data
- **Events** -- zero-alloc `emit!` via `sol_log_data`, no borsh
- **Tx introspection** -- flash-loan detection, composition guards, compute budget

All the stuff you'd write yourself. Except audited, tested, and you don't
have to maintain it.

```rust
use jiminy::prelude::*;
```

**No allocator. No borsh. No proc macros. Ship faster.**

---

## New in 0.12

- **`zero_copy_layout!`** macro: declare `#[repr(C)]` structs that overlay directly onto account bytes. No proc macros, no borsh, just typed fields at byte offsets.
- **`ZeroCopySlice` / `ZeroCopySliceMut`**: length-prefixed `[u32][T; len]` arrays in account data. Zero-copy iteration, random access, mutation. `init()` for first write.
- **`pod_read<T>()`**: alignment-safe owned copy via `read_unaligned`. Works on all targets, great for native tests.
- **Syscall-based sysvar access**: `clock_timestamp()`, `clock_slot()`, `clock_epoch()`, `rent_lamports_per_byte_year()`. No account slot needed. Just call and get the value.
- **Instruction deduplication**: `jiminy-solana`'s `compose` and `introspect` modules now delegate to `jiminy-core::instruction` instead of reimplementing the same parsing logic.

---

## Architecture

Nine crates, three layers. Use `jiminy` for everything or pull in just the
layer you need.

```text
┌──────────────────────────────────────────────────────────────┐
│  jiminy  (umbrella - pure re-exports, zero local code)       │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Ring 3+  jiminy-finance · jiminy-lending · staking    │  │
│  │           jiminy-vesting · multisig · distribute       │  │
│  │                                                        │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │  Ring 2   jiminy-solana                          │  │  │
│  │  │           token/ · cpi/ · crypto/ · oracle ···   │  │  │
│  │  │                                                  │  │  │
│  │  │  ┌────────────────────────────────────────────┐  │  │  │
│  │  │  │  Ring 1   jiminy-core                      │  │  │  │
│  │  │  │           account/ · check/ · math · ···   │  │  │  │
│  │  │  └────────────────────────────────────────────┘  │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### Ring 1 - Systems layer (`jiminy-core`)

| Module | |
|---|---|
| `account` | Header, reader, writer, cursor, lifecycle, pod, overlay, collection, list, bits |
| `check` | Validation checks, asserts, PDA derivation & verification |
| `instruction` | Transaction introspection, composition guards, flash-loan detection |
| `math` | Checked arithmetic, BPS, scaling (u128 intermediates) |
| `sysvar` | Clock and Rent sysvar readers (syscall-based + account-based) |
| `state` | State machine transition checks |
| `time` | Deadline, cooldown, staleness checks |
| `event` | Zero-alloc event emission via `sol_log_data` |
| `programs` | Well-known program IDs *(feature: `programs`)* |

### Ring 2 - Platform helpers (`jiminy-solana`)

| Module | |
|---|---|
| `token` | SPL Token account/mint readers, Token-2022 extension screening |
| `cpi` | Safe CPI wrappers, reentrancy guards, return data readers |
| `crypto` | Ed25519 precompile verification, Merkle proof verification |
| `authority` | Two-step authority rotation (propose + accept) |
| `balance` | Pre/post CPI balance delta guards |
| `compute` | Compute budget guards |
| `compose` | Transaction composition guards (flash-loan detection) |
| `introspect` | Raw transaction introspection |
| `oracle` | Pyth V2 price feed readers (zero external deps) |
| `twap` | TWAP accumulators |
| `upgrade` | Program upgrade authority verification *(feature: `programs`)* |

### Ring 3+ - Protocol math & domain crates

| Crate | |
|---|---|
| `jiminy-finance` | AMM math, constant-product swaps, slippage & economic bounds |
| `jiminy-lending` | Lending protocol primitives (collateralization, liquidation, interest) |
| `jiminy-staking` | Staking reward accumulators (MasterChef-style) |
| `jiminy-vesting` | Vesting schedule helpers (linear, cliff, stepped, periodic) |
| `jiminy-multisig` | M-of-N multi-signer threshold checks |
| `jiminy-distribute` | Dust-safe proportional distribution & fee extraction |

---

## Install

```toml
# Full toolkit (recommended)
[dependencies]
jiminy = "0.12"

# Or pick individual crates for minimal deps
jiminy-core = "0.12"      # Account layout, checks, math, PDA
jiminy-solana = "0.12"    # Token, CPI, crypto, oracle
jiminy-finance = "0.12"   # AMM math, slippage
```

## Adding Jiminy to an existing Pinocchio project

Already using pinocchio directly? You have two options:

### Option 1: Keep both dependencies

```toml
[dependencies]
pinocchio = "0.10"
jiminy = "0.12"
```

This works fine. Cargo deduplicates the pinocchio crate as long as versions are
compatible. You keep your existing `use pinocchio::*` imports and add jiminy
imports alongside them.

### Option 2: Drop the direct pinocchio dependency (recommended)

```toml
[dependencies]
jiminy = "0.12"
```

Jiminy re-exports the entire pinocchio crate, plus `pinocchio-system` and
`pinocchio-token`. Replace your pinocchio imports:

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
dependency. Same for `jiminy::pinocchio_system` and `jiminy::pinocchio_token`.
One crate, one version, no duplication.

The prelude also re-exports the most common CPI structs:

```rust
use jiminy::prelude::*;

// System program CPI - no more hand-rolling 52-byte instruction data
CreateAccount {
    from: payer,
    to: new_account,
    lamports,
    space: 128,
    owner: program_id,
}
.invoke()?;

// Token program CPI
TokenTransfer {
    from: source_token,
    to: dest_token,
    authority: owner,
    amount: 1_000_000,
}
.invoke()?;
```

---

## Quick Start

```rust
use jiminy::prelude::*;
```

One import. Everything listed above lands in scope. Account checks, token
readers, CPI wrappers, DeFi math, macros, `AccountList`, cursors, and the
pinocchio core types. You don't need a direct pinocchio dependency anymore.

All public functions are available both via the prelude and through their
module paths (`jiminy::token::*`, `jiminy::cpi::*`, `jiminy::math::*`, etc.).

---

## A real example

```rust
use jiminy::prelude::*;

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
| `check_accounts_unique!(a, b, c, ...)` | -- | Variadic: all accounts have different addresses |
| `check_accounts_unique_2(a, b)` | -- | Two accounts have different addresses |
| `check_accounts_unique_3(a, b, c)` | -- | Three accounts all different (src != dest != fee) |
| `check_accounts_unique_4(a, b, c, d)` | -- | Four accounts all different (two-hop swaps) |
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
let token_program = accs.next_token_program()?;  // validates SPL Token or Token-2022
let any_token     = accs.next_writable_token_account(&usdc_mint, user.address())?;
let rent          = accs.next_rent()?;
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
| `check_token_account_mint(account, mint)` | Mint matches expected |
| `check_token_account_owner(account, owner)` | Owner matches expected |
| `check_token_account_initialized(account)` | State == 1 |
| `check_no_delegate(account)` | No active delegate (prevents fund pulling) |
| `check_no_close_authority(account)` | No close authority set |
| `check_token_balance_gte(account, min)` | Token balance >= minimum |
| `check_token_program_match(account, prog)` | Account owned by the right token program |
| `check_not_frozen(account)` | Reject frozen token accounts upfront |

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
| `scale_amount(amount, from, to)` | Decimal-aware token amount conversion (u128 intermediate) |
| `scale_amount_ceil(amount, from, to)` | Same, ceiling (protocol-side math) |

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
| `check_slot_staleness(last, current, max)` | Slot-based oracle/data feed staleness check |

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
| `check_ata(account, wallet, mint)` | Verify account is the canonical ATA |
| `check_ata_with_program(account, wallet, mint, prog)` | Same, for Token-2022 ATAs |

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
| `check_accounts_unique!(a, b, c)` | -- | Variadic uniqueness (any N accounts) |
| `error_codes! { base = 6000; ... }` | `#[error_code]` | Define numbered `ProgramError::Custom` codes |
| `instruction_dispatch! { ... }` | `#[program]` | Tag-byte dispatch to handler functions |
| `impl_pod!(T1, T2, ...)` | -- | Batch `unsafe impl Pod` for `#[repr(C)]` types |

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
| `safe_realloc(account, new_size, payer)` | Resize account + top up rent from payer |
| `safe_realloc_shrink(account, new_size, dest)` | Shrink account + return excess rent |
| `transfer_lamports(from, to, amount)` | Direct lamport transfer between program-owned accounts (no CPI) |

### Safe CPI wrappers

Bundle validation + invocation so you can't forget a writable or signer check
before issuing a CPI. All zero-copy, all `#[inline(always)]`.

| Function | What it does |
|---|---|
| `safe_create_account(payer, account, space, owner)` | System CPI: create account with rent-exempt balance |
| `safe_create_account_signed(payer, account, space, owner, signers)` | Same, with PDA signer seeds |
| `safe_transfer_sol(from, to, amount)` | System CPI: transfer SOL with nonzero check |
| `safe_transfer_tokens(from, to, authority, amount)` | Token CPI: SPL transfer with validation |
| `safe_transfer_tokens_signed(from, to, authority, amount, signers)` | Same, with PDA signer seeds |
| `safe_checked_transfer(from, to, auth, mint, from_owner, to_owner, amount)` | Paranoid transfer: mint + owner checks first |
| `safe_burn(account, mint, authority, amount)` | Token CPI: burn with validation |
| `safe_mint_to(mint, account, authority, amount)` | Token CPI: mint tokens with validation |
| `safe_mint_to_signed(mint, account, authority, amount, signers)` | Same, with PDA signer seeds |
| `safe_close_token_account(account, destination, authority)` | Token CPI: close account |

```rust
// One-liner CPI - checks signer, writable, nonzero for you
safe_transfer_tokens(source_ata, dest_ata, owner, amount)?;

// Paranoid transfer - also validates mint + owners match before CPI
safe_checked_transfer(
    source_ata, dest_ata, owner,
    &usdc_mint, source_wallet.address(), dest_wallet.address(),
    amount,
)?;

// Direct lamport transfer between program-owned PDAs (no CPI needed)
transfer_lamports(pool_pda, user_pda, withdrawal_amount)?;
```

### ATA validation

| Function | What it does |
|---|---|
| `check_ata(account, wallet, mint)` | Verify account is the canonical ATA |
| `check_ata_with_program(account, wallet, mint, token_prog)` | Same, for Token-2022 ATAs |

### Logging (opt-in)

Zero-alloc diagnostic logging behind the `log` feature flag. Uses the raw
`sol_log` syscall, no extra deps.

```toml
jiminy = { version = "0.12", features = ["log"] }
```

| Function | What it logs |
|---|---|
| `log_msg("text")` | Static message |
| `log_val("label", u64)` | Label + u64 value |
| `log_signed("label", i64)` | Label + signed value |
| `log_addr("label", &address)` | Label + first/last 4 bytes hex |
| `log_bool("label", bool)` | Label + Y/N |

### Well-known program IDs

```rust
use jiminy::programs;

programs::SYSTEM             // 11111111111111111111111111111111
programs::TOKEN              // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
programs::TOKEN_2022         // TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
programs::ASSOCIATED_TOKEN   // ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bTu
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

`check_accounts_unique!(a, b, c, ...)` (variadic macro) plus the original
`check_accounts_unique_2`, `check_accounts_unique_3`, and `check_accounts_unique_4`
functions.
Anchor's released versions (through 0.32.x) don't have a built-in for this.
The upcoming Anchor 1.0 adds duplicate mutable account validation at the
accounts-struct level, which guards against the same account occupying two
mutable positions. Jiminy's functions are explicit per-operation checks you
call inside instruction logic. Same-account-as-source-and-dest is a classic
token program exploit vector.

### Oracle staleness (slot-based)

`check_slot_staleness` compares the slot of the last oracle update against the
current slot. Stale prices lead to liquidation errors and arbitrage exploits.

### Decimal-aware amount scaling

`scale_amount` and `scale_amount_ceil` convert token amounts between different
decimal precisions using u128 intermediates. Comparing USDC (6 decimals) to
SOL (9 decimals) or pricing tokens with different precision is the most common
cross-mint arithmetic operation in DeFi. Getting it wrong leads to off-by-1000
bugs.

### Direct lamport transfer (no CPI)

`transfer_lamports` moves SOL directly between two program-owned accounts
without a system program CPI. Cheapest way to
move lamports between PDAs your program controls. No signer required,
no CPI overhead.

### Zero-alloc event emission

`emit!` and `emit_slices` write structured event data to the transaction log
via `sol_log_data`. Indexers (Helius, Triton, etc.) pick these up. Raw bytes
on the stack, single syscall, done.

```rust
let disc = [0x01u8]; // your event discriminator
let amt = amount.to_le_bytes();
emit!(&disc, user.address().as_ref(), &amt);
```

### Transaction introspection

`read_program_id_at`, `read_instruction_data_range`, `read_instruction_account_key`,
and `check_has_compute_budget`. Read any instruction in the current transaction
directly from Sysvar Instructions data. Verify transaction shape before
touching any state.

### Ed25519 precompile verification

`check_ed25519_signature` and `check_ed25519_signer`. Verify that an Ed25519
precompile instruction in the transaction was signed by an expected key over
an expected message. Used for gasless relayers, signed price feeds, off-chain
authorization flows. The runtime already did the crypto - you just need to
check it was the right signer and message. Zero-copy from the sysvar.

### Authority handoff (two-step rotation)

`check_pending_authority`, `write_pending_authority`, `accept_authority`. The
standard DeFi pattern for safe authority transfer: current authority proposes,
new authority accepts. Prevents fat-finger key transfers. Zero-copy reads at
byte offsets you define.

### Merkle proof verification

`verify_merkle_proof` and `sha256_leaf`. Verify merkle proofs using the native
`sol_sha256` syscall. Sorted pair hashing with domain separators (matches the
OpenZeppelin / SPL convention). Whitelists, airdrops, allowlists - all on the
stack, no alloc.

### Pyth oracle readers

`read_pyth_price`, `read_pyth_ema`, `check_pyth_price_fresh`, `check_pyth_confidence`.
Zero-copy Pyth V2 price feed reading at fixed byte offsets. No `pyth-sdk-solana`
dependency, no borsh, no alloc. Validates magic/version/account-type/status.
One function call replaces 6 crate dependencies.

```rust
let data = pyth_account.try_borrow()?;
let p = read_pyth_price(&data)?;
// p.price * 10^(p.expo) = human-readable price
check_pyth_price_fresh(p.publish_time, current_time, 30)?; // max 30s stale
check_pyth_confidence(p.price, p.conf, 5)?; // max 5% band
```

### AMM math

`isqrt`, `constant_product_out`, `constant_product_in`, `check_k_invariant`,
`price_impact_bps`, `initial_lp_amount`, `proportional_lp_amount`.

Integer square root via Newton's method for LP token minting. Constant-product
swap math with u128 intermediates and fee support. K-invariant verification
for post-swap safety. Price impact estimation. Every DEX needs these, no
pinocchio crate provides them.

```rust
let out = constant_product_out(reserve_a, reserve_b, amount_in, 30)?; // 30 bps fee
check_k_invariant(ra_before, rb_before, ra_after, rb_after)?;
let lp = isqrt(amount_a as u128 * amount_b as u128)?;
```

### Balance delta (safe swap composition)

`snapshot_token_balance`, `check_balance_increased`, `check_balance_decreased`,
`check_balance_delta`, `check_lamport_balance_increased`.

Read balance before CPI, execute
CPI, verify balance changed correctly after. Named functions make the pattern
auditor-visible.

```rust
let before = snapshot_token_balance(vault)?;
safe_transfer_tokens(...)?; // CPI into AMM
check_balance_increased(vault, before, min_output)?;
```

### Close revival sentinel

`safe_close_with_sentinel`, `check_not_revived`, `check_alive`.
Defends against Sealevel Attack #9: attacker revives a closed account within
the same transaction by transferring lamports back. The sentinel writes
`[0xFF; 8]` to the first 8 bytes before zeroing, so revived accounts are
detectable on re-entry.

```rust
safe_close_with_sentinel(vault, destination)?; // writes dead sentinel
// Later, in any instruction that accepts this account:
check_not_revived(vault)?;
```

### Staking rewards math

`update_reward_per_token`, `pending_rewards`, `update_reward_debt`,
`emission_rate`, `rewards_earned`.

Reward-per-token accumulator with u128 precision and
1e12 scaling factor. Getting the math wrong leads to reward theft or stuck
funds.

```rust
let new_rpt = update_reward_per_token(pool.rpt, new_rewards, pool.total_staked)?;
let claimable = pending_rewards(user.staked, new_rpt, user.reward_debt)?;
user.reward_debt = update_reward_debt(user.staked, new_rpt);
```

### Vesting schedules

`vested_amount`, `check_cliff_reached`, `unlocked_at_step`, `claimable`,
`elapsed_steps`.

Linear vesting with cliff, stepped/periodic unlocks, safe claimable
computation. Pure arithmetic for team tokens, investor unlocks, grant programs.

```rust
let vested = vested_amount(total_grant, start, cliff, end, now);
let claim = claimable(vested, user.already_claimed);
```

### Multi-signer threshold

`check_threshold`, `count_signers`, `check_all_signers`, `check_any_signer`.

M-of-N signature checking with built-in duplicate address prevention.
Prevents the duplicate-signer attack (same key passed in multiple account
slots to inflate the count).

```rust
check_threshold(&[admin_a, admin_b, admin_c], 2)?; // 2-of-3 multisig
```

### Compute budget guards

`remaining_compute_units`, `check_compute_remaining`, `require_compute_remaining`.

Wraps `sol_remaining_compute_units()` so batch-processing loops can bail
with a clean error instead of running into a wall. Also useful for adaptive
code paths that choose between expensive and cheap logic.

```rust
for item in items.iter() {
    check_compute_remaining(5_000)?; // need ~5K CU per item
    process(item)?;
}
```

### Transaction composition analysis

`check_no_other_invocation`, `check_no_subsequent_invocation`,
`detect_flash_loan_bracket`, `count_program_invocations`.

Higher-level introspection: detect flash-loan sandwiches, prevent specific
programs from appearing in the same transaction, count invocations. Builds
on `introspect` but answers structural questions about the whole tx.

```rust
let data = sysvar_ix.try_borrow()?;
let me = cpi::get_instruction_index(&data)?;
if detect_flash_loan_bracket(&data, me, &FLASH_LENDER)? {
    return Err(MyError::FlashLoanNotAllowed.into());
}
```

### CPI return data

`read_return_data`, `read_return_data_from`, `read_return_u64`.

Read values returned by a CPI callee via `sol_get_return_data`. Verify the
return data came from the expected program (prevents a malicious intermediary
from overwriting results).

```rust
let output_amount = read_return_u64(&SWAP_PROGRAM)?;
check_slippage(output_amount, user_min_output)?;
```

### Program upgrade verification

`read_upgrade_authority`, `check_program_immutable`, `check_upgrade_authority`.

Read BPF Upgradeable Loader state from a program's data account. Verify an
external program is frozen (immutable) or that a known governance key
controls upgrades before integrating with it.

```rust
check_program_immutable(amm_program_data)?;        // must be frozen
check_upgrade_authority(lend_data, &dao_multisig)?; // known upgrade auth
```

### TWAP accumulators

`update_twap_cumulative`, `compute_twap`, `check_twap_deviation`.

Time-weighted average price math. Maintain a cumulative price sum, compute
TWAP between any two observations, and check spot/TWAP deviation as an
anti-manipulation guard.

```rust
pool.cumulative = update_twap_cumulative(pool.cumulative, spot, pool.last_ts, now)?;
let twap = compute_twap(old.cumulative, new.cumulative, old.ts, new.ts)?;
check_twap_deviation(spot, twap, 500)?; // max 5% spread
```

### Lending math

`collateralization_ratio_bps`, `check_healthy`, `check_liquidatable`,
`max_liquidation_amount`, `liquidation_seize_amount`, `simple_interest`,
`utilization_rate_bps`.

Lending protocol primitives. All basis-point denominated, u128
intermediates, overflow-checked.

```rust
check_healthy(collateral_val, debt_val, 12_500)?; // 125% min
let max_repay = max_liquidation_amount(debt, 5_000)?; // 50% close factor
let seized = liquidation_seize_amount(repay, 500)?;   // 5% bonus
```

### Proportional distribution

`proportional_split`, `extract_fee`.

Dust-safe N-way splits where `sum(parts) == total` is guaranteed. Fee
extraction where `net + fee == amount` holds exactly. Uses the
largest-remainder method for the integer division leftovers.

```rust
let shares = [50u64, 30, 20];
let mut amounts = [0u64; 3];
proportional_split(1_000_003, &shares, &mut amounts)?;
// amounts sums to exactly 1_000_003

let (net, fee) = extract_fee(1_000_000, 30, 1_000)?; // 0.3% + 1000 flat
assert_eq!(net + fee, 1_000_000);
```

---

## Modular crates (v0.12+)

Starting with v0.11, Jiminy is split into focused crates. v0.12 adds
`zero_copy_layout!`, `ZeroCopySlice`, `pod_read`, and syscall-based
sysvar access.

```toml
# Full toolkit - zero local code, re-exports everything
jiminy = "0.12"

# Or pick what you need
jiminy-core = "0.12"        # Account layout, checks, math, PDA, sysvar
jiminy-solana = "0.12"      # Token, CPI, crypto, oracle, introspection
jiminy-finance = "0.12"     # AMM math, slippage
jiminy-lending = "0.12"     # Lending/liquidation primitives
jiminy-staking = "0.12"     # Reward accumulators
jiminy-vesting = "0.12"     # Vesting schedules
jiminy-multisig = "0.12"    # M-of-N threshold
jiminy-distribute = "0.12"  # Dust-safe splits
```

The root `jiminy` crate re-exports everything. Module paths are unchanged:

```rust
// These all still work
use jiminy::prelude::*;
use jiminy::token::token_account_amount;
use jiminy::cpi::safe_transfer_tokens;
use jiminy::math::checked_mul_div;

// Or use subcrates directly for minimal dependency trees
use jiminy_core::check::check_signer;
use jiminy_solana::crypto::verify_merkle_proof;
```

### Migration from 0.10

The API is the same. If you depend on `jiminy = "0.10"`, upgrade
to `"0.12"` and everything compiles. 0.12 adds new functions and macros
but nothing was removed or renamed.

---

## Compared to the alternatives

|  | Raw pinocchio | Anchor | **Jiminy** |
| --- | --- | --- | --- |
| Allocator required | No | Yes | **No** |
| Borsh required | No | Yes | **No** |
| Proc macros | No | Yes | **No** (declarative macros give you the same benefits) |
| Account validation | Manual | `#[account(...)]` | Functions + macros (`error_codes!`, `instruction_dispatch!`) |
| System CPI | Manual bytes | `system_program::create_account` | `CreateAccount { .. }.invoke()` |
| Token CPI | Manual bytes | Anchor SPL | `TokenTransfer { .. }.invoke()` |
| Token account reads | Manual offsets | Borsh deser | Zero-copy readers + check functions |
| Mint account reads | Manual offsets | Borsh deser | Zero-copy readers + check functions |
| Token-2022 screening | Manual | Not built-in | `check_safe_token_2022_mint` |
| CPI reentrancy guard | Manual | Not built-in | `check_no_cpi_caller` |
| Slippage protection | Manual | Not built-in | `check_slippage` |
| DeFi math (u128) | Manual | Not built-in | `checked_mul_div` / `bps_of` |
| Decimal scaling | Manual | Not built-in | `scale_amount` / `scale_amount_ceil` |
| State machine checks | Manual | Not built-in | `check_state_transition` |
| Time/deadline checks | Manual | Not built-in | `check_not_expired` / `check_cooldown` |
| Oracle staleness | Manual | Not built-in | `check_slot_staleness` |
| Source != dest guard | Manual | Not built-in* | `check_accounts_unique!(a, b, c)` |
| Direct lamport xfer | Manual | Not built-in | `transfer_lamports` |
| Event emission | Manual | Borsh + proc macros | `emit!` / `emit_slices` (zero alloc) |
| Tx introspection | Manual | Not built-in | `read_program_id_at` / `check_has_compute_budget` |
| Ed25519 sig verify | Manual | Not built-in | `check_ed25519_signature` |
| Authority handoff | Manual | Not built-in | `accept_authority` |
| Merkle proofs | Manual | Not built-in | `verify_merkle_proof` |
| Oracle price feeds | `pyth-sdk-solana` (6 deps) | `pyth-sdk-solana` | `read_pyth_price` (zero deps) |
| AMM math / isqrt | Manual | Not built-in | `constant_product_out` / `isqrt` |
| Balance delta guard | Manual | Not built-in | `check_balance_increased` |
| Close revival defense | Manual | Not built-in | `safe_close_with_sentinel` |
| Staking rewards math | Manual | Not built-in | `update_reward_per_token` |
| Vesting schedules | Manual | Not built-in | `vested_amount` / `unlocked_at_step` |
| M-of-N multisig | Manual | Not built-in | `check_threshold` |
| Compute budget guard | Manual | Not built-in | `check_compute_remaining` |
| Flash loan detection | Manual | Not built-in | `detect_flash_loan_bracket` |
| CPI return data | Manual syscall | Framework-internal | `read_return_u64` |
| Program upgrade check | Manual | `ProgramData` type | `check_program_immutable` |
| TWAP math | Manual | Not built-in | `compute_twap` / `check_twap_deviation` |
| Lending/liquidation math | Manual | Not built-in | `check_healthy` / `max_liquidation_amount` |
| Dust-safe distribution | Manual | Not built-in | `proportional_split` / `extract_fee` |
| PDA derivation + bump | Manual syscall | `seeds + bump` constraint | `assert_pda` / `find_pda!` / `derive_pda!` |
| Data reads/writes | Manual index math | Borsh | `SliceCursor` / `DataWriter` |

*\* Anchor's upcoming 1.0 adds struct-level duplicate mutable account detection. Jiminy's checks are explicit per-operation runtime guards.*

Anchor is great for what it does. But if you went with pinocchio, you already
made your choice. You shouldn't have to hand-roll every check that comes after it.

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

Both use the Jiminy Header v1 layout and the `jiminy::prelude` import.

---

## Workspace layout

```
jiminy/
├── src/                   # Root facade (lib.rs + prelude.rs only)
├── crates/
│   ├── jiminy-core/       # Ring 1: account/, check/, math, sysvar, …
│   ├── jiminy-solana/     # Ring 2: token/, cpi/, crypto/, oracle, …
│   ├── jiminy-finance/    # Ring 3: amm, slippage
│   ├── jiminy-lending/    #         lending/liquidation
│   ├── jiminy-staking/    #         staking rewards
│   ├── jiminy-vesting/    #         vesting schedules
│   ├── jiminy-multisig/   #         M-of-N threshold
│   └── jiminy-distribute/ #         proportional splits
├── examples/
│   ├── jiminy-vault/      # Full vault CRUD example
│   └── jiminy-escrow/     # Two-party escrow example
├── bench/                 # CU benchmarks vs raw pinocchio & Anchor
└── docs/                  # Layout convention spec
```

---

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some debugging time, donations welcome at `solanadevdao.sol` (`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

---

## License

Apache-2.0. See [LICENSE](LICENSE).

pinocchio is also Apache-2.0: [anza-xyz/pinocchio](https://github.com/anza-xyz/pinocchio).
