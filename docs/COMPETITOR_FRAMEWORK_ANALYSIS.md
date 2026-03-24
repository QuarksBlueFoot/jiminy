# Solana Zero-Copy Framework Analysis: Quasar & Star Frame

> **Research date**: 2026-03-27  
> **Sources**: Direct source code analysis of cloned repositories  
> **Quasar**: `blueshift-gg/quasar` @ GitHub, crates.io v0.0.0 (all crates), `quasar-lang`, `quasar-derive`, `quasar-pod`, `quasar-spl`, `quasar-idl`, `quasar-profile`  
> **Star Frame**: `staratlasmeta/star_frame` @ GitHub, crates.io v0.30.0, `star_frame`, `star_frame_proc`, `star_frame_idl`, `star_frame_spl`, `star_frame_cli`

---

## Executive Summary

Both **Quasar** and **Star Frame** are production-oriented Solana program frameworks that compete directly in the zero-copy space. Neither is a toy project — both have sophisticated designs with clear engineering investment. Here's how they position:

| Dimension | Quasar | Star Frame | Jiminy |
|---|---|---|---|
| **Philosophy** | "Anchor UX, hand-written CU" | "Traits and types all the way down" | "Deterministic ABI, zero-copy standard" |
| **Runtime** | Pinocchio (solana-account-view) | Pinocchio 0.9.2 | Pinocchio |
| **Zero-copy** | Pointer-cast `#[repr(C)]` + inline dynamic | bytemuck `Pod` + custom `UnsizedType` system | `#[repr(C)]` declarative macros, align-1 |
| **Dynamic data** | Inline prefix fields (`String<P,N>`, `Vec<T,P,N>`) | `unsized_type` system (List, Map, UnsizedString, Set) | Segmented layouts (`segmented_layout!`) |
| **Proc macros** | Yes — `#[account]`, `#[program]`, `#[derive(Accounts)]` | Yes — `#[derive(StarFrameProgram)]`, `#[derive(AccountSet)]`, `#[unsized_type]` | No — declarative macros only |
| **Discriminator** | Developer-specified bytes, variable length | Configurable type (default 8-byte sighash) via `AccountDiscriminant` | Deterministic `layout_id` (SHA-256) |
| **Cross-program ABI** | No standard, `Interface<T>` for multi-program | No cross-program type safety | `jiminy_interface!`, tiered trust model |
| **Maturity** | Beta (v0.0.0), under active development | Production (v0.30.0), 30+ releases, Star Atlas production | Production (v0.7.0) |
| **CU efficiency** | Near hand-written (direct syscall CPI) | 60-93% less than Anchor | Near hand-written |

**Key finding**: Quasar is the closest competitor to Jiminy in design philosophy (both prioritize CU minimization and zero-copy purity). Star Frame is the most feature-complete alternative (unsized types, full IDL, CLI). Neither has Jiminy's deterministic layout_id or cross-program interface system.

---

## 1. Quasar Framework — Deep Analysis

### 1.1 Architecture

**Crate structure** (monorepo):

| Crate | Purpose | Dependencies |
|---|---|---|
| `quasar-lang` | Runtime: account types, CPI, events, sysvars, error handling | `solana-account-view 2.0`, `solana-address 2.2`, `solana-instruction-view 2.0`, `quasar-derive`, `quasar-pod` |
| `quasar-derive` | Proc macros: `#[account]`, `#[program]`, `#[derive(Accounts)]`, `#[instruction]`, `#[error_code]`, `#[event]` | `syn 2`, `quote 1`, `proc-macro2` |
| `quasar-pod` | Alignment-1 integer types: `PodU8..PodU128`, `PodI8..PodI128`, `PodBool` | `#![no_std]`, optional `wincode` feature |
| `quasar-spl` | SPL Token + Token-2022 zero-copy types and CPI | `quasar-lang` |
| `quasar-idl` | IDL JSON generator + TypeScript/Rust client codegen + discriminator collision detection | `syn`, `serde` |
| `quasar-profile` | Static CU profiler — parses SBF ELF, walks DWARF, produces flamegraph | `memmap2`, `sha2` |
| `cli` | `quasar init/build/test/deploy/profile/dump` | Standard tooling |

**Design philosophy**: "Anchor-compatible ergonomics, hand-written CU efficiency." The API surface intentionally mirrors Anchor (`#[program]`, `#[account]`, `#[derive(Accounts)]`, `Ctx<T>`) but the generated code compiles to near-raw Pinocchio performance.

**`#![no_std]`**: The entire `quasar-lang` crate is `no_std`. No heap allocations unless explicitly opted in via the `alloc` feature. This is a hard constraint — all account data flows through pointer casts, never through deserialization.

### 1.2 Zero-Copy Approach

Quasar has a **dual-path** zero-copy system:

#### Fixed Accounts (pointer-cast)

For accounts with only fixed-size fields:
```rust
#[account(discriminator = 1)]
pub struct Counter {
    pub authority: Address,
    pub count: PodU64,
}
```

The `#[account]` proc macro generates:
1. A `CounterZc` companion struct — `#[repr(C)]`, alignment 1, for pointer-cast overlay
2. `Discriminator` impl with the specified byte(s) as prefix
3. `Space` impl = `disc_len + sum(field_sizes)`
4. `Owner` impl binding to the enclosing program's `declare_id!()`
5. `Deref<Target = CounterZc>` / `DerefMut` — casts data pointer past discriminator to `&CounterZc`

Compile-time assertions verify alignment 1 for the ZC companion.

#### Dynamic Accounts (offset caching)

For accounts with `String<P,N>` or `Vec<T,P,N>` fields:
```rust
#[account(discriminator = 5)]
pub struct Profile<'a> {
    pub owner: Address,
    pub name: String<u16, 64>,
    pub bio: String<u16, 4096>,
    pub tags: Vec<Address, u8, 16>,
}
```

The macro generates:
1. `ProfileZc` — only the **fixed** portion (fields before the first dynamic field)
2. `Profile<'a>` — borrowing view with `__off: [u32; N-1]` array (N = number of dynamic fields)
3. A `parse()` function that walks prefix bytes once, caching cumulative offsets
4. O(1) accessors: `name() -> &str`, `tags() -> &[Address]`, `name_raw() -> RawEncoded<2>`
5. `set_inner()` / `set_dynamic_fields()` for writes with `MAX_DYNAMIC_TAIL` stack buffer

**Wire layout for dynamic accounts**:
```
[disc_bytes][fixed_field_1][fixed_field_2][u16:name_len][name_bytes][u16:bio_len][bio_bytes][u8:tags_count][tags_data]
                                          ↑ __off[0]               ↑ __off[1]             (implied from N-1 offsets)
```

**BUMP_OFFSET optimization**: When an account has a `bump: u8` field, the macro computes its byte offset at compile time and sets `Discriminator::BUMP_OFFSET = Some(offset)`. PDA validation then reads the bump directly from account data and calls `verify_program_address` (~200 CU) instead of `based_try_find_program_address` (~544 CU). This is a roughly **344 CU saving per PDA validation**.

### 1.3 Account Validation

Quasar validation is composable via the check traits:

| Trait | Purpose | Generated by |
|---|---|---|
| `checks::Signer` | `is_signer` flag check | `define_account!` |
| `checks::Mutable` | `is_writable` flag check | `define_account!` |
| `checks::Owner` | Owner address comparison | `impl Owner for T` |
| `checks::Address` | Exact address comparison | `#[account(address = ...)]` |
| `checks::Executable` | Executable flag check | `Program<T>` |
| `AccountCheck` | Custom runtime hook | User `impl` or `#[account(constraint = ...)]` |

**Batched header validation**: The generated `parse_accounts` function reads the first 4 bytes of each `RuntimeAccount` as a `u32` and compares against precomputed constants:
- `NODUP = 0xFF` (not borrowed, no flags)
- `NODUP_SIGNER = 0xFF | (1 << 8)` (not borrowed + signer)
- `NODUP_MUT = 0xFF | (1 << 16)` (not borrowed + writable)
- `NODUP_MUT_SIGNER = 0xFF | (1 << 8) | (1 << 16)` (not borrowed + signer + writable)

This collapses duplicate-check + signer-check + writable-check into a **single u32 comparison** per account — extremely CU efficient.

### 1.4 Key Types and Traits

**Core traits**:
- `Owner` — `const OWNER: Address`
- `Id` — `const ID: Address` (for program types)
- `Discriminator` — `const DISCRIMINATOR: &'static [u8]` + optional `BUMP_OFFSET`
- `Space` — `const SPACE: usize`
- `ParseAccounts` — parse + validate from raw `AccountView` slice, returns `(Self, Bumps)`
- `FromAccountView` — construct typed wrapper from single `AccountView`
- `AccountCheck` — runtime validation hook
- `CheckOwner` — owner validation (blanket impl for `Owner`, custom for interfaces)
- `AsAccountView` — unwrap back to raw view
- `StaticView` — marker for pointer-cast safe types
- `ZeroCopyDeref` — `Deref`/`DerefMut` to ZC companion
- `ProgramInterface` — multi-program address matching
- `AccountCount` — `const COUNT: usize` for dispatch buffer sizing

**Account wrappers**:
- `Account<T>` — owned, validated, `Deref<Target = TZc>` / `DerefMut`
- `Signer` — signer-checked, no data access
- `SystemAccount` — system-owned
- `Program<T>` — executable + address-checked
- `Interface<T>` — multi-program interface
- `UncheckedAccount` — no validation (opt-in dangerous)

**Context types**:
- `Context` — raw entrypoint data (program_id, accounts, data, remaining_ptr)
- `Ctx<T>` — parsed and validated accounts + bumps
- `CtxWithRemaining<T>` — like `Ctx` but captures remaining accounts for CPI

### 1.5 Macro System

**All proc macros** (in `quasar-derive`):

| Macro | Type | Generates |
|---|---|---|
| `#[account(discriminator = N)]` | Attribute | ZC companion, traits, accessors, dynamic codegen |
| `#[derive(Accounts)]` | Derive | `ParseAccounts`, `AccountCount`, `parse_accounts()`, bump struct |
| `#[program]` | Attribute | Instruction dispatch table, `Owner` binding |
| `#[instruction(discriminator = N)]` | Attribute | `Discriminator` for data parsing, arg codegen |
| `#[error_code]` | Attribute | Error enum with numbered variants, `Display` |
| `#[event]` | Attribute | `Event` trait, borsh encoding for log/self-CPI |

The `#[account]` macro classifies fields into `DynKind`:
- `DynKind::Fixed` — normal fixed-size field
- `DynKind::Str { prefix, max }` — `String<P, N>` marker type
- `DynKind::Vec { elem, prefix, max }` — `Vec<T, P, N>` marker type  
- `DynKind::Tail { element }` — prefix-less tail field consuming remaining bytes

**Validation rule**: Fixed fields MUST precede all dynamic fields. The macro validates this ordering.

### 1.6 Memory Layout

**Fixed account wire format**:
```
[discriminator: 1-N bytes][field_1][field_2]...[field_N]
```

**Dynamic account wire format**:
```
[discriminator][fixed_fields][prefix_1][data_1][prefix_2][data_2]...[prefix_N][data_N]
```

No alignment padding. No segment table. No version field. No layout_id.

- Discriminator: developer-specified, variable length (commonly 1 byte for CU efficiency)
- All-zero discriminator banned at compile time (prevents uninitialized data matching)
- `Space::SPACE` includes discriminator + fixed fields (for `create_account`)
- Dynamic accounts: MIN_SPACE = disc + fixed + sum(prefix_sizes), MAX_SPACE = MIN_SPACE + sum(max_data_sizes)

### 1.7 CPI Handling

Quasar's CPI is **directly optimized against the syscall ABI**:

1. `CpiCall<N, D>` — const-generic struct with account count (`N`) and data size (`D`) known at compile time. Everything stays on the stack.
2. `BufCpiCall` — variable-length variant for Borsh-serialized data, uses stack buffer up to `MAX_DYNAMIC_TAIL`.
3. `cpi_account_from_view()` — batched flag extraction reads the 4-byte RuntimeAccount header as u32, shifts right 8 to drop borrow_state, keeping `[is_signer, is_writable, executable]` in one operation.
4. `invoke_raw()` — goes directly to `sol_invoke_signed_c` syscall, bypassing `InstructionView::invoke_signed_unchecked`.
5. `RawEncoded` — zero-copy CPI pass-through for dynamic fields (memcpy raw prefix+data without decode/re-encode).

SPL CPI methods on `Program<Token>` and `TokenInterface`: `transfer`, `approve`, `burn`, `mint_to`, `close`, `freeze`, `thaw`, `sync_native`, `initialize_mint`, `initialize_account`, etc. All return `CpiCall` for chaining.

### 1.8 Instruction Dispatch

The `dispatch!` macro:
```rust
dispatch!(input_ptr, instruction_data, DISC_LEN, {
    [0] => handle_init(InitAccounts),
    [1] => handle_deposit(DepositAccounts),
    [2] => handle_withdraw(WithdrawAccounts),
});
```

1. Reads program_id from the end of instruction data (SVM layout)
2. Reads account count from offset 0 of input buffer
3. Matches discriminator (first N bytes) against compile-time byte patterns
4. Checks account count >= `AccountCount::COUNT` for the matched variant
5. Allocates `MaybeUninit<[AccountView; COUNT]>` on stack
6. Calls `parse_accounts()` to walk the SVM buffer and fill the array
7. Constructs `Context` and calls the handler

### 1.9 Strengths

1. **Extreme CU efficiency**: Batched u32 header validation, direct syscall CPI, bump offset optimization, `no_alloc!` entrypoint
2. **Familiar API**: Anchor-like syntax (`#[program]`, `Ctx<T>`, `#[derive(Accounts)]`) lowers migration barrier
3. **Dynamic field innovation**: Inline prefix fields with offset caching — space-efficient for 1-3 dynamic fields
4. **RawEncoded CPI pass-through**: Zero decode/re-encode for CPI forwarding of dynamic data 
5. **Comprehensive tooling**: CLI, IDL generator with collision detection, SBF binary profiler with flamegraph
6. **Multi-program interfaces**: `Interface<T>` + `ProgramInterface` trait for Token/Token-2022 polymorphism
7. **PDA optimization**: `BUMP_OFFSET` reading bump from account data saves ~344 CU per PDA check
8. **Event system**: Dual emission — log-based (~100 CU) and self-CPI (~1000 CU, unforgeable)

### 1.10 Weaknesses

1. **No cross-program ABI**: No layout_id, no deterministic discriminator, no cross-program type-safe reads
2. **No account versioning**: No version field in header, no migration path between layout versions
3. **v0.0.0 on crates.io**: All crates at version 0.0.0 — not yet published with real version numbers
4. **Proc macro dependency**: All codegen is in proc macros — harder to audit than declarative macros
5. **No segment table**: Dynamic fields lack explicit capacity tracking — realloc on every write that exceeds current allocation
6. **Limited dynamic collections**: Only `String<P,N>` and `Vec<T,P,N>` — no Map, Set, or nested dynamic types
7. **All-or-nothing parsing**: Dynamic accounts parse all field offsets upfront (no lazy dynamic field access)
8. **Stack buffer limit**: `MAX_DYNAMIC_TAIL = 2048` for dynamic writes — large accounts need `alloc` feature
9. **No Miri CI visible**: Claims Miri validation but no visible CI configuration for it
10. **No schema system**: IDL is generated from source, not from runtime-inspectable metadata

---

## 2. Star Frame — Deep Analysis

### 2.1 Architecture

**Crate structure** (monorepo):

| Crate | Purpose | Key Dependencies |
|---|---|---|
| `star_frame` | Runtime: account sets, instructions, program trait, unsized types, CPI, context | `pinocchio 0.9.2`, `bytemuck`, `borsh`, `ptr_meta`, `derive-where`, `typenum`, `fixed` |
| `star_frame_proc` | Proc macros: `StarFrameProgram`, `AccountSet`, `InstructionSet`, `InstructionArgs`, `ProgramAccount`, `unsized_type`, `unsized_impl`, `GetSeeds`, `Align1` | `syn 2`, `quote`, `proc-macro-error2` |
| `star_frame_idl` | IDL generation with Codama nodes, structural verification (fail-closed), verifier rules SFIDL001-011 | `codama-nodes 0.5.2`, `serde`, `semver` |
| `star_frame_spl` | SPL Token/Token-2022/ATA zero-copy types | `star_frame`, `spl-token-interface`, `spl-associated-token-account-interface` |
| `star_frame_cli` | `sf new` — project scaffolding with atomic staging directory, validation | Standard tooling |

**Design philosophy**: "Traits and types all the way down." Every concept is a trait with associated types. The instruction lifecycle has explicit phases: decode → validate → process → cleanup. Heavy use of Rust's type system for compile-time guarantees.

**Author**: Star Atlas Meta (gaming/metaverse company). This is a production framework used by Star Atlas's on-chain programs.

### 2.2 Zero-Copy Approach

Star Frame has **two distinct paths** for zero-copy:

#### Fixed Accounts (bytemuck Pod)

```rust
#[zero_copy(pod)]
#[derive(ProgramAccount, Default, Debug, Eq, PartialEq)]
pub struct CounterAccount {
    pub authority: Pubkey,
    pub count: u64,
}
```

`#[zero_copy(pod)]` expands to:
- `#[repr(C, packed)]`
- `unsafe impl Pod {}`
- `unsafe impl Zeroable {}`
- `unsafe impl Align1 {}`

`ProgramAccount` derive generates:
- `const DISCRIMINANT: <OwnerProgram>::AccountDiscriminant` — configurable discriminant type
- `validate_account_info()` — owner check + discriminant check with optimized unaligned reads

**Discriminant validation** is size-adaptive (borrowed from Typhoon):
- 1 byte: direct `*data_ptr == disc`
- 2 bytes: `read_unaligned::<u16>`
- 4 bytes: `read_unaligned::<u32>`
- 8 bytes: `read_unaligned::<u64>`
- N bytes: `slice::from_raw_parts` comparison

#### Unsized Accounts (UnsizedType system)

This is Star Frame's most innovative subsystem. The `unsized_type` proc macro + `UnsizedType` trait hierarchy enables **variable-sized zero-copy data structures** with runtime resizing:

```rust
#[unsized_type(program_account, seeds = CounterSeeds)]
pub struct CounterAccount {
    pub authority: Pubkey,
    #[unsized_start]
    pub count_tracker: UnsizedMap<Pubkey, PackedValue<u64>>,
}
```

**Core trait hierarchy**:

```
UnsizedType (core trait)
├── UnsizedTypePtr (pointer validation, check_pointers)
├── FromOwned (write owned values to buffer)
├── UnsizedInit (initialization with compile-time size guarantee)
├── RawSliceAdvance (raw pointer advancement)
└── UnsizedTypeDataAccess (data access, realloc)
```

**Available unsized types**:
- `List<T, L>` — dynamic array with configurable length type (u8/u16/u32/u64), binary search, insert, remove
- `Map<K, V, L>` — sorted key-value map backed by `List<ListItemSized<K,V>>`, O(log n) lookup
- `Set<T, L>` — sorted unique set backed by `List`
- `UnsizedString<L>` — variable-length UTF-8 string backed by `List<u8>`
- `UnsizedList<T, L>` — list of unsized elements
- `UnsizedMap<K, V, L>` — map where keys and/or values are unsized
- `RemainingBytes` — consume remaining buffer as raw bytes
- `Checked<T>` — wrapper that validates bit patterns via `CheckedBitPattern`

**Wrapper system** for safe access:
- `SharedWrapper<'_, T::Ptr>` — immutable view, holds borrow guard
- `ExclusiveWrapper<'_, T::Ptr, AccountInfo>` — mutable view with **realloc support**
- `ExclusiveRecurse` — nested mutable access for child fields

**Pointer safety**: The `check_pointers()` method on `UnsizedTypePtr` verifies that all internal pointers remain valid within the allocated range. This runs on `Drop` of `ExclusiveTopDrop` to catch invalidation. The system passes **Miri under Tree Borrows** with `ptr_meta` for fat-pointer construction.

### 2.3 Account Validation

Star Frame uses a multi-phase validation pipeline:

**Phase 1: Decode** (`AccountSetDecode`)
```rust
fn decode_accounts(accounts: &mut &'a [AccountInfo], decode_input: D, ctx: &mut Context) -> Result<Self>;
```
Takes accounts off the `&[AccountInfo]` slice, applies type-level decoding (discriminant check, owner check).

**Phase 2: Validate** (`AccountSetValidate`)
```rust
fn validate_accounts(&mut self, validate_input: V, ctx: &mut Context) -> Result<()>;
```
Applies `#[validate(...)]` attribute rules: `funder`, `arg = Create(...)`, `has_one`, Seeds, custom expressions.

**Phase 3: Process** (`StarFrameInstruction::process`)
User's instruction logic.

**Phase 4: Cleanup** (`AccountSetCleanup`)
```rust
fn cleanup_accounts(&mut self, cleanup_input: C, ctx: &mut Context) -> Result<()>;
```
Automatic rent normalization, account closing, refunding. Configurable via `#[cleanup(...)]` attributes.

**Modifier composition** (inner → outer wrapping):
- `Signer<Mut<Account<T>>>` — signer + mutable + typed account
- `Init<Seeded<Account<T>>>` — initialization + PDA seeded + typed  
- Each modifier adds its validation in `decode` or `validate` phase

### 2.4 Key Types and Traits

**Program traits**:
- `StarFrameProgram` — `type InstructionSet`, `type AccountDiscriminant: Pod`, `const ID: Pubkey`, `fn entrypoint()`, `fn handle_error()`
- `InstructionSet` — `type Discriminant: Pod`, `fn dispatch()`
- `Instruction` — `fn process_from_raw()`
- `StarFrameInstruction` — opinionated instruction: `type Accounts`, `type ReturnType: NoUninit`, `fn process()`
- `InstructionArgs` — splits struct into decode/validate/run/cleanup args
- `InstructionDiscriminant<IxSet>` — discriminant value for an instruction in a set

**Account set traits**:
- `AccountSetDecode<'a, D>` — decode from `&[AccountInfo]`
- `AccountSetValidate<V>` — validate with args
- `AccountSetCleanup<C>` — cleanup with args
- `TryFromAccounts<'a>` — convenience combining decode + validate
- `ProgramAccount` — discriminant + owner validation
- `SingleAccountSet` — marker for single-account types

**Account types**:
- `Account<T: ProgramAccount + UnsizedType>` — the core account type, handles both sized and unsized
- `BorshAccount<T>` — Borsh-deserialized accounts (slower, more flexible)
- `Signer<T>` — signer modifier
- `Mut<T>` — mutable modifier
- `Init<T>` — initialization modifier (create_account CPI)
- `Seeded<T>` — PDA seeded modifier
- `Program<T>` — program account
- `SystemAccount` — system-owned
- `Rest` — remaining accounts

**Data types**:
- `PackedValue<T>` — alignment-1 packed wrapper via `#[repr(C, packed)]`
- `PodBool` — `u8` boolean with Pod
- `KeyFor<T>` — typed pubkey wrapper (`Pubkey` with phantom type)
- `OptionalKeyFor<T>` — `Option<Pubkey>` as `Pubkey` (zero = None)
- `RemainingData` — raw bytes slice
- `UnitSystem<Unit, Value>` — typed numeric units (e.g., tokens, lamports)

### 2.5 Macro System

**All proc macros** (in `star_frame_proc`):

| Macro | Type | Generates |
|---|---|---|
| `#[derive(StarFrameProgram)]` | Derive | Entrypoint, program setup module, `StarFrameProgram` impl |
| `#[derive(InstructionSet)]` | Derive | Dispatch logic via discriminant matching |
| `#[derive(AccountSet)]` | Derive | `AccountSetDecode`, `AccountSetValidate`, `AccountSetCleanup`, CPI types |
| `#[derive(ProgramAccount)]` | Derive | `ProgramAccount` impl with discriminant and owner |
| `#[derive(InstructionArgs)]` | Derive | Split struct into lifecycle args |
| `#[star_frame_instruction]` | Attribute | Full `StarFrameInstruction` impl |
| `#[unsized_type]` | Attribute | `UnsizedType`, `UnsizedTypePtr`, init, IDL type generation |
| `#[unsized_impl]` | Attribute | Methods on unsized types with resize support |
| `#[derive(GetSeeds)]` | Derive | PDA seed derivation |
| `#[derive(Align1)]` | Derive | Alignment-1 marker trait |
| `#[zero_copy(pod)]` | Attribute | `repr(C, packed)`, `Pod`, `Zeroable`, `Align1` |

### 2.6 Memory Layout

**Fixed account wire format**:
```
[discriminant: N bytes][field_1][field_2]...[field_N]
```

Discriminant size is configurable per program via `StarFrameProgram::AccountDiscriminant`. Default is 8-byte Anchor-style sighash. Can be as small as `u8` or as large as needed.

**Unsized account wire format**:
```
[discriminant][fixed_fields][unsized_field_1_data][unsized_field_2_data]...
```

The unsized system uses the `UnsizedType::get_ptr()` method to walk the buffer and construct fat pointers (via `ptr_meta`) to each unsized field. Each field knows its own length encoding:
- `List<T, L>` — `L` bytes for length, then `len * size_of::<T>()` data bytes
- `Map<K, V, L>` — same as `List<ListItemSized<K,V>, L>`
- `UnsizedString<L>` — same as `List<u8, L>`

No alignment padding (all types require `Align1`). No segment table. No layout_id.

### 2.7 CPI Handling

Star Frame has a **statically-typed CPI builder**:

```rust
MyProgram::cpi(&MyInstruction { .. }, MyInstructionCpiAccounts { .. }, program_override)?.invoke()?;
```

The `AccountSet` derive generates a companion `CpiAccountSet` with:
- `CpiAccounts` type alias for the CPI account tuple
- `AccountLen` for compile-time account count
- `ContainsOption` for handling optional accounts (affects program_id passing)

CPI data is Borsh-serialized with the discriminant prepended. The builder handles:
- Static vs dynamic account array sizes (via `typenum`)
- Optional accounts (program AccountInfo required when set contains options)
- PDA signing via `invoke_signed`

**Key detail**: Star Frame uses `pinocchio::instruction::Instruction` and `AccountMeta` for CPI — not raw syscall. This is slightly higher overhead than Quasar's direct `sol_invoke_signed_c` approach.

### 2.8 Instruction Dispatch

Star Frame uses an enum-based dispatch via `InstructionSet`:

```rust
#[derive(InstructionSet)]
pub enum CounterInstructionSet {
    Initialize(Initialize),
    Increment(Increment),
}
```

The derived `dispatch()`:
1. Reads discriminant from instruction data (size = `size_of::<Discriminant>`)
2. Matches against each variant's `InstructionDiscriminant::DISCRIMINANT`
3. Calls `Instruction::process_from_raw()` for the matched variant

The `process_from_raw` default implementation (for `StarFrameInstruction`):
1. Borsh-deserialize instruction data
2. Split into lifecycle args via `InstructionArgs::split_to_args()`
3. Decode accounts via `AccountSetDecode`
4. Validate accounts via `AccountSetValidate`
5. Process instruction via `StarFrameInstruction::process()`
6. Cleanup via `AccountSetCleanup`
7. Set return data if `ReturnType` is non-zero-sized

**Discriminant system**: Default is Anchor-compatible 8-byte sighash. Programs can override to any `Pod` type. This makes Star Frame backward-compatible with existing Anchor clients.

### 2.9 Strengths

1. **Most advanced dynamic data**: `UnsizedType` system with List, Map, Set, UnsizedString, nested unsized types, runtime resize with pointer safety
2. **Production proven**: v0.30.0, 30+ releases, used by Star Atlas (real production gaming/metaverse)
3. **Massive CU savings**: 60-93% reduction vs Anchor across all benchmarks (per their own measurements)
4. **Typed CPI**: Compile-time verified CPI with generated `CpiAccounts` types
5. **4-phase instruction lifecycle**: decode → validate → process → cleanup with typed args per phase
6. **IDL with structural verification**: Fail-closed verifier (SFIDL001-011) catches IDL/code mismatch
7. **Comprehensive modifier system**: Composable `Signer<Mut<Init<Seeded<Account<T>>>>>` with compile-time validation
8. **Context caching**: Sysvar caching (rent, clock), funder/recipient caching across phases
9. **Codama integration**: IDL output using Codama nodes for multi-language client generation
10. **Fixed-point arithmetic**: Integrated `fixed` crate for financial math
11. **Miri-validated**: The unsized type system explicitly passes Miri under Tree Borrows

### 2.10 Weaknesses

1. **No cross-program ABI**: No layout_id, no standard cross-program type-safe reads
2. **Not `no_std`**: Uses `std` (Box, Vec, BTreeMap, String) — larger binary size, heap allocation
3. **Heavy proc macro dependency**: More proc macros than any other framework — audit burden is significant
4. **Borsh for instruction data**: Instruction args require `BorshDeserialize` — serialization overhead on every instruction
5. **Pinocchio 0.9.2**: Not yet updated to Pinocchio 0.10+ (no lazy entrypoint, no new account-view features)
6. **No deterministic discriminator**: Discriminants are configurable but not content-addressed or schema-derived
7. **Complexity**: The trait hierarchy is deep — `StarFrameInstruction` + `InstructionArgs` + `AccountSetDecode` + `AccountSetValidate` + `AccountSetCleanup` + CPI types + modifier composition. Learning curve is steep.
8. **`std` dependency disqualifies `no_alloc`**: Can't use `no_alloc!` entrypoint because `Box`, `Vec`, `BTreeMap` require allocator
9. **No raw CPI pass-through**: No equivalent to Quasar's `RawEncoded` for zero-copy CPI forwarding
10. **`fixed_star_frame`**: Forked the `fixed` crate (pending upstream MR) — maintenance burden

---

## 3. Comparison Matrix

### 3.1 Feature Comparison: All Frameworks

| Feature | Jiminy | Quasar | Star Frame | Steel | Anchor | Raw Pinocchio |
|---|---|---|---|---|---|---|
| **Zero-copy accounts** | ✅ align-1 overlay | ✅ pointer-cast ZC | ✅ bytemuck Pod | ✅ bytemuck | ❌ borsh | ✅ manual |
| **Dynamic/variable data** | Segments (12B/seg) | Inline prefix | UnsizedType system | ❌ | ❌ | Manual |
| **Deterministic layout_id** | ✅ SHA-256 | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Cross-program ABI** | ✅ interfaces + tiers | ❌ | ❌ | ❌ | IDL-based | ❌ |
| **Account versioning** | ✅ extends + compat | ❌ | ❌ | ❌ | ❌ | ❌ |
| **`no_std` / `no_alloc`** | ✅ | ✅ | ❌ (uses `std`) | ❌ (`solana_program`) | ❌ | ✅ |
| **Proc macros** | ❌ (declarative only) | ✅ heavy | ✅ very heavy | ❌ (`macro_rules!`) | ✅ very heavy | ❌ |
| **IDL generation** | jiminy-schema | quasar-idl | star_frame_idl (Codama) | ❌ | ✅ (anchor-idl) | ❌ |
| **CLI** | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ |
| **SBF profiler** | ❌ | ✅ flamegraph | ❌ | ❌ | ❌ | ❌ |
| **Event system** | ❌ | ✅ log + self-CPI | ❌ | ❌ | ✅ | ❌ |
| **CPI builder** | Manual | ✅ const-generic | ✅ typed + Borsh | ✅ helpers | ✅ | Manual |
| **Multi-program interface** | ❌ (single owner) | ✅ `Interface<T>` | Token/Token-2022 via SPL | ❌ | ✅ | ❌ |
| **Instruction dispatch** | Manual match | `dispatch!` macro | `InstructionSet` enum | Manual match | `#[program]` | Manual match |
| **Return data** | ❌ | ❌ | ✅ typed `ReturnType` | ❌ | ❌ | Manual |
| **CU overhead** | ~Pinocchio | ~Pinocchio +5-10% | ~Pinocchio +10-30% | ~solana_program | ~2-10x Pinocchio | Baseline |
| **Discriminator** | SHA-256 layout_id | Dev-specified bytes | Configurable Pod type | 1-byte enum | 8-byte sighash | Manual |
| **Test framework** | Mollusk | CLI + fixtures | Mollusk | CLI | Bankrun/validator | Manual |
| **Collections** | ZeroCopySlice | Vec<T,P,N> | List, Map, Set, UnsizedMap | ❌ | ❌ | ❌ |
| **Strings** | ❌ | String<P,N> | UnsizedString | ❌ | ❌ | ❌ |

### 3.2 CU Efficiency Comparison

Based on benchmarks and source analysis:

| Operation | Raw Pinocchio | Quasar | Jiminy | Star Frame | Steel | Anchor |
|---|---|---|---|---|---|---|
| Account parse (1) | ~50 CU | ~55 CU | ~60 CU | ~170 CU | ~300 CU | ~600 CU |
| Account parse (8) | ~400 CU | ~420 CU | ~450 CU | ~480 CU | ~2400 CU | ~3000 CU |
| PDA validation | ~544 CU | ~200 CU* | ~544 CU | ~544 CU | ~1500 CU | ~1500 CU |
| CPI invoke | ~Syscall | ~Syscall | ~Syscall | +Borsh overhead | +solana_program | +Borsh+alloc |
| Account init | ~1800 CU | ~1900 CU | ~1900 CU | ~2000 CU | ~3000 CU | ~5000 CU |

*Quasar's bump offset optimization saves ~344 CU per PDA check.

Star Frame's benchmark numbers (166-481 CU for 1-8 accounts) are **extremely** impressive — but note these are for their optimized paths, and the 60-93% reduction is vs Anchor, not vs raw Pinocchio.

### 3.3 Dynamic Data Comparison

| Aspect | Jiminy Segments | Quasar Inline | Star Frame Unsized |
|---|---|---|---|
| **Overhead per dynamic field** | 12 bytes (descriptor) | 1-4 bytes (prefix) | 1-4 bytes (length) |
| **Max array fields** | 8 segments | Unlimited (walk cost grows) | Unlimited (pointer walk) |
| **Capacity tracking** | ✅ explicit | ❌ (realloc on write) | ❌ (realloc on write) |
| **Cross-program readable** | ✅ (self-describing) | ❌ (requires codec) | ❌ (requires codec) |
| **Collection types** | ZeroCopySlice | Vec | List, Map, Set, UnsizedMap, UnsizedString |
| **Nested dynamic** | ❌ | ❌ | ✅ (UnsizedList, UnsizedMap) |
| **Runtime resize** | Manual realloc | Stack buffer + realloc | ✅ `ExclusiveWrapper` auto-resize |
| **Binary search** | ❌ | ❌ | ✅ (List, Map, Set) |
| **Insert/Remove** | Manual | set_inner (full rewrite) | ✅ O(n) shift operations |
| **Memory safety** | Bounds-checked | Bounds-checked | Miri-validated pointer safety |
| **Raw CPI forwarding** | ❌ | ✅ RawEncoded | ❌ |

---

## 4. Other Ecosystem Frameworks

### 4.1 Typhoon (exotic-markets-labs)
- **Version**: 0.2.2, `#![no_std]`, MIT/Apache-2.0
- **Repo**: `exotic-markets-labs/typhoon`
- **Approach**: Lightweight pinocchio wrapper with discriminator optimized validation (Star Frame credits Typhoon for their discriminator check pattern)
- **Key innovation**: Size-adaptive discriminant comparison (1/2/4/8 byte fast paths)
- **Status**: Active development, minimal but focused

### 4.2 Pina (pina-rs)
- **Version**: 0.6.0
- **Repo**: `pina-rs/pina`
- **Approach**: Pinocchio framework with Codama-based code generation (`pina_codama_renderer`)
- **Key innovation**: Codama integration for multi-language client SDKs from IDL
- **Status**: Active, has Codama renderer for bytemuck models

### 4.3 Shank (Metaplex)
- **Version**: 0.4.8
- **Approach**: NOT a framework — IDL extraction from annotated Rust code
- **Used by**: Metaplex programs, SPL programs
- **Key trait**: `ShankAccount`, `ShankInstruction` annotations for IDL extraction
- **Status**: Mature, maintained by Metaplex

### 4.4 Codama
- **Version**: 0.8.0
- **Approach**: NOT a framework — universal IDL → multi-language client generator
- **Supports**: Rust, TypeScript, Python, Java client generation from Codama IDL nodes
- **Used by**: Star Frame (via `star_frame_idl`), Pina
- **Status**: Active, becoming the ecosystem standard for client generation

### 4.5 Bolt (MagicBlock)
- **Version**: 0.2.4
- **Approach**: Entity Component System (ECS) on Solana, built ON TOP of Anchor
- **Key dependency**: `anchor-lang`
- **Not a zero-copy framework** — uses Borsh through Anchor
- **Status**: Active, focused on gaming (ephemeral rollups)

---

## 5. Strategic Analysis for Jiminy

### 5.1 Where Jiminy Uniquely Leads

1. **Deterministic layout_id**: The ONLY framework with content-addressed ABI fingerprinting. Quasar has developer-specified discriminators. Star Frame has configurable discriminants. Neither can detect schema changes automatically.

2. **Cross-program type safety**: `jiminy_interface!` with tiered trust (5 tiers) is completely unmatched. No other framework even attempts standardized cross-program account reading.

3. **Declarative-only macros**: The ONLY framework that avoids proc macros entirely. This is a genuine auditability advantage — proc macros are essentially code-executing black boxes during compilation.

4. **Segment capacity tracking**: Jiminy's `SegmentDescriptor` with explicit capacity is unique. All other frameworks (Quasar, Star Frame) realloc on demand without capacity awareness.

5. **`no_std` + `no_alloc` purity**: Only Jiminy and Quasar achieve this. Star Frame's `std` dependency is a concrete disadvantage for binary size.

### 5.2 Where Jiminy Should Learn

| From | Innovation | Priority |
|---|---|---|
| **Quasar** | Inline dynamic fields (prefix + offset caching) for 1-2 field case | HIGH — already identified in ecosystem report |
| **Quasar** | `BUMP_OFFSET` compile-time PDA optimization (~344 CU saving) | HIGH — easy to implement, big CU win |
| **Quasar** | `RawEncoded` CPI pass-through | MEDIUM |
| **Quasar** | Batched u32 header validation (single compare for dup+signer+writable) | MEDIUM — applicable if jiminy adds account parsing |
| **Quasar** | SBF binary profiler with flamegraph | LOW — tooling, not core |
| **Star Frame** | Nested unsized types (UnsizedMap, UnsizedList) | LOW — complex, niche |
| **Star Frame** | Map/Set with binary search | MEDIUM — useful for ordered account data |
| **Star Frame** | Instruction return data (`ReturnType: NoUninit + set_return_data`) | LOW — niche |
| **Star Frame** | 4-phase instruction lifecycle (decode/validate/process/cleanup) | LOW — adds complexity |
| **Star Frame** | Context caching (rent, clock sysvars) | MEDIUM — easy CU win |
| **Star Frame** | Codama IDL integration | MEDIUM — ecosystem alignment |
| **Typhoon** | Size-adaptive discriminant fast paths | MEDIUM — applicable to layout_id checks |

### 5.3 Competitive Positioning

```
                High CU Efficiency ────────── Low CU Efficiency
                        │
    Raw Pinocchio ──── Quasar ──── Jiminy ──── Star Frame ──── Steel ──── Anchor
                        │                          │
                High Ergonomics              Most Dynamic Data
                Low Auditability             Features
```

**Jiminy's sweet spot**: The intersection of maximum safety (no proc macros, deterministic ABI, cross-program verification) with near-maximum performance. It's the framework for programs that need to be **auditable, composable, and fast** — not just fast.

**Quasar's sweet spot**: Maximum performance with Anchor-familiar ergonomics. Best choice for teams migrating from Anchor who want CU efficiency without learning a new paradigm.

**Star Frame's sweet spot**: Maximum type safety and feature completeness for complex programs. Best choice for large programs with dynamic data (gaming, metaverse, complex DeFi).

### 5.4 Threat Assessment

| Competitor | Threat Level | Why |
|---|---|---|
| **Quasar** | **HIGH** | Same performance tier as Jiminy, Anchor-familiar API reduces adoption friction, active development |
| **Star Frame** | **MEDIUM** | stronger dynamic data story, but `std` dependency and complexity limit audience |  
| **Anchor** | **LOW** (decreasing) | Established but losing mindshare due to CU costs, no zero-copy |
| **Steel** | **LOW** | Thin library, no dynamic data, limited development |
| **Typhoon** | **LOW** | Very minimal, no dynamic data |
| **Pina** | **LOW** | Codama-focused, not positioned as zero-copy standard |

### 5.5 Recommended Actions

**Immediate (v0.8)**:
1. Implement `BUMP_OFFSET`-style PDA optimization — ~344 CU per PDA is free money
2. Add `inline_dynamic_layout!` for 1-2 dynamic fields (Quasar's pattern, with layout_id integration)
3. Add `AccountShape::inspect()` for tooling

**Near-term (v0.9)**:
4. Context caching (rent, clock sysvars)
5. Multi-program interface support in `jiminy_interface!`
6. Generic `ZeroCopySlice<T, L>` prefix (u8/u16/u32)

**Strategic**:
7. Codama IDL output option for `jiminy-schema`
8. `segmented_interface!` for cross-program dynamic data
9. Binary search on `ZeroCopySlice` (sorted variant)

---

## Appendix: Repository Details

### Quasar
- **Repo**: https://github.com/blueshift-gg/quasar
- **Authors**: Leonardo Donatacci, Dean Little (Blueshift GG)
- **License**: MIT OR Apache-2.0
- **Solana SDK**: `solana-account-view 2.0`, `solana-address 2.2` (Pinocchio subcrates)
- **Rust edition**: 2021
- **CI**: GitHub Actions
- **Testing**: Integration tests in `tests/`, trybuild compile tests

### Star Frame
- **Repo**: https://github.com/staratlasmeta/star_frame
- **Author**: Star Atlas Meta
- **License**: Apache-2.0
- **Solana SDK**: `pinocchio 0.9.2`, `solana-pubkey 3.0`
- **Rust edition**: 2021, MSRV 1.84.1
- **CI**: GitHub Actions
- **Testing**: Mollusk SVM, trybuild, doc tests, unit tests
- **Release cadence**: ~monthly (v0.28 Jan 2026, v0.29 Feb 2026, v0.30 Feb 2026)
