# Pinocchio Framework: Comprehensive Research Report

**Source**: `anza-xyz/pinocchio` (GitHub), crates.io v0.10.2  
**Researched**: 2026-03-27 -- direct source code analysis
**Repo structure**: Monorepo with `sdk/` (core crate) + `programs/` (system, token, token-2022, ATA, memo)

---

## 1. Architecture

### Crate Decomposition (v0.10.2+)

Pinocchio is architecturally split across five small, Anza-maintained crates:

| Crate | Purpose |
|---|---|
| `solana-account-view` | `AccountView`, `RuntimeAccount`, `Ref`/`RefMut` borrow guards |
| `solana-address` | `Address` type (`[u8; 32]`), PDA derivation syscalls |
| `solana-instruction-view` | `InstructionView`, `InstructionAccount`, CPI module |
| `solana-program-error` | `ProgramError` enum, `ProgramResult` type alias |
| `pinocchio` (SDK) | Entrypoint macros, sysvar access, re-exports everything above |

The core `pinocchio` crate is `#![no_std]` and re-exports the sub-crates:

```rust
pub use {
    solana_account_view::{self as account, AccountView},
    solana_address::{self as address, Address},
    solana_program_error::{self as error, ProgramResult},
};
#[cfg(feature = "cpi")]
pub use {solana_instruction_view as instruction, solana_instruction_view::cpi};
```

### Program Entrypoint Model

Pinocchio offers **three entrypoint strategies**, each a declarative macro:

**1. `entrypoint!` -- All-in-one (most common)**
```rust
entrypoint!(process_instruction);
// Expands to:
//   program_entrypoint!(process_instruction);
//   default_allocator!();       // bump allocator at 0x300000000
//   default_panic_handler!();   // logs panic file, prints "** PANICKED **"
```

Signature:
```rust
fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult;
```

**2. `program_entrypoint!` -- Entrypoint without allocator/panic**
Same signature as above, but the developer must independently declare an allocator and panic handler. Suitable for `no_std` programs.

**3. `lazy_program_entrypoint!` -- On-demand parsing**
```rust
fn process_instruction(mut context: InstructionContext) -> ProgramResult;
```

`InstructionContext` wraps the raw input buffer and exposes:
- `remaining()` → number of unparsed accounts
- `next_account()` → returns `MaybeAccount` (Account or Duplicated index)
- `instruction_data()` / `instruction_data_unchecked()` → instruction bytes
- `program_id()` / `program_id_unchecked()` → program address

The lazy entrypoint only reads accounts when called, saving CUs for programs that don't need all accounts. Trade-off: the program must handle duplicate account mapping manually.

**4. `process_entrypoint()` -- Public function for custom entrypoints**
Exposed as `pinocchio::entrypoint::process_entrypoint::<MAX_ACCOUNTS>(input, handler)`. Programs can write a raw `#[no_mangle] pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64` and call this for standard parsing after any fast-path logic.

### Instruction Dispatch

Pinocchio provides **zero** instruction dispatch infrastructure. There is no equivalent of Anchor's `#[program]` or instruction discriminator system. Programs must manually:
1. Parse `instruction_data[0]` (or first N bytes) as a discriminator
2. `match` on it
3. Slice remaining data for arguments

Example pattern (from pinocchio-vault benchmark):
```rust
match instruction_data[0] {
    0 => process_init_vault(program_id, accounts, &instruction_data[1..]),
    1 => process_deposit(program_id, accounts, &instruction_data[1..]),
    _ => Err(ProgramError::InvalidInstructionData),
}
```

### Account Indexing

Accounts are received as a `&mut [AccountView]` slice. There is no named-account system. Programs index accounts positionally:
```rust
let payer = &accounts[0];
let vault = &accounts[1];
let system = &accounts[2];
```

---

## 2. Zero-Copy Approach

### Core Principle: Pointer Overlay on Runtime Buffer

The runtime serializes accounts into a contiguous byte buffer. Pinocchio's entrypoint does **not** copy or deserialize this data -- it interprets the buffer in-place via pointer casts.

**`RuntimeAccount` (the raw account header):**
```rust
#[repr(C)]
pub struct RuntimeAccount {
    pub borrow_state: u8,      // reuses duplicate flag byte for borrow tracking
    pub is_signer: u8,
    pub is_writable: u8,
    pub executable: u8,
    pub resize_delta: i32,     // v1.0: tracks accumulated resize; v2.0: renamed `padding`
    pub address: Address,       // 32 bytes
    pub owner: Address,         // 32 bytes
    pub lamports: u64,
    pub data_len: u64,
    // Actual account data follows immediately in memory
}
```

**`AccountView` -- The zero-copy wrapper:**
```rust
#[repr(C)]
pub struct AccountView {
    raw: *mut RuntimeAccount,   // pointer into the runtime input buffer
}
```

`AccountView` is a raw pointer wrapper. It does **not** own data. Every accessor reads directly from the runtime buffer:
```rust
pub fn lamports(&self) -> u64 {
    unsafe { (*self.raw).lamports }
}
pub fn data_ptr(&self) -> *mut u8 {
    unsafe { (self.raw as *mut u8).add(size_of::<RuntimeAccount>()) }
}
```

### Account Data as Typed Overlays

Programs interpret account data by casting the raw bytes to `#[repr(C)]` structs:

```rust
#[repr(C)]
pub struct TokenAccount {
    mint: Address,              // 32 bytes
    owner: Address,             // 32 bytes  
    amount: [u8; 8],            // stored as raw bytes, read via u64::from_le_bytes
    delegate_flag: [u8; 4],     // COption flag
    delegate: Address,          // 32 bytes
    // ... etc
}

impl TokenAccount {
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        &*(bytes.as_ptr() as *const TokenAccount)
    }
}
```

Key pattern: multi-byte integers are stored as `[u8; N]` arrays and read via `u64::from_le_bytes()` / `u128::from_le_bytes()`. This avoids alignment issues on non-BPF targets since the struct has alignment of 1.

### Borrow Tracking

The `borrow_state` field in `RuntimeAccount` repurposes the duplicate-marker byte (which is `0xFF` for non-duplicate accounts) as a reference counter:

- `0xFF` (255) = NOT_BORROWED -- available for mutable borrow
- `0` = mutably borrowed
- `2..254` = immutably borrowed (count of remaining immutable borrows)

`try_borrow()` returns `Ref<'_, [u8]>` which decrements state on creation and increments on `Drop`.  
`try_borrow_mut()` returns `RefMut<'_, [u8]>` which sets state to 0 and resets to 255 on `Drop`.

Both `Ref` and `RefMut` support `map`, `try_map`, and `filter_map` for projecting to typed overlays:
```rust
let token = Ref::map(account_view.try_borrow()?, |data| unsafe {
    TokenAccount::from_bytes_unchecked(data)
});
```

**Unsafe bypass:** `borrow_unchecked()` and `borrow_unchecked_mut()` return raw `&[u8]` / `&mut [u8]` without touching borrow state, saving ~2 CUs per access. Used when program has already verified no duplicate accounts.

---

## 3. Account Validation

Pinocchio provides **no declarative validation framework**. All validation is manual, imperative code. The `AccountView` API provides these primitive checks:

| Method | Check |
|---|---|
| `is_signer() -> bool` | Transaction was signed by this account |
| `is_writable() -> bool` | Account is writable in this transaction |
| `owned_by(&Address) -> bool` | Account's owner matches expected program |
| `executable() -> bool` | Account is an executable program |
| `is_data_empty() -> bool` | Account data length is 0 |
| `data_len() -> usize` | Account data length |
| `address() -> &Address` | Account's public key |
| `unsafe owner() -> &Address` | Returns owner reference directly (unsafe: may be invalidated by `assign`/`close`) |

### Typical Validation Pattern

```rust
// Signer check
if !authority.is_signer() {
    return Err(ProgramError::MissingRequiredSignature);
}
// Owner check
if !vault.owned_by(program_id) {
    return Err(ProgramError::IncorrectProgramId);
}
// Writable check
if !vault.is_writable() {
    return Err(ProgramError::InvalidArgument);
}
// Size check
if vault.data_len() < EXPECTED_SIZE {
    return Err(ProgramError::AccountDataTooSmall);
}
// Initialization check
if !vault.is_data_empty() {
    return Err(ProgramError::AccountAlreadyInitialized);
}
```

**Notable: `owner()` is unsafe.** The safe way to check ownership is `owned_by()` which does a comparison. Using `owner()` directly returns a reference that can be invalidated by `assign()` or `close()` on the same `AccountView`.

### From-Account-View Pattern

The companion `pinocchio-token` crate uses a `from_account_view` pattern for typed validation:

```rust
impl TokenAccount {
    pub fn from_account_view(av: &AccountView) -> Result<Ref<'_, TokenAccount>, ProgramError> {
        if av.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if !av.owned_by(&ID) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Ref::map(av.try_borrow()?, |data| unsafe {
            Self::from_bytes_unchecked(data)
        }))
    }
}
```

This combines size + owner + borrow check in one call.

---

## 4. Key Types and Traits

### Core Types

| Type | Location | Description |
|---|---|---|
| `Address` | `solana-address` | `#[repr(transparent)]` newtype over `[u8; 32]`. The Solana public key. Optionally `Copy` via feature flag. |
| `AccountView` | `solana-account-view` | Zero-copy window into a runtime account. Wraps `*mut RuntimeAccount`. |
| `RuntimeAccount` | `solana-account-view` | `#[repr(C)]` raw account header matching the runtime's serialization layout. |
| `Ref<'a, T>` | `solana-account-view` | Immutable borrow guard for account data. Tracks borrow via `borrow_state`. |
| `RefMut<'a, T>` | `solana-account-view` | Mutable borrow guard for account data. |
| `InstructionContext` | `pinocchio::entrypoint::lazy` | Lazy-parsed instruction wrapper for on-demand account access. |
| `MaybeAccount` | `pinocchio::entrypoint::lazy` | `enum { Account(AccountView), Duplicated(u8) }` -- result of lazy account parse. |
| `ProgramError` | `solana-program-error` | Enum with 26 builtin error variants + `Custom(u32)`. |
| `ProgramResult` | `solana-program-error` | `Result<(), ProgramError>` alias. |
| `InstructionView<'a,'b,'c,'d>` | `solana-instruction-view` | Holds `program_id`, `data`, `accounts` for CPI. |
| `InstructionAccount<'a>` | `solana-instruction-view` | `#[repr(C)]` struct: `address`, `is_writable`, `is_signer`. |
| `CpiAccount<'a>` | `solana-instruction-view::cpi` | `#[repr(C)]` struct matching `sol_invoke_signed_c` layout. |
| `Seed<'a>` | `solana-instruction-view::cpi` | `#[repr(C)]` wrapper for seed bytes (ptr + len). |
| `Signer<'a,'b>` | `solana-instruction-view::cpi` | `#[repr(C)]` wrapper for a PDA signer's seeds slice. |

### Sysvar Types

| Type | Module |
|---|---|
| `Rent` | `pinocchio::sysvars::rent` |
| `Clock` | `pinocchio::sysvars::clock` |
| `Fees` | `pinocchio::sysvars::fees` |
| `SlotHashes` | `pinocchio::sysvars::slot_hashes` |

### Key Traits

| Trait | Description |
|---|---|
| `Sysvar` | `fn get() -> Result<Self, ProgramError>` -- loads sysvar via syscall, no account needed. |
| `Resize` | Feature-gated. `fn resize(&mut self, new_len: usize) -> Result<(), ProgramError>` for account data grow/shrink. |
| `UnsafeResize` | Feature-gated alternative without bounds checking. |

### Key Macros (ALL declarative -- no proc macros)

| Macro | Kind | Description |
|---|---|---|
| `entrypoint!` | `macro_rules!` | Emits program entrypoint + allocator + panic handler |
| `program_entrypoint!` | `macro_rules!` | Emits only the entrypoint |
| `lazy_program_entrypoint!` | `macro_rules!` | Emits lazy (on-demand parsing) entrypoint |
| `default_allocator!` | `macro_rules!` | Bump allocator at `0x300000000` |
| `no_allocator!` | `macro_rules!` | Panicking allocator + exposes `allocate_unchecked` for manual heap use |
| `default_panic_handler!` | `macro_rules!` | Panic hook for `std` context |
| `nostd_panic_handler!` | `macro_rules!` | `#[panic_handler]` for `no_std` context |
| `address!` | `macro_rules!` | `Address::from_str_const("...")` convenience |
| `declare_id!` | `macro_rules!` | Declares static `ID`, `check_id()`, `id()` |
| `seeds!` | `macro_rules!` | Constructs `[Seed; N]` array from expressions |
| `impl_sysvar_get!` | `macro_rules!` | Implements `Sysvar::get()` for a sysvar type |

**Pinocchio uses zero proc macros.** Everything is declarative `macro_rules!` or manual code.

---

## 5. Strengths

### 5.1 Minimal Dependencies
The entire SDK has only 4 direct dependencies, all Anza-maintained with minimal transitive deps. No `solana-program` (~200 dependency tree) needed.

### 5.2 Extreme CU Efficiency
- Entrypoint deserialization uses zero copies -- pointer overlay on runtime buffer
- Account parsing is unrolled via `process_n_accounts!` macro (5 accounts at a time inlined)
- `#[inline(always)]` everywhere for hot paths
- `Address` equality uses `u64`-chunked comparison (`read_unaligned` × 4) for better CU performance
- PDA derivation via `derive_address()` uses SHA-256 syscall directly (~100 CU) vs `create_program_address` syscall (~1500 CU)

### 5.3 Binary Size
No external dependencies means dramatically smaller `.so` binaries. Programs can use `no_allocator!` to eliminate heap entirely.

### 5.4 No Proc Macros
Eliminates compile-time explosion from proc macro expansion. Faster builds, easier to audit.

### 5.5 Flexible Entrypoint Options
Three entrypoint strategies serve different optimization needs. Custom entrypoint support via `process_entrypoint()` allows fast-path branching before full deserialization.

### 5.6 Zero-Alloc Path Available
With `no_allocator!` + `nostd_panic_handler!` + `lazy_program_entrypoint!`, a program can be fully `no_std` and `no_alloc`, using only the 32KB heap manually via `allocate_unchecked`.

### 5.7 Proper Borrow Tracking
Runtime-compatible `Ref`/`RefMut` guards that track borrows via the account's duplicate marker byte -- same mechanism the runtime uses, so CPI borrow checks work correctly.

### 5.8 Const PDA Derivation
`Address::derive_address_const()` enables compile-time PDA computation for known seeds.

---

## 6. Weaknesses / Complaints

### 6.1 No Account Validation Framework
Every program must write the same boilerplate signer/owner/size checks manually. Anchor's `#[account]` attributes and constraints eliminate entire classes of bugs. With pinocchio, forgetting a check is easy.

### 6.2 No Instruction Dispatch
No discriminator system, no automatic routing. Every program hand-writes a `match` on the first byte(s). This is tedious and error-prone for programs with many instructions.

### 6.3 No IDL / Client Generation
No generated TypeScript client, no JSON IDL, no instruction builders. Client-side integration requires manual work matching the on-chain byte layout.

### 6.4 Extremely Unsafe
The codebase is heavily `unsafe`. Key examples:
- `AccountView::new_unchecked` -- raw pointer, no validation
- `borrow_unchecked` / `borrow_unchecked_mut` -- bypass borrow tracking
- `close_unchecked` -- zeroes 48 bytes at a fixed offset, UB if account not runtime-created
- `from_bytes_unchecked` -- pointer cast with no alignment/size validation
- `owner()` returns a reference that can be invalidated by `assign()`/`close()`

Programs *must* understand the safety contracts. Misuse leads to UB, not a helpful error.

### 6.5 No Serialization Support
No built-in borsh, bincode, or any (de)serialization. Programs must manually read/write bytes:
```rust
raw[1..9].copy_from_slice(&balance.to_le_bytes());
```
This is error-prone and tedious for complex state.

### 6.6 No Error Code Macros
`ProgramError::Custom(u32)` is the only way to define program-specific errors. No built-in macro to derive meaningful error types (though `ToStr` trait exists for display).

### 6.7 Duplicate Account Handling
The standard `entrypoint!` handles duplicates transparently, but `lazy_program_entrypoint!` returns `MaybeAccount::Duplicated(index)` which the program must handle manually -- easy to get wrong.

### 6.8 `AccountView` Doesn't Track Data Length Changes Well
The `resize_delta` (v1) / `padding` (v2) reuse is confusing. The account-resize feature stores original data length in what was the padding field. The `close_unchecked()` doesn't update `resize_delta`, so subsequent `resize()` may be incorrect.

### 6.9 CPI Lifetime Soup
`InstructionView<'a, 'b, 'c, 'd>` has four lifetime parameters. `CpiAccount`, `Seed`, `Signer` each have their own lifetimes. Building CPI calls correctly requires careful lifetime management.

### 6.10 No Testing Infrastructure
No test framework, no `ProgramTest` equivalent, no BanksClient. Programs must bring their own test harness (typically `solana-program-test` from the main SDK, which requires the full `solana-program` dependency).

---

## 7. CPI Handling

### Building CPI Instructions

CPI uses the `InstructionView` struct and `InstructionAccount` helpers:

```rust
let instruction_accounts = [
    InstructionAccount::writable_signer(payer.address()),
    InstructionAccount::writable(vault.address()),
];

let mut instruction_data = [0u8; 12];
instruction_data[0] = 2; // discriminator
instruction_data[4..12].copy_from_slice(&amount.to_le_bytes());

let instruction = InstructionView {
    program_id: &system_program::ID,
    accounts: &instruction_accounts,
    data: &instruction_data,
};
```

### Invoke Functions

| Function | Stack | Heap | Borrow Check |
|---|---|---|---|
| `invoke::<N>()` | Yes (array) | No | Yes |
| `invoke_signed::<N>()` | Yes (array) | No | Yes |
| `invoke_with_bounds::<MAX>()` | Yes (array) | No | Yes |
| `invoke_signed_with_bounds::<MAX>()` | Yes (array) | No | Yes |
| `invoke_with_slice()` | No | Yes (Box) | Yes |
| `invoke_signed_with_slice()` | No | Yes (Box) | Yes |
| `invoke_unchecked()` | N/A | N/A | **No** |
| `invoke_signed_unchecked()` | N/A | N/A | **No** |

The `_unchecked` variants skip address matching and borrow validation -- significant CU savings but UB if accounts are incorrectly borrowed.

### Borrow Validation During CPI

The checked variants iterate instruction accounts and verify:
1. The `AccountView` addresses match `InstructionAccount` addresses  
2. Writable accounts are not already borrowed (any form)
3. Read-only accounts are not mutably borrowed

### Signer Seeds

PDA signing uses `Seed` and `Signer` wrappers:
```rust
let seeds = seeds!(b"vault", authority.as_ref(), &[bump]);
let signer = Signer::from(&seeds);
invoke_signed(&instruction, &[payer, vault], &[signer])?;
```

### Return Data

```rust
cpi::set_return_data(&data);              // set (max 1024 bytes)
let ret: Option<ReturnData> = cpi::get_return_data();  // get
```

### CPI Account Limits

- `MAX_STATIC_CPI_ACCOUNTS = 64` (stack-allocated variants)
- `MAX_CPI_ACCOUNTS = 128` (heap-allocated variants, will increase to 255 with SIMD-0339)

### Companion CPI Crates

`pinocchio-system` (v0.5) and `pinocchio-token` (v0.5) provide typed instruction builders:

```rust
// pinocchio-system
Transfer { from: &payer, to: &vault, lamports: 1_000_000 }.invoke()?;
CreateAccount { from: &payer, to: &mint, lamports, space, owner: &token_program_id }.invoke_signed(&[signer])?;

// pinocchio-token
MintTo::new(&mint, &token_account, &authority, amount).invoke_signed(&[signer])?;
Transfer::new(&from_token, &to_token, &authority, amount).invoke()?;
```

---

## 8. Error Handling

### ProgramError Enum

26 builtin variants (`InvalidArgument`, `MissingRequiredSignature`, `IncorrectProgramId`, etc.) + `Custom(u32)` for program-specific errors.

### Error Propagation

Standard `Result<(), ProgramError>` with `?` operator:
```rust
let data = account.try_borrow()?;  // returns ProgramError::AccountBorrowFailed on failure
```

### Custom Errors

Programs define custom errors via `ProgramError::Custom(code)`:
```rust
const ERR_INSUFFICIENT_BALANCE: u32 = 0x1000;
return Err(ProgramError::Custom(ERR_INSUFFICIENT_BALANCE));
```

The `ToStr` trait enables human-readable error messages:
```rust
impl ToStr for MyError {
    fn to_str(&self) -> &'static str {
        match self { MyError::InsufficientBalance => "Insufficient balance" }
    }
}
```

### Error-to-u64 Conversion

Errors are returned from the entrypoint as `u64` values. Builtin errors occupy upper 32 bits (shifted by `BUILTIN_BIT_SHIFT = 32`). Custom errors use lower 32 bits + the `CUSTOM_ZERO` base.

### No Error Derive Macros

Unlike Anchor's `#[error_code]`, pinocchio provides no macros for defining error enums. Programs must manually impl `From<MyError> for ProgramError` or use raw `Custom(u32)` values.

---

## 9. Memory Layout

### Runtime Serialization Format

The SVM serializes accounts into a flat byte buffer:

```
[8 bytes] num_accounts
For each account:
  [1 byte] dup_marker (0xFF = not duplicate, else = index of original)
  If not duplicate:
    [1 byte] is_signer
    [1 byte] is_writable  
    [1 byte] executable
    [4 bytes] padding/resize_delta
    [32 bytes] address
    [32 bytes] owner
    [8 bytes] lamports
    [8 bytes] data_len
    [data_len bytes] account data
    [10240 bytes] resize padding (MAX_PERMITTED_DATA_INCREASE)
    [variable] alignment padding to 8-byte boundary
    [8 bytes] rent_epoch (unused by pinocchio)
  If duplicate:
    [7 bytes] padding
[8 bytes] instruction_data_len
[variable] instruction_data
[32 bytes] program_id
```

### Account Data Struct Design Conventions

Pinocchio programs use `#[repr(C)]` structs with **alignment 1**:

```rust
#[repr(C)]
pub struct Mint {
    mint_authority_flag: [u8; 4],  // COption tag
    mint_authority: Address,        // always present, invalid if flag != 1
    supply: [u8; 8],               // u64 as raw bytes
    decimals: u8,
    is_initialized: u8,
    freeze_authority_flag: [u8; 4],
    freeze_authority: Address,
}
```

**Critical patterns:**
- Multi-byte integers as `[u8; N]` arrays, read via `from_le_bytes()` -- avoids alignment traps
- `COption<T>` modeled as `flag: [u8; 4]` + `value: T` -- the value is always allocated, flag indicates presence
- All structs are `repr(C)` to match the on-chain layout exactly
- `Address` inside structs works because it's `repr(transparent)` over `[u8; 32]`

### Padding and Alignment

- BPF has `align_of::<u128>() == 8` (not 16 as on x86)
- Account data in the buffer is followed by 10240 bytes of resize padding
- After resize padding, alignment padding brings the pointer to an 8-byte boundary
- The `align_pointer!` macro handles this: `(ptr + 7) & !7`

### Account Data Access

Two paths:
1. **Safe**: `try_borrow() -> Ref<[u8]>` / `try_borrow_mut() -> RefMut<[u8]>` -- tracks borrows
2. **Unsafe**: `borrow_unchecked() -> &[u8]` / `borrow_unchecked_mut() -> &mut [u8]` -- no tracking

Both return raw byte slices. Programs cast to typed overlays:
```rust
let data = account.try_borrow()?;
let token = unsafe { TokenAccount::from_bytes_unchecked(&*data) };
```

### Account Close

`close_unchecked()` zeroes 48 bytes immediately before the data pointer (owner + lamports + data_len):
```rust
pub unsafe fn close_unchecked(&self) {
    write_bytes(self.data_ptr().sub(48), 0, 48);
}
```

The safe `close()` additionally checks borrow state and updates `resize_delta`.

---

## 10. Macro System

### All Macros Are Declarative (`macro_rules!`)

Pinocchio has **zero proc macros**. This is a deliberate design choice:
- Eliminates build-time proc macro compilation overhead
- No hidden code generation -- everything is auditable
- No dependency on `syn`, `quote`, `proc-macro2`

### Entrypoint Macros

```rust
// Standard entrypoint (allocator + panic handler)
entrypoint!(handler);
entrypoint!(handler, { 16 });  // limit to 16 accounts

// Entrypoint only
program_entrypoint!(handler);
program_entrypoint!(handler, { 16 });

// Lazy entrypoint
lazy_program_entrypoint!(handler);
```

The second argument to `entrypoint!`/`program_entrypoint!` controls `MAX_ACCOUNTS`, reducing the stack-allocated `[MaybeUninit<AccountView>; N]` array size.

### Allocator/Panic Macros

```rust
default_allocator!();     // Bump allocator
no_allocator!();          // Panicking allocator + allocate_unchecked helper
default_panic_handler!(); // For std programs
nostd_panic_handler!();   // For no_std programs
```

`no_allocator!()` also emits:
```rust
fn allocate_unchecked::<T>(offset: usize) -> &'static mut T
```
This lets programs manually place types on the 32KB heap at known offsets.

### Address/ID Macros

```rust
address!("So11111111111111111111111111111111111111112")  // const Address
declare_id!("MyProgram11111111111111111111111111111111")  // ID + check_id() + id()
```

### CPI Helper Macros

```rust
seeds!(b"vault", authority.as_ref(), &[bump])  // [Seed; N] array
```

### Internal Optimization Macros

The entrypoint uses several internal macros for CU-optimized parsing:
- `align_pointer!` -- aligns pointer to 8-byte BPF boundary
- `advance_input_with_account!` -- advances buffer past account data + padding
- `process_n_accounts!` -- unrolls account parsing N times inline
- `process_accounts!` -- maps small counts (1–5) to the appropriate unrolled pattern

### What's Not Provided (that libraries like jiminy fill)

- No `error_code!` or equivalent for defining program errors
- No instruction dispatch macro
- No account validation/constraint macros  
- No PDA derivation helpers (beyond `Address` methods)
- No account layout definition macros
- No CPI guard macros

---

## Summary

Pinocchio is a **minimal, zero-copy, zero-dependency foundation** for Solana programs. It excels at raw performance -- CU optimization, binary size, and compile times -- by eliminating abstractions. The cost is extreme verbosity and manual safety management.

**Best for:**
- Performance-critical programs where every CU counts
- Small single-instruction programs
- Teams with deep Solana/BPF expertise who can manage unsafe code

**Needs supplementation for:**
- Account validation (jiminy, custom macros)
- Instruction dispatch (jiminy, hand-rolled)
- Client generation (none available)
- Testing (solana-program-test from main SDK)
- Serialization (manual or custom solution)
- Error code definitions (custom macros)

The ecosystem position is "the low-level layer" -- analogous to raw `libc` vs `std`. Libraries like jiminy build the safety and convenience layer on top while preserving pinocchio's zero-copy performance characteristics.
