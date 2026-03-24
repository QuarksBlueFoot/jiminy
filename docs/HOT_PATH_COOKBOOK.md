# Hot Path Cookbook

Recipes for squeezing every CU out of Jiminy's zero-copy primitives.
These are the patterns that matter when your swap instruction is
competing for blockspace and every compute unit is money.

## 1. Zero-Copy Account Reading (The Basics)

```rust,ignore
use jiminy::prelude::*;

zero_copy_layout! {
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

// Tier 1 load: validate owner + disc + version + layout_id + size.
let data = Vault::load(account, program_id)?;
let vault = Vault::overlay(&data)?;
```

**CU cost:** ~150 CU for full validation + overlay.

## 2. Cross-Program Account Reads

Reading another program's Jiminy account without depending on its crate:

```rust,ignore
// In YOUR program: you know the layout because of the ABI contract.
zero_copy_layout! {
    pub struct ForeignPool, discriminator = 5, version = 1 {
        header:     AccountHeader = 16,
        liquidity:  u64           = 8,
        fee_rate:   u64           = 8,
        authority:  Address       = 32,
    }
}

// Tier 2: validates owner + layout_id, returns borrowed bytes.
let data = ForeignPool::load_foreign(pool_account, &POOL_PROGRAM_ID)?;
let pool = ForeignPool::overlay(&data)?;
let liquidity = pool.liquidity;
```

**Key:** `load_foreign` validates `owner + layout_id` but skips `disc`/`version`
since you may not know the other program's version scheme. It returns
borrowed bytes, so you must call `overlay()` to get the typed reference.

## 3. Token Balance Reads (No Borsh)

```rust,ignore
use jiminy_solana::token::{token_account_amount, token_account_owner};

// Direct byte read - no deserialization.
let balance = token_account_amount(token_account)?;
let owner = token_account_owner(token_account)?;
```

Or using `jiminy-layouts` for full struct access:

```rust,ignore
use jiminy_layouts::SplTokenAccount;
use jiminy_core::account::pod_from_bytes;

let token = pod_from_bytes::<SplTokenAccount>(data)?;
let amount = token.amount();
```

## 4. Versioned Account Migration

Safely reading a V2 account that extends V1:

```rust,ignore
zero_copy_layout! {
    pub struct VaultV1, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

zero_copy_layout! {
    pub struct VaultV2, discriminator = 1, version = 2, extends = VaultV1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
        fee_rate:  u64           = 8,  // new field
    }
}

// V2 processor can read both:
let data = account.try_borrow()?;
if data.len() >= VaultV2::LEN {
    let v2 = VaultV2::load_checked(&data)?;
    // use v2.fee_rate
} else {
    let v1 = VaultV1::load_checked(&data)?;
    // default fee_rate = 0
}
```

## 5. Batch Account Processing

When processing multiple accounts (e.g., liquidation scanner):

```rust,ignore
use jiminy_core::prelude::*;

// Use load_best_effort for maximum tolerance
fn scan_accounts(accounts: &[AccountView]) -> Vec<u64> {
    let mut balances = Vec::new();
    for acc in accounts {
        if let Ok(data) = acc.try_borrow() {
            if let Ok(vault) = Vault::overlay(&data) {
                balances.push(vault.balance);
            }
        }
    }
    balances
}
```

## 6. PDA Derivation + Account Init

```rust,ignore
use jiminy_core::prelude::*;

let (pda, bump) = find_pda!(&[b"vault", authority.address()], program_id);

init_account!(
    Vault,
    payer,
    vault_account,
    &[b"vault", authority.address(), &[bump]],
    program_id,
    system_program,
);

let mut vault = Vault::overlay_mut(vault_account.try_borrow_mut()?.as_mut())?;
vault.balance = 0;
vault.authority = *authority.address();
```

## 7. Safe CPI with Guards

```rust,ignore
use jiminy_solana::cpi::*;

// Reentrancy guard: prevent CPI attacks
check_no_cpi_caller(sysvar_instructions)?;

// Safe token transfer (checks program ID)
safe_transfer_tokens(
    token_program,
    source,
    destination,
    authority,
    amount,
)?;
```

## 8. Reading Anchor Accounts

```rust,ignore
use jiminy_anchor::{anchor_disc, check_and_body};
use jiminy_core::account::pod_from_bytes;

const ANCHOR_POOL_DISC: [u8; 8] = anchor_disc("Pool");

#[repr(C)]
#[derive(Clone, Copy)]
struct AnchorPoolBody {
    liquidity: [u8; 8],
    sqrt_price: [u8; 16],
}
unsafe impl jiminy_core::account::Pod for AnchorPoolBody {}
impl jiminy_core::account::FixedLayout for AnchorPoolBody {
    const SIZE: usize = 24;
}

let body = check_and_body(data, &ANCHOR_POOL_DISC)?;
let pool = pod_from_bytes::<AnchorPoolBody>(body)?;
```

## CU Budget Rules of Thumb

| Operation | Approximate CU |
|-----------|---------------|
| `pod_from_bytes` (overlay) | ~5 CU |
| `load` (full validation) | ~150 CU |
| `load_foreign` (cross-program) | ~100 CU |
| `sol_log` (one message) | ~100 CU |
| Borsh deserialize (small struct) | ~500-2000 CU |
| SHA-256 (32 bytes) | ~500 CU |
| Token transfer CPI | ~4000 CU |
| CreateAccount CPI | ~5000 CU |

---

## 9. Borrow-Splitting with `split_fields`

When you need to read or mutate multiple fields simultaneously, the
standard `overlay` approach borrows the entire account data. With
`split_fields` / `split_fields_mut`, each field becomes an independent
`FieldRef` / `FieldMut` - no aliasing, no `unsafe`, the borrow
checker is happy:

```rust,ignore
use jiminy::prelude::*;

zero_copy_layout! {
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

// ── Immutable split ──────────────────────────────────────────────
let data = account.try_borrow()?;
let (header, balance, authority) = Vault::split_fields(&data)?;
let bal: u64 = balance.read_u64();
let auth: &Address = authority.as_address();

// ── Mutable split - update two fields at once ────────────────────
let mut data = account.try_borrow_mut()?;
let (_header, balance, authority) = Vault::split_fields_mut(&mut data)?;
balance.write_u64(1000);
authority.copy_from(new_authority.as_ref());
```

**When to use:**

- You need to read field A and write field B simultaneously.
- You're passing individual field references to helper functions.
- You want to avoid `overlay()` aliasing when the borrow checker
  complains.

**CU cost:** Same as `overlay` (~5 CU). It's just a bounds check
and pointer arithmetic.

## 10. Le* Types for Alignment-1 Storage

Jiminy's `LeU16`, `LeU32`, `LeU64`, `LeU128`, `LeI16`, `LeI32`,
`LeI64`, `LeI128` types store integers as raw LE byte arrays.
They're `Pod + FixedLayout`, alignment 1, and can be compared directly
(implementing `PartialEq`, `PartialOrd`, `Ord`, `Display`):

```rust,ignore
use jiminy::prelude::*;

zero_copy_layout! {
    pub struct Pool, discriminator = 2, version = 1 {
        header:      AccountHeader = 16,
        liquidity:   LeU64         = 8,
        sqrt_price:  LeU128        = 16,
        fee_rate:    LeU16         = 2,
        authority:   Address       = 32,
    }
}

let pool = Pool::overlay(&data)?;

// Read: convert to native type.
let liq: u64 = pool.liquidity.get();

// Write: set from native type.
let mut pool = Pool::overlay_mut(&mut data)?;
pool.liquidity.set(1_000_000);

// Direct comparison (no .get() needed).
if pool.liquidity > LeU64::from(500_000u64) {
    // ...
}

// Display (prints the native value).
msg!("liquidity: {}", pool.liquidity);
```

**When to use Le* instead of raw `u64`:**

- **Always** for on-chain structs in `zero_copy_layout!`. Le* avoids
  unaligned read UB on big-endian or strict-alignment targets.
- The `u64 = 8` syntax still works and maps to raw `[u8; 8]` in the
  repr(C) struct. Le* gives you typed accessors for free.

## 11. Segmented Layout Recipes

### Creating a segmented account

```rust,ignore
use jiminy::prelude::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Order {
    pub price: [u8; 8],
    pub qty:   [u8; 8],
}
unsafe impl Pod for Order {}
impl FixedLayout for Order { const SIZE: usize = 16; }

segmented_layout! {
    pub struct OrderBook, discriminator = 5, version = 1 {
        header:     AccountHeader = 16,
        market:     Address       = 32,
    } segments {
        bids: Order = 16,
        asks: Order = 16,
    }
}

// Compute total account size for 10 bids + 5 asks.
let size = OrderBook::compute_account_size(&[10, 5])?;
// = 48 (fixed) + 16 (table: 2 × 8) + 160 (10×16) + 80 (5×16) = 304

// After creating the account (e.g., via CPI), initialize segments.
let mut data = account.try_borrow_mut()?;
OrderBook::init_segments(&mut data, &[10, 5])?;
```

### Reading segment data

```rust,ignore
// Named index constants: generated from segment names.
let bids = OrderBook::segment::<Order>(&data, OrderBook::bids)?;
let asks = OrderBook::segment::<Order>(&data, OrderBook::asks)?;

// Read element by index.
let first_bid: Order = bids.read(0)?;

// Iterate.
for order in bids.iter() {
    let price = u64::from_le_bytes(order.price);
    // ...
}
```

### Push and swap-remove

```rust,ignore
// Push a new order to the bids segment.
OrderBook::push::<Order>(&mut data, OrderBook::bids, &new_order)?;

// Remove order at index 2 (O(1), swaps with last).
let removed: Order = OrderBook::swap_remove::<Order>(
    &mut data, OrderBook::bids, 2,
)?;
```

### Mutable segment views

```rust,ignore
let mut asks = OrderBook::segment_mut::<Order>(&mut data, OrderBook::asks)?;
asks.set(0, &updated_ask)?;  // overwrite element at index 0
```

### Full validation

```rust,ignore
// Validates bounds, element sizes, ordering, and no overlaps.
OrderBook::validate_segments(&data)?;
```

## 12. FieldRef / FieldMut Standalone Usage

`FieldRef` and `FieldMut` aren't just for `split_fields` - you can
construct them from any `&[u8]` / `&mut [u8]` slice for typed access:

```rust,ignore
use jiminy::abi::{FieldRef, FieldMut};

// Read a u64 from an arbitrary byte offset.
let field = FieldRef::new(&data[24..32]);
let value: u64 = field.read_u64();
let addr: &Address = field.as_address(); // or .read_address() for copy

// Write a u64 at an arbitrary offset.
let mut field = FieldMut::new(&mut data[24..32]);
field.write_u64(42);

// Helper methods:
field.len();      // 8
field.as_bytes(); // &[u8]
```

## Anti-Patterns

### DON'T: Copy account data to stack

```rust,ignore
// BAD: copies entire struct to stack
let vault_copy: Vault = pod_read(&data)?;

// GOOD: zero-copy reference
let vault: &Vault = pod_from_bytes(&data)?;
```

### DON'T: Use `load_unchecked` without good reason

```rust,ignore
// BAD: skips all safety checks
let vault = unsafe { Vault::load_unchecked(&data) }?;

// GOOD: use the appropriate trust tier
let vault = Vault::load_checked(account, program_id)?;
```

### DON'T: Serialize just to log

```rust,ignore
// BAD: allocates + serializes
msg!("Balance: {}", vault.balance);

// GOOD: log raw bytes
jiminy_core::log::log_u64(vault.balance);
```
