# Migration Cookbook

Step-by-step recipes for adopting Jiminy. Whether you're coming from
raw pinocchio, Anchor, or Borsh - there's a path that doesn't require
rewriting everything.

---

## 1. Raw Pinocchio → Jiminy

**Before:** Hand-rolled pinocchio code with manual index-based account
access and ad-hoc validation.

**After:** Jiminy's prelude for structured validation, `AccountList`,
`zero_copy_layout!`, and compile-time safety.

### Step 1: Add the dependency

```toml
[dependencies]
# Replace the direct pinocchio dependency if you only need Jiminy's common surface.
jiminy = "0.16"
# pinocchio = "0.10"   ← remove
```

For the common path, use `jiminy::prelude::*` or `jiminy::hopper_runtime::*`.
Keep the direct `pinocchio` dependency only if you still call backend-specific
APIs during migration.

### Step 2: Replace account access

```rust
// BEFORE: index-based, no validation
let payer = &accounts[0];
let vault = &accounts[1];

// AFTER: iterator-based with inline checks
let mut accs = AccountList::new(accounts);
let payer = accs.next_writable_signer()?;
let vault = accs.next_writable()?;
```

### Step 3: Replace hand-rolled checks

```rust
// BEFORE: manual checks, easy to forget
if !account.is_signer() {
    return Err(ProgramError::MissingRequiredSignature);
}

// AFTER: one-liner, impossible to forget the error type
check_signer(account)?;
```

### Step 4: Adopt zero_copy_layout!

```rust
// BEFORE: manual struct + offset constants
const VAULT_DISC: u8 = 1;
const HEADER_SIZE: usize = 2;
const BALANCE_OFFSET: usize = HEADER_SIZE;
const AUTHORITY_OFFSET: usize = BALANCE_OFFSET + 8;

// AFTER: macro generates struct, constants, and loaders
zero_copy_layout! {
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}
```

### Step 5: Adopt init_account!

```rust
// BEFORE: manual CPI + zeroing + header write
CreateAccount { from: payer, to: vault, lamports, space: 56, owner: program_id }.invoke()?;
let mut data = vault.try_borrow_mut()?;
data.fill(0);
data[0] = VAULT_DISC;
data[1] = 1; // version

// AFTER: one macro call
init_account!(payer, vault, program_id, Vault)?;
```

---

## 2. Anchor → Jiminy (hot path)

**Strategy:** Keep Anchor for orchestration and client codegen. Use Jiminy
for performance-critical instructions where CU cost matters.

### Architecture

```
your-program/
├── src/
│   ├── lib.rs                  ← Anchor #[program] for admin/setup
│   └── hot_path/
│       ├── mod.rs              ← jiminy instruction_dispatch!
│       └── swap.rs             ← jiminy zero-copy swap logic
```

### Step 1: Add jiminy alongside anchor

```toml
[dependencies]
anchor-lang = "0.30"
jiminy = "0.16"
```

### Step 2: Move hot instructions to hopper-runtime/jiminy

```rust
// hot_path/swap.rs: processes raw account data, no Anchor overhead
use jiminy::prelude::*;

pub fn process_swap(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let user = accs.next_signer()?;
    let pool = accs.next_writable()?;

    let verified = Pool::load(pool, program_id)?;
    let pool_state = verified.get();
    // ... swap logic with jiminy math ...

    Ok(())
}
```

### Step 3: Route hot instructions to jiminy

In your Anchor entrypoint or a custom dispatcher, route specific
instruction tags to the jiminy handler:

```rust
// lib.rs
pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    match data[0] {
        0..=9 => anchor_handler(program_id, accounts, data),   // admin/setup
        10..  => hot_path::dispatch(program_id, accounts, data), // jiminy
    }
}
```

### Reading Anchor accounts from jiminy

Use `jiminy-anchor` to read Anchor-created accounts without importing
`anchor-lang`:

```rust
use jiminy_anchor::{anchor_disc, check_anchor_disc, anchor_body};

const POOL_DISC: [u8; 8] = anchor_disc("Pool");

// Verify the 8-byte Anchor discriminator
check_anchor_disc(&data, &POOL_DISC)?;

// Get the body bytes (skip 8-byte discriminator)
let body = anchor_body(&data)?;
```

---

## 3. Account Version Migration (V1 → V2)

### Step 1: Define the new version with extends

```rust
zero_copy_layout! {
    pub struct VaultV2, discriminator = 1, version = 2, extends = Vault {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
        fee_bps:   u16           = 2,  // new field
    }
}
```

The `extends` keyword asserts at compile time:
- Same discriminator
- Larger or equal size
- Strictly higher version

### Step 2: Handle both versions in your processor

```rust
let data = account.try_borrow()?;
let version = read_version(&data)?;

match version {
    1 => {
        let vault = Vault::overlay(&data)?;
        // Read only V1 fields.
        process_v1(vault)?;
    }
    2 => {
        let vault = VaultV2::overlay(&data)?;
        // Full access to V1 + V2 fields.
        process_v2(vault)?;
    }
    _ => return Err(ProgramError::InvalidAccountData),
}
```

### Step 3: Migrate accounts in-place (optional)

```rust
// Realloc to new size if needed.
safe_realloc(account, VaultV2::LEN, payer, system_program)?;

let mut data = account.try_borrow_mut()?;
// Write new default values for appended fields.
let vault = VaultV2::overlay_mut(&mut data)?;
vault.fee_bps = 30; // 0.3% default

// Update version and layout_id.
write_header(&mut data, VaultV2::DISC, VaultV2::VERSION, &VaultV2::LAYOUT_ID)?;
```

---

## 4. Borsh Deserialization → Zero-Copy

### Before (borsh)

```rust
use borsh::BorshDeserialize;

#[derive(BorshDeserialize)]
struct Vault {
    authority: Pubkey,
    balance: u64,
}

let vault = Vault::try_from_slice(&data)?; // allocates + copies
```

### After (jiminy zero-copy)

```rust
zero_copy_layout! {
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

let vault = Vault::overlay(&data)?; // zero-copy, no allocation
```

**Key differences:**
- No `borsh` dependency (saves ~10KB binary size)
- No heap allocation or copy
- Deterministic CU cost (no variable-length parsing)
- Compile-time size assertion via `zero_copy_layout!`
- ABI fingerprint (`LAYOUT_ID`) for cross-program verification

---

## 5. Fixed Layout → Segmented Layout

When a fixed-size account needs variable-length data (e.g., adding a
dynamic list of entries):

### Step 1: Define the segmented layout

```rust
use jiminy::prelude::*;

// V1: fixed layout.
zero_copy_layout! {
    pub struct PoolV1, discriminator = 3, version = 1 {
        header:       AccountHeader = 16,
        authority:    Address       = 32,
        total_staked: u64           = 8,
    }
}

// V2: same fixed prefix, plus a dynamic segment.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StakeEntry {
    pub staker: Address,
    pub amount: u64,
    pub epoch: u64,
}
unsafe impl Pod for StakeEntry {}
impl FixedLayout for StakeEntry { const SIZE: usize = 48; }

segmented_layout! {
    pub struct PoolV2, discriminator = 3, version = 2 {
        header:       AccountHeader = 16,
        authority:    Address       = 32,
        total_staked: u64           = 8,
    } segments {
        stakes: StakeEntry = 48,
    }
}
```

### Step 2: Migrate existing V1 accounts

```rust
// In a migration instruction:
let version = read_version(&data)?;
if version == 1 {
    // 1. Realloc to new size (fixed prefix + table + initial capacity).
    let new_size = PoolV2::compute_account_size(&[0])?; // 0 initial stakes
    safe_realloc(account, new_size, payer, system_program)?;

    // 2. Initialize the segment table.
    let mut data = account.try_borrow_mut()?;
    PoolV2::init_segments(&mut data, &[0])?;

    // 3. Update the header.
    write_header(&mut data, PoolV2::DISC, PoolV2::VERSION, &PoolV2::SEGMENTED_LAYOUT_ID)?;
}
```

### Step 3: Handle both versions in processors

```rust
let data = account.try_borrow()?;
let version = read_version(&data)?;
match version {
    1 => {
        let pool = PoolV1::overlay(&data)?;
        // Read fixed fields only - no segment access.
    }
    2 => {
        let pool = PoolV2::overlay(&data)?;
        // Fixed fields + segment access.
        let stakes = PoolV2::segment::<StakeEntry>(&data, PoolV2::stakes)?;
    }
    _ => return Err(ProgramError::InvalidAccountData),
}
```

**Key points:**

- The `SEGMENTED_LAYOUT_ID` differs from `LAYOUT_ID` because segment
  entries are included in the hash input (`seg:stakes:StakeEntry,...`).
- V1 accounts continue to work until explicitly migrated.
- Use `PoolV2::MIN_ACCOUNT_SIZE` to know the smallest valid V2
  account (fixed prefix + segment table, zero elements).
