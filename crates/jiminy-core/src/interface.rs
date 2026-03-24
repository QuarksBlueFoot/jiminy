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
//!     let data = Vault::load_foreign(&accounts[0])?;
//!     let vault = Vault::overlay(&data)?;
//!     // read vault.balance, vault.authority, etc.
//!     Ok(())
//! }
//! ```
//!
//! ## What gets generated
//!
//! - `#[repr(C)]` struct with typed fields
//! - `LAYOUT_ID` matching the original `zero_copy_layout!` definition
//! - `LEN`, `DISC` (0), `VERSION` (0) constants
//! - `overlay` / `read` (immutable only — no mutable access)
//! - `load_foreign` — Tier 2 loading with owner + layout_id validation
//! - Const field offsets and `split_fields` (immutable only)
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
#[macro_export]
macro_rules! jiminy_interface {
    (
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

        impl $name {
            /// Total byte size of this account layout.
            pub const LEN: usize = 0 $( + $fsize )+;

            /// Deterministic ABI fingerprint (first 8 bytes of SHA-256).
            pub const LAYOUT_ID: [u8; 8] = {
                // NOTE: we use version = 1 in the hash input because
                // the original layout uses version = 1. Interface types
                // don't store their own disc/version — they match the
                // foreign program's hash.
                const INPUT: &str = concat!(
                    "jiminy:v1:",
                    stringify!($name), ":",
                    // Interface assumes version 1 for hash computation.
                    // If the foreign program uses a different version,
                    // the LAYOUT_ID won't match and load_foreign will
                    // reject the account — which is correct behavior.
                    "1:",
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

            /// **Tier 2 — Cross-program read.** Validate owner + layout_id,
            /// then borrow.
            ///
            /// The owner is checked against the program address passed to
            /// `jiminy_interface!` via the `for` clause.
            #[inline(always)]
            pub fn load_foreign<'a>(
                account: &'a $crate::pinocchio::AccountView,
            ) -> Result<$crate::pinocchio::account::Ref<'a, [u8]>, $crate::pinocchio::error::ProgramError> {
                $crate::account::view::validate_foreign(
                    account, &$owner, &Self::LAYOUT_ID, Self::LEN,
                )
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
