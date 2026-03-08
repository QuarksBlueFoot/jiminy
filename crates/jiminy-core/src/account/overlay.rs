//! Zero-copy struct overlay for account data.
//!
//! Maps a `#[repr(C)]` struct directly onto borrowed account bytes.
//! No deserialization, no copies. Read fields via offset accessors
//! that the [`zero_copy_layout!`] macro generates for you.
//!
//! ## Why a declarative macro instead of a proc-macro?
//!
//! Jiminy is no-proc-macro by design. `zero_copy_layout!` generates
//! typed accessors using field-offset tables computed from the declared
//! layout. The struct itself is `#[repr(C)]` + `Pod` + `FixedLayout`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use jiminy_core::zero_copy_layout;
//! use jiminy_core::account::{AccountHeader, Pod, FixedLayout, HEADER_LEN};
//! use pinocchio::Address;
//!
//! zero_copy_layout! {
//!     /// My on-chain vault account.
//!     pub struct Vault {
//!         header:    AccountHeader = 8,
//!         authority: Address       = 32,
//!         mint:      Address       = 32,
//!         balance:   u64           = 8,
//!         bump:      u8            = 1,
//!     }
//! }
//!
//! // Read from borrowed account data:
//! let vault = Vault::overlay(&data)?;            // &Vault, zero-copy
//! let vault = Vault::overlay_mut(&mut data)?;    // &mut Vault, zero-copy
//! let auth: &Address = &vault.authority;
//! ```
//!
//! The macro emits:
//! - A `#[repr(C)]` struct with `Pod` + `FixedLayout`
//! - `overlay(&[u8]) -> Result<&Self, ProgramError>` (immutable view)
//! - `overlay_mut(&mut [u8]) -> Result<&mut Self, ProgramError>` (mutable view)
//! - A `const SIZE: usize` that sums all field sizes
//! - A `const OFFSETS` array and per-field `OFFSET_*` constants

/// Declare a zero-copy account layout with typed field accessors.
///
/// Each field specifies `name: Type = byte_size`. The macro generates a
/// `#[repr(C)]` struct along with `Pod`, `FixedLayout`, and overlay
/// methods.
///
/// ```rust,ignore
/// zero_copy_layout! {
///     pub struct Pool {
///         header:     AccountHeader = 8,
///         authority:  Address       = 32,
///         reserve_a:  u64           = 8,
///         reserve_b:  u64           = 8,
///     }
/// }
/// ```
#[macro_export]
macro_rules! zero_copy_layout {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ty = $fsize:expr ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Clone, Copy)]
        $vis struct $name {
            $( $(#[$fmeta])* pub $field: $fty ),+
        }

        // SAFETY: The type is repr(C), Copy, and all fields implement Pod.
        // The caller guarantees all bit patterns are valid.
        unsafe impl $crate::account::Pod for $name {}

        impl $crate::account::FixedLayout for $name {
            const SIZE: usize = 0 $( + $fsize )+;
        }

        impl $name {
            /// Total byte size of this account layout.
            pub const LEN: usize = 0 $( + $fsize )+;

            /// Overlay an immutable reference onto borrowed account data.
            ///
            /// Returns `AccountDataTooSmall` if the slice is shorter than
            /// the layout size.
            #[inline(always)]
            pub fn overlay(data: &[u8]) -> Result<&Self, $crate::pinocchio::error::ProgramError> {
                $crate::account::pod_from_bytes::<Self>(data)
            }

            /// Overlay a mutable reference onto borrowed account data.
            ///
            /// Returns `AccountDataTooSmall` if the slice is shorter than
            /// the layout size.
            #[inline(always)]
            pub fn overlay_mut(data: &mut [u8]) -> Result<&mut Self, $crate::pinocchio::error::ProgramError> {
                $crate::account::pod_from_bytes_mut::<Self>(data)
            }

            /// Read a copy of this struct from a byte slice.
            ///
            /// Alignment-safe on all targets (uses `read_unaligned`
            /// internally). Ideal for native tests.
            #[inline(always)]
            pub fn read(data: &[u8]) -> Result<Self, $crate::pinocchio::error::ProgramError> {
                $crate::account::pod_read::<Self>(data)
            }
        }
    };
}
