//! Cross-program ABI interface for read-only foreign account access.
//!
//! The `jiminy_interface!` macro generates a lightweight,
//! read-only struct that can decode accounts owned by another program.
//! It produces the same `LAYOUT_ID` as a matching `zero_copy_layout!`
//! declaration, enabling cross-program layout verification without
//! sharing crate dependencies.
//!
//! ## Usage
//!
//! Program B wants to read Program A's `Vault` account:
//!
//! ```rust,ignore
//! use jiminy_core::jiminy_interface;
//! use jiminy_core::account::{AccountHeader, Pod, FixedLayout};
//! use jiminy_core::abi::LeU64;
//! use pinocchio::Address;
//!
//! const PROGRAM_A: Address = [0u8; 32]; // Program A's address
//!
//! jiminy_interface! {
//!     /// Read-only view of Program A's Vault account.
//!     pub struct Vault for PROGRAM_A {
//!         header:    AccountHeader = 16,
//!         balance:   LeU64         = 8,
//!         authority: Address       = 32,
//!     }
//! }
//!
//! // In your instruction handler:
//! fn process(accounts: &[AccountView]) -> ProgramResult {
//!     let verified = Vault::load_foreign(&accounts[0])?;
//!     let vault = verified.get();
//!     // read vault.balance, vault.authority, etc.
//!     Ok(())
//! }
//! ```
//!
//! ## What gets generated
//!
//! - `#[repr(C)]` struct with typed fields
//! - `LAYOUT_ID` matching the original `zero_copy_layout!` definition
//! - `LEN` constant
//! - `overlay` / `read` (immutable only, no mutable access)
//! - `load_foreign` with Tier 2 owner + layout_id validation
//! - Const field offsets and `split_fields` (immutable only)
//!
//! ## Version
//!
//! By default, `version = 1` is used in the `LAYOUT_ID` hash. If the
//! foreign program uses a different version, specify it explicitly:
//!
//! ```rust,ignore
//! jiminy_interface! {
//!     pub struct PoolV2 for PROGRAM_A, version = 2 { ... }
//! }
//! ```
//!
//! ## Design
//!
//! Interface types are intentionally restricted:
//! - **No `load` (Tier 1)** — you don't own this account
//! - **No `overlay_mut`** — foreign accounts are read-only
//! - **No `split_fields_mut`** — same reason
//! - **No `load_checked`** — discriminator/version are owner concerns

/// Declare a read-only interface for a foreign program's account layout.
///
/// Generates a `#[repr(C)]` struct with the same `LAYOUT_ID` as the
/// foreign program's `zero_copy_layout!` definition, plus a
/// `load_foreign` method that validates owner + layout_id.
///
/// The struct name must match the original account name for the
/// `LAYOUT_ID` hash to agree. If you want a local alias, use
/// `type VaultView = Vault;` after the macro invocation.
///
/// ## Version
///
/// By default, the interface assumes the foreign program uses
/// `version = 1`. If the foreign layout uses a different version,
/// specify it explicitly so the `LAYOUT_ID` hash matches:
///
/// ```rust,ignore
/// jiminy_interface! {
///     pub struct PoolV2 for PROGRAM_A, version = 2 {
///         header:    AccountHeader = 16,
///         authority: Address       = 32,
///         reserve:   LeU64         = 8,
///         fee_bps:   LeU16         = 2,
///     }
/// }
/// ```
#[macro_export]
macro_rules! jiminy_interface {
    // ── Public arm: no version (defaults to 1) ───────────────────
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident for $owner:path {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ident = $fsize:expr ),+ $(,)?
        }
    ) => {
        $crate::jiminy_interface! {
            @impl version = 1,
            $(#[$meta])*
            $vis struct $name for $owner {
                $( $(#[$fmeta])* $field : $fty = $fsize ),+
            }
        }
    };

    // ── Public arm: explicit version ─────────────────────────────
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident for $owner:path, version = $ver:literal {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ident = $fsize:expr ),+ $(,)?
        }
    ) => {
        $crate::jiminy_interface! {
            @impl version = $ver,
            $(#[$meta])*
            $vis struct $name for $owner {
                $( $(#[$fmeta])* $field : $fty = $fsize ),+
            }
        }
    };

    // ── Internal implementation arm ──────────────────────────────
    (
        @impl version = $ver:literal,
        $(#[$meta:meta])*
        $vis:vis struct $name:ident for $owner:path {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ident = $fsize:expr ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Clone, Copy)]
        $vis struct $name {
            $( $(#[$fmeta])* pub $field: $fty ),+
        }

        // SAFETY: repr(C) + Copy + all fields are Pod.
        unsafe impl $crate::account::Pod for $name {}

        impl $crate::account::FixedLayout for $name {
            const SIZE: usize = 0 $( + $fsize )+;
        }

        // Compile-time assertion: actual size must match declared sum.
        const _: () = assert!(
            core::mem::size_of::<$name>() == 0 $( + $fsize )+,
            "size_of does not match declared LEN — check field sizes"
        );

        // Compile-time assertion: alignment must not exceed 8 bytes.
        const _: () = assert!(
            core::mem::align_of::<$name>() <= 8,
            "layout alignment exceeds 8 bytes — use Le* wrappers for u128 fields"
        );

        impl $name {
            /// Total byte size of this account layout.
            pub const LEN: usize = 0 $( + $fsize )+;

            /// Deterministic ABI fingerprint (first 8 bytes of SHA-256).
            ///
            /// The version used in the hash input matches the foreign
            /// program's `zero_copy_layout!` version. When no version is
            /// specified in the macro invocation, version 1 is assumed.
            pub const LAYOUT_ID: [u8; 8] = {
                const INPUT: &str = concat!(
                    "jiminy:v1:",
                    stringify!($name), ":",
                    stringify!($ver), ":",
                    $( stringify!($field), ":", $crate::__canonical_type!($fty), ":", stringify!($fsize), ",", )+
                );
                const HASH: [u8; 32] = $crate::__sha256_const(INPUT.as_bytes());
                [HASH[0], HASH[1], HASH[2], HASH[3], HASH[4], HASH[5], HASH[6], HASH[7]]
            };

            /// Expected owner program for this interface.
            pub const OWNER: &'static $crate::pinocchio::Address = &$owner;

            /// Overlay an immutable reference onto borrowed account data.
            #[inline(always)]
            pub fn overlay(data: &[u8]) -> Result<&Self, $crate::pinocchio::error::ProgramError> {
                $crate::account::pod_from_bytes::<Self>(data)
            }

            /// Read a copy of this struct from a byte slice (alignment-safe).
            #[inline(always)]
            pub fn read(data: &[u8]) -> Result<Self, $crate::pinocchio::error::ProgramError> {
                $crate::account::pod_read::<Self>(data)
            }

            /// **Tier 2 — Cross-program read.** Validate owner + layout_id
            /// + exact size, then borrow.
            ///
            /// The owner is checked against the program address passed to
            /// `jiminy_interface!` via the `for` clause.
            ///
            /// Returns a `VerifiedAccount` whose `get()` is infallible.
            #[inline(always)]
            pub fn load_foreign<'a>(
                account: &'a $crate::pinocchio::AccountView,
            ) -> Result<$crate::account::VerifiedAccount<'a, Self>, $crate::pinocchio::error::ProgramError> {
                let data = $crate::account::view::validate_foreign(
                    account, &$owner, &Self::LAYOUT_ID, Self::LEN,
                )?;
                $crate::account::VerifiedAccount::new(data)
            }

            // ── Const field offsets ──────────────────────────────────────

            $crate::__gen_offsets!( $( $field = $fsize ),+ );

            // ── Immutable borrow-splitting ───────────────────────────────

            /// Split borrowed data into per-field `FieldRef` slices.
            #[inline]
            #[allow(unused_variables)]
            pub fn split_fields(data: &[u8]) -> Result<( $( $crate::__field_ref_type!($field), )+ ), $crate::pinocchio::error::ProgramError> {
                if data.len() < Self::LEN {
                    return Err($crate::pinocchio::error::ProgramError::AccountDataTooSmall);
                }
                let mut _pos = 0usize;
                Ok(( $({
                    let start = _pos;
                    _pos += $fsize;
                    $crate::abi::FieldRef::new(&data[start..start + $fsize])
                }, )+ ))
            }
        }
    };
}
