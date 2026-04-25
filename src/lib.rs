#![no_std]
//! # jiminy
//!
//! Zero-copy account layouts, safety checks, and ABI tooling for Solana
//! programs built on Hopper Runtime.
//!
//! Sits between raw runtime primitives and higher-level frameworks like Anchor.
//! You get deterministic account layouts, verified zero-copy access,
//! explicit safety tiers for account loading, and reusable validation
//! functions for on-chain correctness. No framework, no proc macros,
//! no hidden control flow.
//!
//! `no_std`. `no_alloc`. Declarative macros only. Hot-path helpers are
//! `#[inline(always)]`; larger walkers use plain `#[inline]` so the BPF
//! linker can decide. Built on Hopper Runtime, Pinocchio backend wired in
//! today.
//!
//! ## Permanent non-goals
//!
//! - Jiminy will never be a framework that owns your control flow.
//! - Jiminy will never require `alloc`/`std` for core ABI + validation.
//! - Jiminy will never make unsafe account loading the default.
//! - Jiminy will never make domain crates a dependency of core.
//!
//! # Start Here
//!
//! | I want to… | Start with |
//! |---|---|
//! | Start a new program | [`examples/jiminy-vault`](https://github.com/QuarksBlueFoot/jiminy/tree/main/examples/jiminy-vault) → `use jiminy::prelude::*` |
//! | Harden a Pinocchio codebase | [`MIGRATION_COOKBOOK`](https://github.com/QuarksBlueFoot/jiminy/blob/main/docs/MIGRATION_COOKBOOK.md) |
//! | Read external / Anchor accounts | [`load_foreign`](crate::account) / [`jiminy_interface!`](crate::interface) |
//! | Build off-chain tooling | [`jiminy-schema`](https://docs.rs/jiminy-schema) → [`@jiminy/ts`](https://www.npmjs.com/package/@jiminy/ts) |
//!
//! # Modules
//!
//! ## Ring 1 -- `jiminy_core`
//!
//! | Module | |
//! |---|---|
//! | [`account`] | Header, reader, writer, cursor, lifecycle, pod, overlay, collection, list, bits |
//! | [`abi`] | Alignment-safe LE wire types (`LeU64`, `FieldRef`, `FieldMut`) |
//! | [`check`] | Validation checks, asserts, PDA derivation & verification |
//! | [`math`] | Checked arithmetic, BPS, scaling |
//! | [`instruction`] | Transaction introspection (sysvar Instructions) |
//! | [`interface`] | Read-only foreign account interface macro |
//! | [`state`] | State machine transition checks |
//! | [`sysvar`] | Clock & Rent sysvar readers |
//! | [`time`] | Deadline, cooldown, staleness checks |
//! | [`event`] | Zero-alloc event emission via `sol_log_data` |
//! | [`programs`] | Well-known program IDs *(feature: `programs`)* |
//!
//! ## Ring 2 -- `jiminy_solana`
//!
//! | Module | |
//! |---|---|
//! | [`token`] | SPL Token account readers, mint readers, Token-2022 extension screening |
//! | [`cpi`] | Safe CPI wrappers, reentrancy guards, return data readers |
//! | [`crypto`] | Ed25519 precompile verification, Merkle proof verification |
//! | [`authority`] | Two-step authority rotation (propose + accept) |
//! | [`balance`] | Pre/post CPI balance delta guards |
//! | [`compute`] | Compute budget guards |
//! | [`compose`] | Transaction composition guards (flash-loan detection) |
//! | [`introspect`] | Raw transaction introspection |
//! | [`oracle`] | Pyth V2 price feed readers |
//! | [`twap`] | TWAP accumulators |
//! | [`upgrade`] | Program upgrade authority verification *(feature: `programs`)* |
//!
//! ## Community / Domain Extensions (Not Core)
//!
//! These crates demonstrate patterns built using Jiminy.
//! They are not part of the core, and Jiminy does not depend on them.
//!
//! | Crate | |
//! |---|---|
//! | [`jiminy_finance`] | AMM math, slippage / economic bounds |
//! | [`jiminy_lending`] | Lending protocol primitives |
//! | [`jiminy_staking`] | Staking reward accumulators |
//! | [`jiminy_vesting`] | Vesting schedule helpers |
//! | [`jiminy_multisig`] | M-of-N multi-signer threshold |
//! | [`jiminy_distribute`] | Dust-safe proportional distribution |
//!
//! # Macros
//!
//! Macros in Jiminy exist to reduce repetitive safety code, not to introduce
//! abstraction layers. All macros are declarative (`macro_rules!`). No proc
//! macros, no build dependency, no compile-time surprises.
//!
//! ### Account ABI
//!
//! | Macro | |
//! |---|---|
//! | `zero_copy_layout!` | Define `#[repr(C)]` account struct with `Pod`, overlay, tiered loaders, `LAYOUT_ID` |
//! | `segmented_layout!` | Extend `zero_copy_layout!` with dynamic variable-length segments |
//! | `jiminy_interface!` | Declare read-only view of a foreign program's account (cross-program ABI) |
//! | [`init_account!`] | CPI create + zero-init + header write in one call |
//! | [`close_account!`] | Safe close with lamport drain and sentinel byte |
//! | [`check_account!`] | Disc + version + layout_id validation |
//! | [`check_account_strict!`] | Mandatory owner + disc + layout_id (compile error if missing) |
//! | [`impl_pod!`] | Batch `unsafe impl Pod` for `#[repr(C)]` types |
//!
//! ### Safety guards
//!
//! | Macro | |
//! |---|---|
//! | [`require!`] | `if !cond { return Err(e) }` -- the universal guard |
//! | [`require_keys_eq!`] | Two addresses must match |
//! | [`require_keys_neq!`] | Two addresses must differ |
//! | [`require_gte!`] | `a >= b` |
//! | [`require_gt!`] | `a > b` |
//! | [`require_lt!`] | `a < b` |
//! | [`require_lte!`] | `a <= b` |
//! | [`require_eq!`] | Scalar equality |
//! | [`require_neq!`] | Scalar inequality |
//! | [`require_flag!`] | Bit must be set |
//! | [`check_accounts_unique!`] | Pairwise uniqueness for any N accounts |
//!
//! ### Program structure
//!
//! | Macro | |
//! |---|---|
//! | [`error_codes!`] | Sequential error constants + `Into<ProgramError>` |
//! | [`instruction_dispatch!`] | Byte-tag dispatch to handler functions |
//!
//! ### PDA
//!
//! | Macro | |
//! |---|---|
//! | `find_pda!` | Find canonical PDA + bump via syscall |
//! | `derive_pda!` | Derive PDA with known bump (~100 CU) |
//! | `derive_pda_const!` | Compile-time PDA derivation |
//! | `derive_ata_const!` | Compile-time ATA derivation |
//! | `require_pda!` | Derive + assert PDA match, return bump |
//!
//! ### Events
//!
//! | Macro | |
//! |---|---|
//! | `emit!` | Zero-alloc event emission via `sol_log_data` |

// ── Ring 1: systems layer (from jiminy-core) ─────────────────────────────────

pub use jiminy_core::{abi, account, check, compat, event, instruction, interface, math, state, sysvar, time};

#[cfg(feature = "programs")]
pub use jiminy_core::programs;

#[cfg(feature = "log")]
pub use jiminy_core::log;

// ── Ring 2: platform helpers (from jiminy-solana) ────────────────────────────

pub use jiminy_solana::{
    authority, balance, compute, compose, cpi, crypto, introspect, oracle, token, twap,
};

#[cfg(feature = "programs")]
pub use jiminy_solana::upgrade;

// ── Ring 3+: protocol domain crates ──────────────────────────────────────────

pub mod amm {
    //! AMM math: integer square root, constant-product swap formulas, LP minting.
    pub use jiminy_finance::amm::*;
}

pub mod slippage {
    //! Slippage and economic bound checks.
    pub use jiminy_finance::slippage::*;
}

pub mod lending {
    //! Lending protocol primitives: collateralization, liquidation, interest.
    pub use jiminy_lending::*;
}

pub mod staking {
    //! Staking reward accumulators (MasterChef-style).
    pub use jiminy_staking::*;
}

pub mod vesting {
    //! Vesting schedule helpers: linear, cliff, stepped, periodic.
    pub use jiminy_vesting::*;
}

pub mod multisig {
    //! M-of-N multi-signer threshold checks.
    pub use jiminy_multisig::*;
}

pub mod distribute {
    //! Dust-safe proportional distribution and fee extraction.
    pub use jiminy_distribute::*;
}

// ── Subcrate re-exports (for direct crate access) ───────────────────────────

pub use jiminy_core;
pub use jiminy_solana;
pub use jiminy_finance;
pub use jiminy_lending;
pub use jiminy_staking;
pub use jiminy_vesting;
pub use jiminy_multisig;
pub use jiminy_distribute;

// ── Hopper Runtime ───────────────────────────────────────────────────────────

pub use hopper_runtime;
pub use hopper_runtime::{ProgramError, AccountView, Address, ProgramResult};

// ── Prelude ──────────────────────────────────────────────────────────────────

pub mod prelude;

// ── Macros ───────────────────────────────────────────────────────────────────
//
// `#[macro_export]` places macros in the *defining* crate's root namespace.
// Rust does not support re-exporting `macro_rules!` macros across crates,
// so we must define each macro in both `jiminy` and `jiminy-core`. Users
// who depend on either crate get the macros; `$crate::` paths resolve
// correctly in each context. Keep both copies in sync.

/// Require a boolean condition: return `$err` (converted via `Into`) if false.
#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be equal.
#[macro_export]
macro_rules! require_keys_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a != *$b {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be **different**.
#[macro_export]
macro_rules! require_keys_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a == *$b {
            return Err($err.into());
        }
    };
}

/// Require two accounts to have **different** addresses.
#[macro_export]
macro_rules! require_accounts_ne {
    ($a:expr, $b:expr, $err:expr) => {
        if $a.address() == $b.address() {
            return Err($err.into());
        }
    };
}

/// Require `a >= b`.
#[macro_export]
macro_rules! require_gte {
    ($a:expr, $b:expr, $err:expr) => {
        if $a < $b {
            return Err($err.into());
        }
    };
}

/// Require `a > b`.
#[macro_export]
macro_rules! require_gt {
    ($a:expr, $b:expr, $err:expr) => {
        if $a <= $b {
            return Err($err.into());
        }
    };
}

/// Require `a < b`.
#[macro_export]
macro_rules! require_lt {
    ($a:expr, $b:expr, $err:expr) => {
        if $a >= $b {
            return Err($err.into());
        }
    };
}

/// Require `a <= b`.
#[macro_export]
macro_rules! require_lte {
    ($a:expr, $b:expr, $err:expr) => {
        if $a > $b {
            return Err($err.into());
        }
    };
}

/// Require `a == b` for scalar types.
#[macro_export]
macro_rules! require_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a != $b {
            return Err($err.into());
        }
    };
}

/// Require `a != b` for scalar types.
#[macro_export]
macro_rules! require_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a == $b {
            return Err($err.into());
        }
    };
}

/// Require bit `n` to be set in `$byte`, else return `$err`.
#[macro_export]
macro_rules! require_flag {
    ($byte:expr, $n:expr, $err:expr) => {
        if ($byte >> $n) & 1 == 0 {
            return Err($err.into());
        }
    };
}

/// Verify that all passed accounts have unique addresses.
///
/// Variadic - works with 2, 3, 4, or more accounts. Expands to
/// pairwise `!=` checks at compile time. No heap, no loops.
///
/// Replaces `check_accounts_unique_2`, `check_accounts_unique_3`, etc.
///
/// ```rust,ignore
/// check_accounts_unique!(payer, vault, mint);
/// check_accounts_unique!(a, b, c, d);
/// ```
#[macro_export]
macro_rules! check_accounts_unique {
    ($a:expr, $b:expr) => {
        if $a.address() == $b.address() {
            return Err($crate::ProgramError::InvalidArgument);
        }
    };
    ($head:expr, $($tail:expr),+ $(,)?) => {
        $( if $head.address() == $tail.address() {
            return Err($crate::ProgramError::InvalidArgument);
        } )+
        $crate::check_accounts_unique!($($tail),+);
    };
}

/// Define numbered program error codes that map to `ProgramError::Custom`.
///
/// Replaces Anchor's `#[error_code]` proc macro. Each variant gets a
/// `u32` discriminant offset from a base you provide. The macro emits
/// constants and an `Into<ProgramError>` conversion.
///
/// ```rust,ignore
/// error_codes! {
///     base = 6000;
///     Undercollateralized,   // 6000
///     Expired,               // 6001
///     InvalidOracle,         // 6002
/// }
///
/// // Use in require! or return Err(...)
/// require!(collateral >= min, Undercollateralized);
/// ```
#[macro_export]
macro_rules! error_codes {
    (
        base = $base:expr;
        $( $(#[$meta:meta])* $name:ident ),+ $(,)?
    ) => {
        /// Program-specific error codes.
        #[allow(non_upper_case_globals)]
        pub mod errors {
            $crate::error_codes!(@count $base; $( $(#[$meta])* $name ),+ );
        }
        $(
            impl From<errors::$name> for $crate::ProgramError {
                #[inline(always)]
                fn from(_: errors::$name) -> Self {
                    $crate::ProgramError::Custom(errors::$name::CODE)
                }
            }
        )+
    };
    (@count $code:expr; $(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub struct $name;
        impl $name { pub const CODE: u32 = $code; }
    };
    (@count $code:expr; $(#[$meta:meta])* $name:ident, $( $(#[$rmeta:meta])* $rest:ident ),+ ) => {
        $(#[$meta])*
        pub struct $name;
        impl $name { pub const CODE: u32 = $code; }
        $crate::error_codes!(@count $code + 1; $( $(#[$rmeta])* $rest ),+ );
    };
}

/// Route instruction data to handler functions based on a single-byte tag.
///
/// Replaces Anchor's `#[program]` proc macro. Reads byte 0 as the
/// discriminator and dispatches to the matching handler. Returns
/// `InvalidInstructionData` for unknown tags.
///
/// ```rust,ignore
/// instruction_dispatch! {
///     program_id, accounts, instruction_data;
///     0 => process_init(program_id, accounts, ix),
///     1 => process_deposit(program_id, accounts, ix),
///     2 => process_withdraw(program_id, accounts, ix),
/// }
/// ```
///
/// Inside each arm, `ix` is a `SliceCursor` positioned after the tag byte.
#[macro_export]
macro_rules! instruction_dispatch {
    (
        $pid:expr, $accs:expr, $data:expr;
        $( $tag:expr => $handler:expr ),+ $(,)?
    ) => {{
        let mut ix = $crate::account::SliceCursor::new($data);
        let tag = ix.read_u8()?;
        match tag {
            $( $tag => { let _ = &ix; $handler } )+
            _ => Err($crate::ProgramError::InvalidInstructionData),
        }
    }};
}

/// Batch `unsafe impl Pod` for a list of types.
///
/// ```rust,ignore
/// impl_pod!(u8, u16, u32, u64, MyStruct);
/// ```
#[macro_export]
macro_rules! impl_pod {
    ($($t:ty),+ $(,)?) => {
        $( unsafe impl $crate::account::Pod for $t {} )+
    };
}

/// Initialize a Jiminy account: CPI CreateAccount, zero-init, write header.
///
/// Owns the full creation path so developers cannot forget zero_init or
/// layout_id. Returns `Ok(())` after creating a fully-initialized account
/// with a valid 16-byte Jiminy header and zero-filled body.
///
/// Follow with `Layout::load_checked_mut()` to get a mutable overlay
/// for setting field values.
///
/// ```rust,ignore
/// init_account!(payer, account, program_id, Vault)?;
///
/// // Now set fields via overlay:
/// let mut data = account.try_borrow_mut()?;
/// let vault = Vault::overlay_mut(&mut data)?;
/// vault.balance = 0;
/// vault.authority = *authority;
/// ```
///
/// Expands to:
/// 1. `rent_exempt_min(Layout::LEN)` for lamports
/// 2. CPI `CreateAccount` with correct space and owner
/// 3. `zero_init` the full data slice
/// 4. `write_header` with disc + version + layout_id
#[macro_export]
macro_rules! init_account {
    ($payer:expr, $account:expr, $program_id:expr, $Layout:ty) => {{
        let space = <$Layout>::LEN as u64;
        let lamports = $crate::check::rent_exempt_min(<$Layout>::LEN);
        $crate::hopper_runtime::system::instructions::CreateAccount {
            from: $payer,
            to: $account,
            lamports,
            space,
            owner: $program_id,
        }
        .invoke()?;

        let mut data = $account.try_borrow_mut()?;
        $crate::account::zero_init(&mut data);
        $crate::account::write_header(
            &mut data,
            <$Layout>::DISC,
            <$Layout>::VERSION,
            &<$Layout>::LAYOUT_ID,
        )?;
        Ok::<(), $crate::ProgramError>(())
    }};
}

/// Close a Jiminy account: transfer lamports and write close sentinel.
///
/// Wraps `safe_close_with_sentinel` for a consistent one-liner.
///
/// ```rust,ignore
/// close_account!(account, destination)?;
/// ```
#[macro_export]
macro_rules! close_account {
    ($account:expr, $destination:expr) => {
        $crate::account::safe_close_with_sentinel($account, $destination)
    };
}

/// Composable account constraint macro.
///
/// Validates a combination of account properties in a single call.
/// Each keyword maps to a check function:
///
/// | Keyword | Check |
/// |---------|-------|
/// | `owner = $id` | `account.owner() == $id` |
/// | `writable` | `account.is_writable()` |
/// | `signer` | `account.is_signer()` |
/// | `disc = $d` | `data[0] == $d` |
/// | `version >= $v` | `data[1] >= $v` |
/// | `layout_id = $id` | `data[4..12] == $id` |
/// | `size >= $n` | `data.len() >= $n` |
///
/// ```rust,ignore
/// check_account!(vault, owner = program_id, writable, disc = Vault::DISC,
///                layout_id = &Vault::LAYOUT_ID, size >= Vault::LEN);
/// ```
#[macro_export]
macro_rules! check_account {
    ($account:expr, $($constraint:tt)*) => {{
        $crate::__check_account_inner!($account, $($constraint)*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __check_account_inner {
    ($account:expr, ) => { Ok::<(), $crate::ProgramError>(()) };
    ($account:expr $(,)?) => { Ok::<(), $crate::ProgramError>(()) };

    ($account:expr, owner = $id:expr $(, $($rest:tt)*)?) => {{
        $crate::check::check_owner($account, $id)?;
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    ($account:expr, writable $(, $($rest:tt)*)?) => {{
        $crate::check::check_writable($account)?;
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    ($account:expr, signer $(, $($rest:tt)*)?) => {{
        $crate::check::check_signer($account)?;
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    ($account:expr, disc = $d:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::check::check_discriminator(&data, $d)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    ($account:expr, version >= $v:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::check::check_version(&data, $v)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    ($account:expr, layout_id = $id:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::account::check_layout_id(&data, $id)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    ($account:expr, size >= $n:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::check::check_size(&data, $n)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};
}

/// Strict account validation macro — `owner`, `disc`, and `layout_id` are
/// mandatory positional arguments. Forgetting any of them is a compile error.
///
/// Additional optional constraints (`writable`, `signer`, `version >=`,
/// `size >=`) can follow.
///
/// ```rust,ignore
/// check_account_strict!(vault, owner = program_id, disc = Vault::DISC,
///     layout_id = &Vault::LAYOUT_ID, writable, size >= Vault::LEN);
/// ```
#[macro_export]
macro_rules! check_account_strict {
    ($account:expr, owner = $owner:expr, disc = $disc:expr, layout_id = $lid:expr
        $(, $($rest:tt)*)?) => {{
        $crate::check::check_owner($account, $owner)?;
        {
            let data = $account.try_borrow()?;
            $crate::check::check_discriminator(&data, $disc)?;
            $crate::account::check_layout_id(&data, $lid)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};
}

/// Find a PDA and return `(Address, u8)` with the canonical bump.
///
/// Uses the `find_program_address` syscall. Only available on-chain.
#[macro_export]
macro_rules! find_pda {
    ($program_id:expr, $($seed:expr),+ $(,)?) => {{
        #[cfg(target_os = "solana")]
        {
            let seeds: &[&[u8]] = &[$($seed.as_ref()),+];
            ::hopper_runtime::Address::find_program_address(seeds, $program_id)
        }
        #[cfg(not(target_os = "solana"))]
        {
            let _ = ($program_id, $($seed),+);
            unreachable!("find_pda! is only available on target solana")
        }
    }};
}

/// Derive a PDA with a known bump. Cheap (~100 CU, no curve check).
///
/// Wraps [`derive_address`](check::pda::derive_address). The bump is
/// appended automatically. Returns `Address`.
#[macro_export]
macro_rules! derive_pda {
    ($program_id:expr, $bump:expr, $($seed:expr),+ $(,)?) => {{
        ::hopper_runtime::Address::new_from_array($crate::check::pda::derive_address(
            &[$($seed.as_ref()),+],
            Some($bump),
            ($program_id).as_array(),
        ))
    }};
}

/// Derive a PDA at compile time. Requires `const` seeds and bump.
#[macro_export]
macro_rules! derive_pda_const {
    ($program_id:expr, $bump:expr, $($seed:expr),+ $(,)?) => {
        ::hopper_runtime::Address::new_from_array($crate::check::pda::derive_address_const(
            &[$(&$seed),+],
            Some($bump),
            &$program_id,
        ))
    };
}

/// Derive a PDA from seeds, verify the account matches, and return the bump.
///
/// Wraps [`assert_pda`](check::assert_pda) as a macro so you can pass
/// seeds inline without manual slice construction.
///
/// ```rust,ignore
/// let bump = require_pda!(vault_account, program_id, b"vault", user.address())?;
/// ```
#[macro_export]
macro_rules! require_pda {
    ($account:expr, $program_id:expr, $($seed:expr),+ $(,)?) => {{
        let seeds: &[&[u8]] = &[$($seed.as_ref()),+];
        $crate::check::assert_pda($account, seeds, $program_id)
    }};
}
