//! Jiminy ABI field primitives — alignment-1 wire types.
//!
//! These are the canonical field types for `zero_copy_layout!` structs.
//! Each type is `#[repr(transparent)]` over a byte array, guaranteeing
//! alignment 1 and eliminating UB from unaligned references on any target.
//!
//! ## Why
//!
//! SBF has 1-byte alignment for everything, so raw `u64` overlays work
//! on-chain. But native tests, Miri, and future VMs may enforce real
//! alignment — and taking `&u64` to an unaligned pointer is instant UB.
//!
//! These wrappers store fields as `[u8; N]` (LE) and access via
//! `from_le_bytes` / `to_le_bytes`. Zero overhead on SBF. Safe everywhere.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use jiminy_core::abi::{LeU64, LeBool};
//!
//! zero_copy_layout! {
//!     pub struct Vault, discriminator = 1, version = 1 {
//!         header:    AccountHeader = 16,
//!         balance:   LeU64         = 8,
//!         is_frozen: LeBool        = 1,
//!         authority: Address       = 32,
//!     }
//! }
//!
//! let vault = Vault::overlay(&data)?;
//! let bal: u64 = vault.balance.get();
//! ```

use crate::account::{FixedLayout, Pod};

// ── Macro to stamp out unsigned LE integer wrappers ──────────────────────────

macro_rules! impl_le_unsigned {
    ($name:ident, $inner:ty, $size:literal) => {
        #[doc = concat!("Alignment-1, little-endian `", stringify!($inner), "` for on-chain ABI fields.")]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct $name(pub [u8; $size]);

        const _: () = assert!(core::mem::size_of::<$name>() == $size);
        const _: () = assert!(core::mem::align_of::<$name>() == 1);

        impl $name {
            /// Zero value.
            pub const ZERO: Self = Self([0u8; $size]);

            /// Wrap a native value.
            #[inline(always)]
            pub const fn new(v: $inner) -> Self {
                Self(v.to_le_bytes())
            }

            /// Read the native value.
            #[inline(always)]
            pub const fn get(&self) -> $inner {
                <$inner>::from_le_bytes(self.0)
            }

            /// Write a native value.
            #[inline(always)]
            pub fn set(&mut self, v: $inner) {
                self.0 = v.to_le_bytes();
            }
        }

        impl Default for $name {
            #[inline(always)]
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.get())
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.get())
            }
        }

        impl PartialOrd for $name {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            #[inline(always)]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl From<$inner> for $name {
            #[inline(always)]
            fn from(v: $inner) -> Self {
                Self::new(v)
            }
        }

        impl From<$name> for $inner {
            #[inline(always)]
            fn from(v: $name) -> Self {
                v.get()
            }
        }

        // SAFETY: repr(transparent) over [u8; N], all bit patterns valid.
        unsafe impl Pod for $name {}
        impl FixedLayout for $name {
            const SIZE: usize = $size;
        }
    };
}

// ── Macro to stamp out signed LE integer wrappers ────────────────────────────

macro_rules! impl_le_signed {
    ($name:ident, $inner:ty, $size:literal) => {
        #[doc = concat!("Alignment-1, little-endian `", stringify!($inner), "` for on-chain ABI fields.")]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct $name(pub [u8; $size]);

        const _: () = assert!(core::mem::size_of::<$name>() == $size);
        const _: () = assert!(core::mem::align_of::<$name>() == 1);

        impl $name {
            /// Zero value.
            pub const ZERO: Self = Self([0u8; $size]);

            /// Wrap a native value.
            #[inline(always)]
            pub const fn new(v: $inner) -> Self {
                Self(v.to_le_bytes())
            }

            /// Read the native value.
            #[inline(always)]
            pub const fn get(&self) -> $inner {
                <$inner>::from_le_bytes(self.0)
            }

            /// Write a native value.
            #[inline(always)]
            pub fn set(&mut self, v: $inner) {
                self.0 = v.to_le_bytes();
            }
        }

        impl Default for $name {
            #[inline(always)]
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.get())
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.get())
            }
        }

        impl PartialOrd for $name {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            #[inline(always)]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl From<$inner> for $name {
            #[inline(always)]
            fn from(v: $inner) -> Self {
                Self::new(v)
            }
        }

        impl From<$name> for $inner {
            #[inline(always)]
            fn from(v: $name) -> Self {
                v.get()
            }
        }

        // SAFETY: repr(transparent) over [u8; N], all bit patterns valid.
        unsafe impl Pod for $name {}
        impl FixedLayout for $name {
            const SIZE: usize = $size;
        }
    };
}

// ── Unsigned types ───────────────────────────────────────────────────────────

impl_le_unsigned!(LeU16, u16, 2);
impl_le_unsigned!(LeU32, u32, 4);
impl_le_unsigned!(LeU64, u64, 8);
impl_le_unsigned!(LeU128, u128, 16);

// ── Signed types ─────────────────────────────────────────────────────────────

impl_le_signed!(LeI16, i16, 2);
impl_le_signed!(LeI32, i32, 4);
impl_le_signed!(LeI64, i64, 8);
impl_le_signed!(LeI128, i128, 16);

// ── Bool ─────────────────────────────────────────────────────────────────────

/// Alignment-1 boolean for on-chain ABI fields.
///
/// Stored as a single byte: `0` = false, any non-zero = true.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LeBool(pub [u8; 1]);

const _: () = assert!(core::mem::size_of::<LeBool>() == 1);
const _: () = assert!(core::mem::align_of::<LeBool>() == 1);

impl LeBool {
    /// False value.
    pub const FALSE: Self = Self([0]);
    /// True value.
    pub const TRUE: Self = Self([1]);

    /// Wrap a native bool.
    #[inline(always)]
    pub const fn new(v: bool) -> Self {
        Self([v as u8])
    }

    /// Read the native bool.
    #[inline(always)]
    pub const fn get(&self) -> bool {
        self.0[0] != 0
    }

    /// Write a native bool.
    #[inline(always)]
    pub fn set(&mut self, v: bool) {
        self.0[0] = v as u8;
    }
}

impl Default for LeBool {
    #[inline(always)]
    fn default() -> Self {
        Self::FALSE
    }
}

impl core::fmt::Debug for LeBool {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LeBool({})", self.get())
    }
}

impl core::fmt::Display for LeBool {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl From<bool> for LeBool {
    #[inline(always)]
    fn from(v: bool) -> Self {
        Self::new(v)
    }
}

impl From<LeBool> for bool {
    #[inline(always)]
    fn from(v: LeBool) -> Self {
        v.get()
    }
}

// SAFETY: repr(transparent) over [u8; 1], all bit patterns valid.
unsafe impl Pod for LeBool {}
impl FixedLayout for LeBool {
    const SIZE: usize = 1;
}

// ── Field reference wrappers for borrow-splitting ────────────────────────────

/// Immutable typed view over a field-sized byte slice.
///
/// Produced by `split_fields` (generated by `zero_copy_layout!`).
/// Provides typed access without holding a reference to the whole struct.
pub struct FieldRef<'a> {
    data: &'a [u8],
}

impl<'a> FieldRef<'a> {
    /// Create a field reference from a byte slice.
    #[inline(always)]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// The byte length of this field.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the field is zero-length.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Read the raw bytes.
    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
        self.data
    }

    /// Read as an `Address` (32-byte public key).
    ///
    /// Copies the bytes into an owned `Address`.
    #[inline(always)]
    pub fn read_address(&self) -> pinocchio::Address {
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&self.data[..32]);
        pinocchio::Address::from(addr)
    }

    /// Borrow the field bytes as an `&Address` reference.
    ///
    /// # Panics
    ///
    /// Panics if the field is smaller than 32 bytes.
    #[inline(always)]
    pub fn as_address(&self) -> &pinocchio::Address {
        // SAFETY: Address is repr(transparent) over [u8; 32], alignment 1.
        // The slice has been bounds-checked to 32 bytes.
        let ptr = self.data[..32].as_ptr() as *const pinocchio::Address;
        unsafe { &*ptr }
    }

    /// Read a `u64` from the field (LE).
    #[inline(always)]
    pub fn read_u64(&self) -> u64 {
        u64::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
        ])
    }

    /// Read a `u32` from the field (LE).
    #[inline(always)]
    pub fn read_u32(&self) -> u32 {
        u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    /// Read a `u16` from the field (LE).
    #[inline(always)]
    pub fn read_u16(&self) -> u16 {
        u16::from_le_bytes([self.data[0], self.data[1]])
    }

    /// Read a `u8` from the field.
    #[inline(always)]
    pub fn read_u8(&self) -> u8 {
        self.data[0]
    }

    /// Read a `bool` from the field.
    #[inline(always)]
    pub fn read_bool(&self) -> bool {
        self.data[0] != 0
    }

    /// Read an `i64` from the field (LE).
    #[inline(always)]
    pub fn read_i64(&self) -> i64 {
        i64::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
        ])
    }

    /// Read an `i32` from the field (LE).
    #[inline(always)]
    pub fn read_i32(&self) -> i32 {
        i32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    /// Read an `i16` from the field (LE).
    #[inline(always)]
    pub fn read_i16(&self) -> i16 {
        i16::from_le_bytes([self.data[0], self.data[1]])
    }

    /// Read a `u128` from the field (LE).
    #[inline(always)]
    pub fn read_u128(&self) -> u128 {
        u128::from_le_bytes([
            self.data[0],  self.data[1],  self.data[2],  self.data[3],
            self.data[4],  self.data[5],  self.data[6],  self.data[7],
            self.data[8],  self.data[9],  self.data[10], self.data[11],
            self.data[12], self.data[13], self.data[14], self.data[15],
        ])
    }

    /// Read an `i128` from the field (LE).
    #[inline(always)]
    pub fn read_i128(&self) -> i128 {
        i128::from_le_bytes([
            self.data[0],  self.data[1],  self.data[2],  self.data[3],
            self.data[4],  self.data[5],  self.data[6],  self.data[7],
            self.data[8],  self.data[9],  self.data[10], self.data[11],
            self.data[12], self.data[13], self.data[14], self.data[15],
        ])
    }

    /// Read an `i8` from the field.
    #[inline(always)]
    pub fn read_i8(&self) -> i8 {
        self.data[0] as i8
    }
}

/// Mutable typed view over a field-sized byte slice.
///
/// Produced by `split_fields_mut` (generated by `zero_copy_layout!`).
/// Provides typed mutation without holding `&mut` to the whole struct.
pub struct FieldMut<'a> {
    data: &'a mut [u8],
}

impl<'a> FieldMut<'a> {
    /// Create a mutable field reference from a byte slice.
    #[inline(always)]
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data }
    }

    /// The byte length of this field.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the field is zero-length.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Read the raw bytes.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.data
    }

    /// Get the raw mutable bytes.
    #[inline(always)]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data
    }

    /// Read as an `Address` (32-byte public key).
    #[inline(always)]
    pub fn read_address(&self) -> pinocchio::Address {
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&self.data[..32]);
        pinocchio::Address::from(addr)
    }

    /// Borrow the field bytes as an `&Address` reference.
    #[inline(always)]
    pub fn as_address(&self) -> &pinocchio::Address {
        // SAFETY: Address is repr(transparent) over [u8; 32], alignment 1.
        let ptr = self.data[..32].as_ptr() as *const pinocchio::Address;
        unsafe { &*ptr }
    }

    /// Write an `Address` (32-byte public key).
    #[inline(always)]
    pub fn write_address(&mut self, addr: &pinocchio::Address) {
        self.data[..32].copy_from_slice(addr.as_ref());
    }

    /// Write a `u64` (LE).
    #[inline(always)]
    pub fn write_u64(&mut self, v: u64) {
        self.data[..8].copy_from_slice(&v.to_le_bytes());
    }

    /// Write a `u32` (LE).
    #[inline(always)]
    pub fn write_u32(&mut self, v: u32) {
        self.data[..4].copy_from_slice(&v.to_le_bytes());
    }

    /// Write a `u16` (LE).
    #[inline(always)]
    pub fn write_u16(&mut self, v: u16) {
        self.data[..2].copy_from_slice(&v.to_le_bytes());
    }

    /// Write a `u8`.
    #[inline(always)]
    pub fn write_u8(&mut self, v: u8) {
        self.data[0] = v;
    }

    /// Write a `bool`.
    #[inline(always)]
    pub fn write_bool(&mut self, v: bool) {
        self.data[0] = v as u8;
    }

    /// Write an `i64` (LE).
    #[inline(always)]
    pub fn write_i64(&mut self, v: i64) {
        self.data[..8].copy_from_slice(&v.to_le_bytes());
    }

    /// Write an `i32` (LE).
    #[inline(always)]
    pub fn write_i32(&mut self, v: i32) {
        self.data[..4].copy_from_slice(&v.to_le_bytes());
    }

    /// Write an `i16` (LE).
    #[inline(always)]
    pub fn write_i16(&mut self, v: i16) {
        self.data[..2].copy_from_slice(&v.to_le_bytes());
    }

    /// Write an `i8`.
    #[inline(always)]
    pub fn write_i8(&mut self, v: i8) {
        self.data[0] = v as u8;
    }

    /// Write a `u128` (LE).
    #[inline(always)]
    pub fn write_u128(&mut self, v: u128) {
        self.data[..16].copy_from_slice(&v.to_le_bytes());
    }

    /// Write an `i128` (LE).
    #[inline(always)]
    pub fn write_i128(&mut self, v: i128) {
        self.data[..16].copy_from_slice(&v.to_le_bytes());
    }

    /// Read a `u64` (LE).
    #[inline(always)]
    pub fn read_u64(&self) -> u64 {
        u64::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
        ])
    }

    /// Read a `u32` (LE).
    #[inline(always)]
    pub fn read_u32(&self) -> u32 {
        u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    /// Read a `u16` (LE).
    #[inline(always)]
    pub fn read_u16(&self) -> u16 {
        u16::from_le_bytes([self.data[0], self.data[1]])
    }

    /// Read a `u8`.
    #[inline(always)]
    pub fn read_u8(&self) -> u8 {
        self.data[0]
    }

    /// Read a `bool`.
    #[inline(always)]
    pub fn read_bool(&self) -> bool {
        self.data[0] != 0
    }

    /// Read an `i64` (LE).
    #[inline(always)]
    pub fn read_i64(&self) -> i64 {
        i64::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
        ])
    }

    /// Read an `i32` (LE).
    #[inline(always)]
    pub fn read_i32(&self) -> i32 {
        i32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    /// Read an `i16` (LE).
    #[inline(always)]
    pub fn read_i16(&self) -> i16 {
        i16::from_le_bytes([self.data[0], self.data[1]])
    }

    /// Read an `i8`.
    #[inline(always)]
    pub fn read_i8(&self) -> i8 {
        self.data[0] as i8
    }

    /// Read a `u128` (LE).
    #[inline(always)]
    pub fn read_u128(&self) -> u128 {
        u128::from_le_bytes([
            self.data[0],  self.data[1],  self.data[2],  self.data[3],
            self.data[4],  self.data[5],  self.data[6],  self.data[7],
            self.data[8],  self.data[9],  self.data[10], self.data[11],
            self.data[12], self.data[13], self.data[14], self.data[15],
        ])
    }

    /// Read an `i128` (LE).
    #[inline(always)]
    pub fn read_i128(&self) -> i128 {
        i128::from_le_bytes([
            self.data[0],  self.data[1],  self.data[2],  self.data[3],
            self.data[4],  self.data[5],  self.data[6],  self.data[7],
            self.data[8],  self.data[9],  self.data[10], self.data[11],
            self.data[12], self.data[13], self.data[14], self.data[15],
        ])
    }

    /// Copy from a byte slice (e.g., an Address).
    #[inline(always)]
    pub fn copy_from(&mut self, src: &[u8]) {
        self.data[..src.len()].copy_from_slice(src);
    }
}
