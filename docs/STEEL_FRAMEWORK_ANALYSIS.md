# Steel Framework -- Deep Source Code Analysis

**Version analyzed**: 4.0.4 (`regolith-labs/steel` @ `c6d1a79`, master)  
**Solana SDK**: `^2.1` (solana-program)  
**Author**: Regolith Labs (ORE Supply team)  
**License**: Apache-2.0  

---

## 1. Architecture

### Core Abstraction Model

Steel is a **thin, opinionated library** -- not a code generator. It sits directly on top of `solana_program` and provides:

1. **Declarative macros** (`account!`, `instruction!`, `error!`, `event!`) that wire up discriminators and serialization
2. **Extension traits** on `AccountInfo` for chainable validation and deserialization
3. **Helper free functions** for CPIs (system program, SPL token)
4. **A CLI** (`steel-cli`) for project scaffolding

There are **zero proc macros**. Everything is `macro_rules!`. There is no IDL generation, no code generation step, and no build-time analysis.

### Recommended Project Structure

Steel enforces a **two-crate workspace pattern** via its CLI template:

```
workspace/
├── api/              # Interface/definition crate (shared by program + clients)
│   └── src/
│       ├── lib.rs          # declare_id!, re-exports
│       ├── consts.rs       # PDA seeds, constants
│       ├── error.rs        # Custom error enum
│       ├── instruction.rs  # Instruction discriminator enum + data structs
│       ├── sdk.rs          # Client-side instruction builders
│       └── state/
│           ├── mod.rs      # Account discriminator enum, PDA helpers
│           └── counter.rs  # Individual account struct definitions
├── program/          # On-chain program crate
│   └── src/
│       ├── lib.rs          # entrypoint!, dispatch via match
│       ├── initialize.rs   # One file per instruction handler
│       └── add.rs
```

This is **recommended, not enforced** -- Steel is a library, not a framework that generates code.

### Program Entrypoint

```rust
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction::<MyInstruction>(&my_api::ID, program_id, data)?;
    match ix {
        MyInstruction::Add => process_add(accounts, data)?,
        MyInstruction::Initialize => process_initialize(accounts, data)?,
    }
    Ok(())
}
entrypoint!(process_instruction);
```

`entrypoint!` is just `solana_program::entrypoint` -- re-exported verbatim. No wrapping.

---

## 2. Zero-Copy Approach

### Strategy: bytemuck (Pod + Zeroable)

Steel uses **bytemuck** for zero-copy account access. All account data structs must derive `Pod + Zeroable` and be `#[repr(C)]`.

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Counter {
    pub value: u64,
}
```

### Deserialization Path

The core trait is `AccountDeserialize`:

```rust
pub trait AccountDeserialize {
    fn try_from_bytes(data: &[u8]) -> Result<&Self, ProgramError>;
    fn try_from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError>;
}
```

Blanket impl for all `T: Discriminator + Pod`:

```rust
impl<T: Discriminator + Pod> AccountDeserialize for T {
    fn try_from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        if Self::discriminator().ne(&data[0]) {
            return Err(ProgramError::InvalidAccountData);
        }
        bytemuck::try_from_bytes::<Self>(&data[8..])
            .or(Err(ProgramError::InvalidAccountData))
    }
    // ...mut version identical with try_from_bytes_mut
}
```

**Key detail**: Discriminator is checked at `data[0]`, but the struct data starts at `data[8..]`. This means there are **7 unused/reserved bytes** between the discriminator byte and the struct body.

### How `as_account` / `as_account_mut` Work

These are the main entry points developers use. Defined on `AccountInfo`:

```rust
impl AsAccount for AccountInfo<'_> {
    fn as_account<T: AccountDeserialize + Discriminator + Pod>(
        &self, program_id: &Pubkey
    ) -> Result<&T, ProgramError> {
        unsafe {
            self.has_owner(program_id)?;
            let data = self.try_borrow_data()?;
            let expected_len = 8 + std::mem::size_of::<T>();
            if data.len() != expected_len { return Err(...); }
            T::try_from_bytes(std::slice::from_raw_parts(data.as_ptr(), expected_len))
        }
    }
}
```

**Critical unsafe usage**: `as_account` borrows data via `try_borrow_data()`, then creates a raw pointer slice via `std::slice::from_raw_parts` that escapes the `Ref` borrow guard. This effectively "leaks" the borrow -- the returned `&T` / `&mut T` outlives the `Ref`/`RefMut` guard. This is **technically unsound** (the `Ref` is dropped while the reference is still live), but works in practice within the SVM execution model because:
- Account data memory is stable for the duration of instruction processing
- The runtime doesn't move account data buffers

The mut version does the same with `try_borrow_mut_data()` + `from_raw_parts_mut`.

### Header Deserialization

For variable-length accounts (e.g., merkle trees with generic const size):

```rust
pub trait AccountHeaderDeserialize {
    fn try_header_from_bytes(data: &[u8]) -> Result<(&Self, &[u8]), ProgramError>;
    fn try_header_from_bytes_mut(data: &mut [u8]) -> Result<(&mut Self, &mut [u8]), ProgramError>;
}
```

Returns `(header_ref, remaining_bytes)` -- lets you parse a fixed header then interpret the tail based on header fields.

---

## 3. Account Validation

### Chainable Validation Pattern

Steel's signature design: all validation methods return `Result<&Self, ProgramError>`, enabling fluent chaining:

```rust
signer_info.is_signer()?;
counter_info
    .is_empty()?
    .is_writable()?
    .has_seeds(&[COUNTER], &my_api::ID)?;
```

### `AccountInfoValidation` (on `AccountInfo`)

| Method | Check |
|--------|-------|
| `is_signer()` | `self.is_signer == true` |
| `is_writable()` | `self.is_writable == true` |
| `is_executable()` | `self.executable == true` |
| `is_empty()` | `self.data_is_empty() == true` |
| `is_type::<T>(program_id)` | Owner matches + `data[0] == T::discriminator()` |
| `is_program(program_id)` | Address matches + is executable |
| `is_sysvar(sysvar_id)` | Owner is sysvar program + address matches |
| `has_address(address)` | `self.key == address` |
| `has_owner(program_id)` | `self.owner == program_id` |
| `has_seeds(seeds, program_id)` | `self.key == find_program_address(seeds, program_id).0` |

Every method uses `#[track_caller]` and logs the file/line on error via the `trace()` helper.

### `AccountValidation` (on deserialized account data)

After calling `as_account::<Counter>()`, you can chain assertions on the data itself:

```rust
counter_info
    .as_account_mut::<Counter>(&api::ID)?
    .assert_mut(|c| c.value < 100)?;
```

6 variants:
- `assert(closure)` / `assert_mut(closure)` -- default error
- `assert_err(closure, err)` / `assert_mut_err(closure, err)` -- custom ProgramError
- `assert_msg(closure, msg)` / `assert_mut_msg(closure, msg)` -- custom message string

All return `Result<&Self, ProgramError>` or `Result<&mut Self, ProgramError>`.

### PDA Validation via `has_seeds`

```rust
fn has_seeds(&self, seeds: &[&[u8]], program_id: &Pubkey) -> Result<&Self, ProgramError> {
    let pda = Pubkey::find_program_address(seeds, program_id);
    if self.key.ne(&pda.0) {
        return Err(trace("Account has invalid seeds", ProgramError::InvalidSeeds));
    }
    Ok(self)
}
```

**Note**: This calls `find_program_address` every time -- it does NOT accept a bump. This means every PDA check pays the full `find_program_address` compute cost (~1500 CU). There is no `has_seeds_with_bump` optimization.

---

## 4. Key Types and Traits

### Traits

| Trait | Module | Description |
|-------|--------|-------------|
| `Discriminator` | `account::deserialize` | `fn discriminator() -> u8` -- returns single-byte discriminator |
| `AccountDeserialize` | `account::deserialize` | `try_from_bytes` / `try_from_bytes_mut` -- bytemuck deserialize with discriminator check |
| `AccountHeaderDeserialize` | `account::deserialize` | Header + remainder pattern for variable-length accounts |
| `AccountInfoValidation` | `account::validation` | Chainable checks on `AccountInfo` (signer, writable, owner, seeds, etc.) |
| `AsAccount` | `account::validation` | `as_account::<T>` / `as_account_mut::<T>` -- owner + discriminator + bytemuck |
| `AccountValidation` | `account::validation` | `assert()` / `assert_mut()` closures on deserialized data |
| `CloseAccount` | `account::close` | `close(to)` -- drain lamports, assign system program, realloc to 0 |
| `LamportTransfer` | `account::lamports` | `send(lamports, to)` / `collect(lamports, from)` -- direct and CPI transfers |
| `Loggable` | `log` | `log()` / `log_return()` -- sol_log_data and set_return_data |
| `AsSpl` | `spl::validation` | `as_mint()` / `as_token_account()` / `as_associated_token_account()` on `AccountInfo` |

### Key Types

| Type | Description |
|------|-------------|
| `Mint` | `enum { V0(spl_token::Mint), V1(spl_token_2022::Mint) }` -- unified SPL/Token-2022 |
| `TokenAccount` | `enum { V0(spl_token::Account), V1(spl_token_2022::Account) }` -- unified |
| `Numeric` | `#[repr(C)] Pod` wrapper around I80F48 fixed-point (16 bytes) |

### Macros

| Macro | Type | What it generates |
|-------|------|-------------------|
| `account!(Enum, Struct)` | declarative | `impl Discriminator`, `impl AccountValidation`, `to_bytes()` method |
| `instruction!(Enum, Struct)` | declarative | `impl Discriminator`, `try_from_bytes()`, `to_bytes()` (with discriminator prefix) |
| `error!(Enum)` | declarative | `impl From<E> for ProgramError` (Custom(e as u32)) |
| `event!(Struct)` | declarative | `impl Loggable`, `to_bytes()`, `from_bytes()` |
| `entrypoint!` | re-export | Direct re-export of `solana_program::entrypoint!` |
| `impl_to_bytes!` | declarative | Adds `to_bytes() -> &[u8]` via `bytemuck::bytes_of` |
| `impl_from_bytes!` | declarative | Adds `from_bytes(&[u8]) -> &Self` via `bytemuck::from_bytes` |
| `impl_instruction_from_bytes!` | declarative | Adds `try_from_bytes(&[u8]) -> Result<&Self>` for instruction data |

---

## 5. Strengths

### What Steel Does Well

1. **Radical simplicity**: ~800 lines of library code total. No proc macros, no codegen, no build step. Easy to audit and understand completely.

2. **Explicit control flow**: No hidden magic. The developer writes `process_instruction` manually, matches on instruction variants manually, and calls validation methods explicitly. You always know what's happening.

3. **Chainable validation API**: Elegant pattern that reads cleanly and is hard to forget a check:
   ```rust
   counter_info.is_writable()?.has_seeds(&[COUNTER], &api::ID)?;
   ```

4. **bytemuck zero-copy**: Account data is cast in-place via bytemuck -- no deserialization allocations. Competitive with raw pinocchio for read/write performance.

5. **Unified SPL Token / Token-2022 handling**: The `Mint` and `TokenAccount` enums automatically handle both token programs, which is a real pain point in other frameworks.

6. **Strong error reporting**: `#[track_caller]` on every validation method + `trace()` logging gives exact file:line in error messages. Much better than Anchor's generic constraint errors.

7. **Two-crate pattern**: Clean separation between API (types/SDK) and program (logic). Client code can depend on `api` without pulling in program code.

8. **Low dependency surface**: bytemuck, num_enum, thiserror, solana-program, spl-token crates. No custom proc macro crates.

9. **Header deserialization**: `AccountHeaderDeserialize` for variable-length account patterns is a thoughtful addition not found in most frameworks.

10. **Production-tested**: Powers ORE (Regolith Labs' mining protocol), which handles significant mainnet transaction volume.

---

## 6. Weaknesses / Complaints

### Critical Issues

1. **Unsound `unsafe` in `as_account` / `as_account_mut`**: The raw pointer escape from `Ref`/`RefMut` is undefined behavior by Rust's aliasing rules. The `Ref` guard is dropped while a reference derived from it is still alive. Works in practice on SVM but is technically unsound and would fail under Miri.

2. **No bump caching in `has_seeds`**: Every call to `has_seeds()` invokes `Pubkey::find_program_address()` which iterates through bump values. At ~1500 CU per call, this is wasteful for programs that validate many PDAs. There's no `has_seeds_with_bump(seeds, bump, program_id)` variant.

3. **7 wasted bytes per account**: The memory layout uses `data[0]` for discriminator but starts struct data at `data[8..]`, leaving bytes 1–7 completely unused. This wastes 7 bytes of rent-exempt lamports per account for no apparent reason. Likely reserved for future use but currently dead space.

4. **No IDL generation**: The README TODO literally says "tip: ask Cursor to generate one." Programs built with Steel have no standard way to produce an IDL for client consumption. This is a significant gap for ecosystem tooling (explorers, SDKs, etc.).

5. **No realloc support**: There's no helper for account reallocation. `CloseAccount` uses `self.realloc(0, true)` internally but there's no `grow_account` or `resize_account` utility.

6. **Single-byte discriminator**: Only 256 possible account types and 256 possible instructions per program. Anchor uses 8-byte SHA-256 discriminators. While 256 is enough for most programs, it provides zero collision resistance for cross-program type safety.

### Design Limitations

7. **No compile-time account list validation**: Unlike Anchor's `#[derive(Accounts)]`, Steel doesn't generate the account list destructuring. The developer writes `let [a, b, c] = accounts else { ... }` manually. If you forget to check a field or get the order wrong, there's no compile-time safety net.

8. **Instruction data as `[u8; N]` for multi-byte values**: Because bytemuck requires `Pod`, you can't use `u64` directly in instruction data on some architectures. Steel's template uses `pub amount: [u8; 8]` with manual `u64::from_le_bytes()` conversion. This is error-prone.

9. **No account serialization on write**: bytemuck casts are in-place, so writes go through directly. But there's no "flush" or "commit" concept -- mutations are immediately visible in the account data buffer. This is actually a feature for performance but can surprise developers used to Anchor's `exit()` serialization model.

10. **`Mint::assert_mut` panics**: The `AccountValidation` impl for `Mint` has `assert_mut` methods that `panic!("not implemented")`. This is a runtime bomb if anyone tries to use mutable assertions on mints.

11. **No sysvar deserialization helpers**: No wrappers for Clock, Rent, etc. beyond basic `is_sysvar()` address check.

12. **CPI helpers always `Vec::with_capacity` + allocate**: The `invoke_signed` and `invoke_signed_with_bump` functions create a `Vec` to combine seeds with bump. This heap allocation on every CPI could be avoided with a fixed-size array approach.

13. **No compute budget management**: No helpers for requesting additional compute units or setting priority fees.

---

## 7. CPI Handling

### System Program CPIs

```rust
// Create account (non-PDA, via invoke)
create_account(from, to, system_program, space, owner) -> ProgramResult

// Create PDA account (with auto-discriminator set)
create_program_account::<T>(target, system_program, payer, owner, seeds) -> ProgramResult
create_program_account_with_bump::<T>(..., bump) -> ProgramResult

// Raw allocation (handles existing balance edge case)
allocate_account(target, system_program, payer, space, owner, seeds) -> ProgramResult
allocate_account_with_bump(..., bump) -> ProgramResult
```

`allocate_account_with_bump` has a nice pattern: if the target account already has lamports (e.g., from a prior transfer), it handles the three-step process: (1) top up rent, (2) allocate, (3) assign. If lamports are 0, it uses a single `create_account` CPI.

### SPL Token CPIs (feature-gated behind `spl`)

All wrapped as free functions (not traits):

| Function | Description |
|----------|-------------|
| `transfer(authority, from, to, program, amount)` | Token transfer (deprecated Transfer) |
| `transfer_signed(...)` + `..._with_bump(...)` | PDA-signed variant |
| `transfer_checked(...)` + signed variants | Transfer with decimal check |
| `mint_to_signed(...)` + `..._with_bump(...)` | Mint tokens |
| `mint_to_checked_signed(...)` | Checked mint |
| `burn(...)` + signed variants | Burn tokens |
| `burn_checked(...)` + signed variants | Checked burn |
| `freeze(...)` + signed variants | Freeze token account |
| `create_associated_token_account(...)` | Create ATA via ATA program CPI |
| `close_token_account(...)` + signed variants | Close token account |

Every signed variant has a `_with_bump` version that avoids re-deriving the PDA.

### Generic CPI

```rust
invoke_signed(instruction, account_infos, program_id, seeds) -> ProgramResult
invoke_signed_with_bump(instruction, account_infos, seeds, bump) -> ProgramResult
```

Both allocate a `Vec` to append the bump byte to the seeds slice.

---

## 8. Error Handling

### Error Definition

```rust
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum MyError {
    #[error("You did something wrong")]
    Dummy = 0,
}
error!(MyError);
```

The `error!` macro generates:
```rust
impl From<MyError> for ProgramError {
    fn from(e: MyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
```

### Error Propagation

- All validation methods return `Result<&Self, ProgramError>` -- standard `?` propagation
- The `trace()` helper logs `"{msg}: {file}:{line}"` via `sol_log` and returns the error
- `#[track_caller]` on every validation method means the log shows the **caller's** location, not Steel's internal code

### Error messages

Steel's error messages include the actual values in comparison failures:
```
"Account has invalid address 5xK3... != 7mN2..."
"Account has invalid owner 11111... != MyProgram..."
"Account data length is invalid 40 != 48"
```

This is significantly more helpful than Anchor's generic constraint errors.

---

## 9. Memory Layout

### Account Data Layout

```
[ disc: u8 | reserved: 7 bytes | struct_data: sizeof::<T>() bytes ]
|--- 8 bytes header ---|------------ N bytes body --------------|
```

- **Byte 0**: Single-byte discriminator (enum variant value)
- **Bytes 1–7**: Unused/zeroed (reserved?)
- **Bytes 8..8+sizeof(T)**: The `#[repr(C)]` Pod struct data, accessed via bytemuck cast

Total account size: `8 + std::mem::size_of::<T>()`

### Why 8-Byte Header?

Likely for alignment purposes -- keeping the struct body 8-byte aligned regardless of what the discriminator byte does. This matches Anchor's 8-byte discriminator size, though Anchor actually uses all 8 bytes for a SHA-256 hash prefix.

### Struct Requirements

All account structs must be:
- `#[repr(C)]` -- C-compatible layout, no field reordering
- `Pod` -- no padding bytes, all fields must be Pod
- `Zeroable` -- safe to represent as all zeros
- `Copy + Clone` -- required by Pod

### Instruction Data Layout

```
[ disc: u8 | struct_data: bytemuck::bytes_of(self) ]
```

Instructions use a **1-byte** discriminator (no padding) -- the struct data immediately follows byte 0. Contrast with accounts which have 8-byte headers.

The `instruction!` macro's `to_bytes()`:

```rust
pub fn to_bytes(&self) -> Vec<u8> {
    [
        [DiscriminatorEnum::StructName as u8].to_vec(),
        bytemuck::bytes_of(self).to_vec(),
    ].concat()
}
```

This allocates (client-side only, not on-chain).

---

## 10. Macro System

### All Declarative -- Zero Proc Macros

Steel uses **only `macro_rules!`** macros. This is a deliberate design choice:
- No proc macro compile time overhead
- No syn/quote dependency for macro expansion
- Simpler to audit -- you can read the macro and see exactly what it expands to
- No custom derive complexity

### `account!(EnumName, StructName)`

Expands to:
1. `impl Discriminator for StructName` -- returns `EnumName::StructName.into()` (u8)
2. `impl AccountValidation for StructName` -- all 6 assert/assert_mut/assert_err/assert_msg variants
3. `to_bytes() -> &[u8]` method via `bytemuck::bytes_of`

The `AccountDeserialize` impl is **not** generated by the macro -- it comes from the blanket impl on all `T: Discriminator + Pod`.

### `instruction!(EnumName, StructName)`

Expands to:
1. `impl Discriminator for StructName` -- returns `EnumName::StructName as u8`
2. `try_from_bytes(&[u8]) -> Result<&Self, ProgramError>` -- bytemuck conversion
3. `to_bytes() -> Vec<u8>` -- discriminator byte + struct bytes (allocates, for client use)

### `error!(EnumName)`

Expands to: `impl From<EnumName> for ProgramError` (Custom variant)

### `event!(StructName)`

Expands to:
1. `impl Loggable` -- `log()` calls `sol_log_data`, `log_return()` calls `set_return_data`
2. `to_bytes()` / `from_bytes()` methods

---

## 11. Instruction Dispatch

### `parse_instruction` -- The Dispatch Function

```rust
pub fn parse_instruction<'a, T: TryFrom<u8>>(
    api_id: &'a Pubkey,
    program_id: &'a Pubkey,
    data: &'a [u8],
) -> Result<(T, &'a [u8]), ProgramError> {
    if program_id.ne(&api_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    let (tag, data) = data.split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    let ix = T::try_from(*tag)
        .or(Err(ProgramError::InvalidInstructionData))?;
    Ok((ix, data))
}
```

Steps:
1. Verify `program_id` matches the expected API program ID
2. Split first byte as discriminator
3. Convert byte to instruction enum via `TryFrom<u8>` (from `num_enum::TryFromPrimitive`)
4. Return `(enum_variant, remaining_data_slice)`

The developer then manually `match`es on the enum variant:

```rust
match ix {
    MyInstruction::Add => process_add(accounts, data)?,
    MyInstruction::Initialize => process_initialize(accounts, data)?,
}
```

### Instruction Data Parsing (in handlers)

```rust
pub fn process_add(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = Add::try_from_bytes(data)?;  // bytemuck cast of remaining bytes
    let amount = u64::from_le_bytes(args.amount);  // manual conversion
    // ...
}
```

---

## 12. Account Lifecycle

### Creation (Initialize)

```rust
// 1. Validate the account is empty and has correct PDA
counter_info.is_empty()?.is_writable()?.has_seeds(&[COUNTER], &api::ID)?;

// 2. Create the PDA account with discriminator
create_program_account::<Counter>(
    counter_info,     // target 
    system_program,   // system program
    signer_info,      // payer
    &api::ID,         // owner (your program)
    &[COUNTER],       // PDA seeds
)?;

// 3. Initialize fields
let counter = counter_info.as_account_mut::<Counter>(&api::ID)?;
counter.value = 0;
```

`create_program_account::<T>()` internally:
1. Calls `allocate_account_with_bump` with space `8 + sizeof::<T>()`
2. Sets `data[0] = T::discriminator()`

### Mutation

```rust
let counter = counter_info
    .as_account_mut::<Counter>(&api::ID)?
    .assert_mut(|c| c.value < 100)?;
counter.value += amount;
// No flush needed -- writes go directly to account data buffer via bytemuck
```

### Closing

```rust
impl CloseAccount for AccountInfo<'_> {
    fn close(&self, to: &AccountInfo<'info>) -> Result<(), ProgramError> {
        **to.lamports.borrow_mut() += self.lamports();
        **self.lamports.borrow_mut() = 0;
        self.assign(&system_program::ID);
        self.realloc(0, true)?;
        Ok(())
    }
}
```

Steps:
1. Transfer all lamports to recipient
2. Set lamports to 0
3. Assign owner back to system program
4. Realloc data to 0 bytes

Also available as a free function: `close_account(account, recipient)`.

### Lamport Transfers

Direct (program-owned accounts):
```rust
account.send(lamports, &recipient);  // direct lamport manipulation, no CPI
```

CPI (user-owned accounts):
```rust
account.collect(lamports, &from);  // system_instruction::transfer CPI
```

---

## Summary Comparison: Steel vs Anchor vs Pinocchio/Jiminy

| Aspect | Steel | Anchor | Pinocchio/Jiminy |
|--------|-------|--------|------------------|
| **Macro type** | `macro_rules!` only | Proc macros (heavy) | `macro_rules!` only |
| **IDL** | None | Auto-generated | None (planned) |
| **Discriminator** | 1 byte (u8 enum) | 8 bytes (SHA-256 prefix) | 1 byte (layout ID) |
| **Account header** | 8 bytes (1 used + 7 wasted) | 8 bytes (all used) | Variable (jiminy: 1+ bytes) |
| **Zero-copy** | bytemuck Pod cast | Optional (borsh default) | Raw pointer arithmetic |
| **Account validation** | Chainable trait methods | Declarative `#[account]` constraints | Manual checks |
| **CPI** | Helper free functions | CpiContext pattern | Raw invoke |
| **Compile-time safety** | Low (runtime checks) | High (proc macro generated) | Low (runtime checks) |
| **Code generation** | None | Heavy | None |
| **Solana SDK** | solana-program | anchor-lang (wraps solana-program) | pinocchio (no solana-program) |
| **Compute efficiency** | Good (bytemuck) | Moderate (borsh overhead) | Best (zero abstraction) |
| **Lines of library code** | ~800 | ~20,000+ | Varies |

### Key Design Insight

Steel occupies the middle ground: more ergonomic than raw solana_program, but without Anchor's compile-time guarantees or code generation. It's essentially "solana_program with good extension traits and patterns." The chainable validation API is its most distinctive and well-designed feature.
