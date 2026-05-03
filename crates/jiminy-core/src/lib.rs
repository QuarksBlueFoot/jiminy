#![no_std]
//! # jiminy-core
//!
//! Account layout, validation, math, PDA, and all the zero-copy primitives
//! your Hopper program needs before it touches a token.
//!
//! This is the systems layer. Everything here works with raw `AccountView`
//! bytes and has zero dependencies beyond hopper-runtime itself. If your program
//! never calls SPL Token, this crate is all you need.
//!
//! ```rust,ignore
//! use jiminy_core::prelude::*;
//! ```
//!
//! # Modules
//!
//! | Module | |
//! |---|---|
//! | [`account`] | Header, reader, writer, cursor, lifecycle, pod, overlay, collection, list, bits |
//! | [`abi`] | Alignment-1 LE field types (`LeU64`, `LeBool`, …) and borrow-splitting refs |
//! | [`check`] | Validation checks, asserts, PDA derivation & verification |
//! | [`compat`] | Optional `solana-zero-copy` integration *(feature: `solana-zero-copy`)* |
//! | [`instruction`] | Transaction introspection (sysvar Instructions) |
//! | [`interface`] | Cross-program ABI interfaces (`jiminy_interface!`) |
//! | [`math`] | Checked arithmetic, BPS, scaling |
//! | [`field`] | Typed field descriptors for named zero-copy offsets |
//! | [`packed`] | Reserved bytes and extension-region helpers |
//! | [`sysvar`] | Clock & Rent sysvar readers |
//! | [`state`] | State machine transition checks |
//! | [`state_utils`] | State hygiene helpers for layout lifecycle work |
//! | [`time`] | Deadline, cooldown, staleness checks |
//! | [`event`] | Zero-alloc event emission via `sol_log_data` |
//! | [`programs`] | Well-known program IDs *(feature: `programs`)* |
//!
//! # Macros
//!
//! All macros are declarative (`macro_rules!`). No proc macros.
//!
//! | Macro | |
//! |---|---|
//! | [`require!`] | `if !cond { return Err(e) }` -- the universal guard |
//! | [`require_keys_eq!`] | Two addresses must match |
//! | [`check_accounts_unique!`] | Pairwise uniqueness for any N accounts |
//! | [`error_codes!`] | Define numbered error codes without a proc macro |
//! | [`instruction_dispatch!`] | Byte-tag instruction routing |
//! | [`jiminy_interface!`](crate::jiminy_interface) | Read-only interface for foreign program accounts |
//! | [`impl_pod!`] | Batch `unsafe impl Pod` |
//! | [`assert_legacy_layout!`] | Validate existing non-Jiminy account ABIs without adding a header |
//! | [`segmented_layout!`] | Fixed prefix + dynamic segments for variable-length accounts |
//!
//! # What does NOT belong here
//!
//! Token/mint readers, Token-2022 screening, CPI guards, Ed25519, Merkle,
//! oracles, AMM math, lending, staking, vesting -- see `jiminy-solana`,
//! `jiminy-finance`, and other domain crates.

// ── Domain modules ───────────────────────────────────────────────────────────

pub mod account;
pub mod check;
pub mod event;
pub mod field;
pub mod instruction;
pub mod math;
pub mod packed;
pub mod prelude;
pub mod state;
pub mod state_utils;
pub mod sysvar;
pub mod time;

#[cfg(feature = "log")]
pub mod log;

#[cfg(feature = "programs")]
pub mod programs;

pub mod abi;
pub mod compat;
pub mod interface;

pub use field::*;
pub use packed::*;
pub use state_utils::*;

// ── Hopper Runtime re-exports ─────────────────────────────────────────────────
//
// Downstream crates depend on just jiminy-core; they get hopper-runtime for free.

pub use hopper_runtime;
pub use hopper_runtime::{ProgramError, AccountView, Address, ProgramResult};

// ── Internal helpers (used by macros, not public API) ────────────────────────

/// Const SHA-256 helper for `zero_copy_layout!` layout ID generation.
#[doc(hidden)]
pub const fn __sha256_const(data: &[u8]) -> [u8; 32] {
    sha2_const_stable::Sha256::new().update(data).finalize()
}

// ── Macros ───────────────────────────────────────────────────────────────────

/// Require a boolean condition: return `$err` (converted via `Into`) if false.
#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr $(,)?) => {
        if !($cond) {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be equal.
///
/// Accepts owned `Address` values, `&Address` references, or a mix of both.
#[macro_export]
macro_rules! require_keys_eq {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        let __jiminy_a: &$crate::Address = &$a;
        let __jiminy_b: &$crate::Address = &$b;
        if __jiminy_a != __jiminy_b {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be **different**.
///
/// Accepts owned `Address` values, `&Address` references, or a mix of both.
#[macro_export]
macro_rules! require_keys_neq {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        let __jiminy_a: &$crate::Address = &$a;
        let __jiminy_b: &$crate::Address = &$b;
        if __jiminy_a == __jiminy_b {
            return Err($err.into());
        }
    };
}

/// Require two accounts to have **different** addresses.
#[macro_export]
macro_rules! require_accounts_ne {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a.address() == $b.address() {
            return Err($err.into());
        }
    };
}

/// Require `a >= b`.
#[macro_export]
macro_rules! require_gte {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a < $b {
            return Err($err.into());
        }
    };
}

/// Require `a > b`.
#[macro_export]
macro_rules! require_gt {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a <= $b {
            return Err($err.into());
        }
    };
}

/// Require `a < b`.
#[macro_export]
macro_rules! require_lt {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a >= $b {
            return Err($err.into());
        }
    };
}

/// Require `a <= b`.
#[macro_export]
macro_rules! require_lte {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a > $b {
            return Err($err.into());
        }
    };
}

/// Require `a == b` for scalar types.
#[macro_export]
macro_rules! require_eq {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a != $b {
            return Err($err.into());
        }
    };
}

/// Require `a != b` for scalar types.
#[macro_export]
macro_rules! require_neq {
    ($a:expr, $b:expr, $err:expr $(,)?) => {
        if $a == $b {
            return Err($err.into());
        }
    };
}

/// Require bit `n` to be set in `$byte`, else return `$err`.
#[macro_export]
macro_rules! require_flag {
    ($byte:expr, $n:expr, $err:expr $(,)?) => {
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
    // Base case: two accounts.
    ($a:expr, $b:expr) => {
        if $a.address() == $b.address() {
            return Err($crate::ProgramError::InvalidArgument);
        }
    };
    // Recursive: compare head against every tail, then recurse on tail.
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
    // Internal counter arm - assigns sequential codes.
    (@count $code:expr; $(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub struct $name;
        impl $name {
            pub const CODE: u32 = $code;
        }
    };
    (@count $code:expr; $(#[$meta:meta])* $name:ident, $( $(#[$rmeta:meta])* $rest:ident ),+ ) => {
        $(#[$meta])*
        pub struct $name;
        impl $name {
            pub const CODE: u32 = $code;
        }
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

/// Initialize a Jiminy account: CPI CreateAccount, zero-init, write header.
///
/// Owns the full creation path so developers cannot forget zero_init or
/// layout_id. Returns `Ok(())` after creating a fully-initialized account
/// with a valid 16-byte Jiminy header and zero-filled body.
///
/// Follow with `Layout::load_checked_mut()` to get a mutable overlay
/// for setting field values.
///
/// **Note:** Requires `hopper_runtime::system` in scope. This macro is re-exported
/// by the root `jiminy` crate which provides it automatically.
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
/// close_account!(account, destination);
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
    // Terminal: no more constraints
    ($account:expr, ) => { Ok::<(), $crate::ProgramError>(()) };
    ($account:expr $(,)?) => { Ok::<(), $crate::ProgramError>(()) };

    // owner = $id
    ($account:expr, owner = $id:expr $(, $($rest:tt)*)?) => {{
        $crate::check::check_owner($account, $id)?;
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    // writable
    ($account:expr, writable $(, $($rest:tt)*)?) => {{
        $crate::check::check_writable($account)?;
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    // signer
    ($account:expr, signer $(, $($rest:tt)*)?) => {{
        $crate::check::check_signer($account)?;
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    // disc = $d
    ($account:expr, disc = $d:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::check::check_discriminator(&data, $d)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    // version >= $v
    ($account:expr, version >= $v:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::check::check_version(&data, $v)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    // layout_id = $id
    ($account:expr, layout_id = $id:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::account::check_layout_id(&data, $id)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};

    // size >= $n
    ($account:expr, size >= $n:expr $(, $($rest:tt)*)?) => {{
        {
            let data = $account.try_borrow()?;
            $crate::check::check_size(&data, $n)?;
        }
        $crate::__check_account_inner!($account, $($($rest)*)?)
    }};
}

/// Strict account validation. Like [`check_account!`] but requires
/// `owner`, `disc`, and `layout_id` as the first three arguments.
/// Forgetting any of them is a compile error.
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
