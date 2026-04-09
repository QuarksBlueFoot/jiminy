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
//! use hopper_runtime::Address;
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
            pub const OWNER: &'static $crate::Address = &$owner;

            /// Overlay an immutable reference onto borrowed account data.
            #[inline(always)]
            pub fn overlay(data: &[u8]) -> Result<&Self, $crate::ProgramError> {
                $crate::account::pod_from_bytes::<Self>(data)
            }

            /// Read a copy of this struct from a byte slice (alignment-safe).
            #[inline(always)]
            pub fn read(data: &[u8]) -> Result<Self, $crate::ProgramError> {
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
                account: &'a $crate::AccountView,
            ) -> Result<$crate::account::VerifiedAccount<'a, Self>, $crate::ProgramError> {
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
            pub fn split_fields(data: &[u8]) -> Result<( $( $crate::__field_ref_type!($field), )+ ), $crate::ProgramError> {
                if data.len() < Self::LEN {
                    return Err($crate::ProgramError::AccountDataTooSmall);
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

// ═══════════════════════════════════════════════════════════════════════════════
// segmented_interface! — read-only cross-program view for segmented accounts
// ═══════════════════════════════════════════════════════════════════════════════

/// Declare a read-only interface for a foreign program's segmented account.
///
/// Extends [`jiminy_interface!`] with segment declarations, generating
/// the same `SEGMENTED_LAYOUT_ID` as the foreign program's
/// `segmented_layout!` definition. This enables cross-program reads of
/// variable-length accounts (order books, staking pools, etc.) without
/// crate dependencies.
///
/// ## Usage
///
/// Program B wants to read Program A's `OrderBook` segmented account:
///
/// ```rust,ignore
/// use jiminy_core::segmented_interface;
/// use jiminy_core::account::{AccountHeader, Pod, FixedLayout};
/// use jiminy_core::abi::LeU64;
/// use hopper_runtime::Address;
///
/// const DEX_PROGRAM: Address = [0u8; 32];
///
/// // Order element type (must match Program A's definition)
/// #[repr(C)]
/// #[derive(Clone, Copy)]
/// struct Order {
///     price: LeU64,
///     size:  LeU64,
/// }
/// unsafe impl Pod for Order {}
/// impl FixedLayout for Order { const SIZE: usize = 16; }
///
/// segmented_interface! {
///     pub struct OrderBook for DEX_PROGRAM {
///         header:  AccountHeader = 16,
///         market:  Address       = 32,
///     } segments {
///         bids: Order = 16,
///         asks: Order = 16,
///     }
/// }
///
/// // In your instruction handler:
/// fn process(accounts: &[AccountView]) -> ProgramResult {
///     let data = OrderBook::load_foreign_segmented(&accounts[0])?;
///     let table = OrderBook::segment_table(&data)?;
///     let bids = SegmentSlice::<Order>::from_descriptor(&data, &table.descriptor(0)?)?;
///     // read bids...
///     Ok(())
/// }
/// ```
///
/// ## What gets generated
///
/// Everything from `jiminy_interface!` plus:
///
/// - `SEGMENTED_LAYOUT_ID` matching the foreign `segmented_layout!`
/// - `SEGMENT_COUNT`, `TABLE_OFFSET`, `DATA_START_OFFSET`, `MIN_ACCOUNT_SIZE`
/// - `segment_table(data)` — read-only segment table access
/// - `segment::<T>(data, index)` — typed read-only segment slice
/// - `validate_segments(data)` — full segment validation
/// - `load_foreign_segmented(account)` — Tier 2 validation with min-size
///
/// No mutable access is generated (consistent with interface philosophy).
#[macro_export]
macro_rules! segmented_interface {
    // ── Public arm: no version (defaults to 1) ───────────────────
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident for $owner:path {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ident = $fsize:expr ),+ $(,)?
        } segments {
            $( $seg_name:ident : $seg_ty:ident = $seg_elem_size:expr ),+ $(,)?
        }
    ) => {
        $crate::segmented_interface! {
            @impl version = 1,
            $(#[$meta])*
            $vis struct $name for $owner {
                $( $(#[$fmeta])* $field : $fty = $fsize ),+
            } segments {
                $( $seg_name : $seg_ty = $seg_elem_size ),+
            }
        }
    };

    // ── Public arm: explicit version ─────────────────────────────
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident for $owner:path, version = $ver:literal {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ident = $fsize:expr ),+ $(,)?
        } segments {
            $( $seg_name:ident : $seg_ty:ident = $seg_elem_size:expr ),+ $(,)?
        }
    ) => {
        $crate::segmented_interface! {
            @impl version = $ver,
            $(#[$meta])*
            $vis struct $name for $owner {
                $( $(#[$fmeta])* $field : $fty = $fsize ),+
            } segments {
                $( $seg_name : $seg_ty = $seg_elem_size ),+
            }
        }
    };

    // ── Internal implementation arm ──────────────────────────────
    (
        @impl version = $ver:literal,
        $(#[$meta:meta])*
        $vis:vis struct $name:ident for $owner:path {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ident = $fsize:expr ),+ $(,)?
        } segments {
            $( $seg_name:ident : $seg_ty:ident = $seg_elem_size:expr ),+ $(,)?
        }
    ) => {
        // Generate the fixed prefix struct via jiminy_interface!
        $crate::jiminy_interface! {
            @impl version = $ver,
            $(#[$meta])*
            $vis struct $name for $owner {
                $( $(#[$fmeta])* $field : $fty = $fsize ),+
            }
        }

        impl $name {
            /// Number of dynamic segments in this layout.
            pub const SEGMENT_COUNT: usize = $crate::__count_segments!($($seg_name)+);

            /// Byte size of the fixed prefix (before the segment table).
            pub const FIXED_LEN: usize = Self::LEN;

            /// Byte offset where the segment table begins.
            pub const TABLE_OFFSET: usize = Self::LEN;

            /// Byte offset where segment data starts (after fixed + table).
            pub const DATA_START_OFFSET: usize =
                Self::LEN + Self::SEGMENT_COUNT * $crate::account::segment::SEGMENT_DESC_SIZE;

            /// Minimum account size: fixed prefix + segment table (no data).
            pub const MIN_ACCOUNT_SIZE: usize = Self::DATA_START_OFFSET;

            /// Deterministic ABI fingerprint including segment declarations.
            ///
            /// Produces the same hash as the foreign `segmented_layout!`.
            pub const SEGMENTED_LAYOUT_ID: [u8; 8] = {
                const INPUT: &str = concat!(
                    "jiminy:v1:",
                    stringify!($name), ":",
                    stringify!($ver), ":",
                    $( stringify!($field), ":", $crate::__canonical_type!($fty), ":", stringify!($fsize), ",", )+
                    $( "seg:", stringify!($seg_name), ":", stringify!($seg_ty), ":", stringify!($seg_elem_size), ",", )+
                );
                const HASH: [u8; 32] = $crate::__sha256_const(INPUT.as_bytes());
                [HASH[0], HASH[1], HASH[2], HASH[3], HASH[4], HASH[5], HASH[6], HASH[7]]
            };

            /// Expected element sizes for each segment, in declaration order.
            #[inline(always)]
            pub const fn segment_sizes() -> &'static [u16] {
                &[ $( $seg_elem_size as u16, )+ ]
            }

            /// **Tier 2 — Cross-program segmented read.**
            ///
            /// Validates owner + `SEGMENTED_LAYOUT_ID` + minimum size,
            /// then borrows account data. Returns the raw data reference
            /// for segment table and element access.
            ///
            /// Unlike `load_foreign` on fixed layouts (which returns a
            /// `VerifiedAccount<T>`), this returns the raw borrowed data
            /// because the account size is variable. Use
            /// [`segment_table`](Self::segment_table) and
            /// [`segment`](Self::segment) for typed access.
            #[inline(always)]
            pub fn load_foreign_segmented<'a>(
                account: &'a $crate::AccountView,
            ) -> Result<$crate::hopper_runtime::Ref<'a, [u8]>, $crate::ProgramError> {
                $crate::account::view::validate_foreign_segmented(
                    account,
                    &$owner,
                    &Self::SEGMENTED_LAYOUT_ID,
                    Self::MIN_ACCOUNT_SIZE,
                )
            }

            /// Read the segment table from account data (read-only).
            #[inline(always)]
            pub fn segment_table(data: &[u8]) -> Result<$crate::account::segment::SegmentTable<'_>, $crate::ProgramError> {
                if data.len() < Self::DATA_START_OFFSET {
                    return Err($crate::ProgramError::AccountDataTooSmall);
                }
                $crate::account::segment::SegmentTable::from_bytes(
                    &data[Self::TABLE_OFFSET..],
                    Self::SEGMENT_COUNT,
                )
            }

            /// Validate the segment table against the account data.
            ///
            /// Checks element sizes, bounds, ordering, and no overlaps.
            #[inline]
            pub fn validate_segments(data: &[u8]) -> Result<(), $crate::ProgramError> {
                let table = Self::segment_table(data)?;
                table.validate(data.len(), Self::segment_sizes(), Self::DATA_START_OFFSET)
            }

            /// Get an immutable typed view over a segment by index.
            #[inline(always)]
            pub fn segment<T: $crate::account::Pod + $crate::account::FixedLayout>(
                data: &[u8],
                index: usize,
            ) -> Result<$crate::account::segment::SegmentSlice<'_, T>, $crate::ProgramError> {
                let desc = {
                    let table = Self::segment_table(data)?;
                    table.descriptor(index)?
                };
                $crate::account::segment::SegmentSlice::from_descriptor(data, &desc)
            }

            // ── Segment index constants ──────────────────────────────

            $crate::__gen_segment_indices!($($seg_name),+);
        }
    };
}
