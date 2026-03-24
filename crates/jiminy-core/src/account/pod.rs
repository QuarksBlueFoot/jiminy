//! Opt-in zero-copy POD (Plain Old Data) casting.
//!
//! Provides a safe API for reinterpreting byte slices as typed structs,
//! when the struct is `#[repr(C)]`, `Copy`, and all bit patterns are valid.
//!
//! ## Alignment
//!
//! Solana's loader aligns program input to 8-byte boundaries. The
//! functions below always check alignment on all targets and return
//! `Err(InvalidAccountData)` if the pointer is misaligned. This prevents
//! UB for types with alignment > 8 (e.g. `u128`). Use `Le128` / `Le*`
//! wrappers for 16-byte scalars.
//!
//! - **`pod_from_bytes` / `pod_from_bytes_mut`**: return a direct
//!   reference into the byte slice (zero-copy). Return
//!   `Err(InvalidAccountData)` if the pointer is misaligned.
//! - **`pod_read`**: copies via `read_unaligned`, so it works
//!   regardless of pointer alignment. Returns an owned `T`, not a
//!   reference. Ideal for native tests with uncontrolled alignment.
//!
//! # Usage
//!
//! ```rust,ignore
//! #[repr(C)]
//! #[derive(Clone, Copy)]
//! struct MyState {
//!     value: u64,
//!     counter: u32,
//! }
//!
//! // SAFETY: MyState is repr(C), Copy, and all bit patterns are valid.
//! unsafe impl Pod for MyState {}
//! impl FixedLayout for MyState { const SIZE: usize = 12; }
//!
//! let state = pod_from_bytes::<MyState>(&data)?;
//! ```

use pinocchio::error::ProgramError;

/// Marker trait for types that can be safely transmuted from any byte pattern.
///
/// # Safety
///
/// The implementing type must be:
/// - `#[repr(C)]` or `#[repr(transparent)]`
/// - `Copy`
/// - Valid for all possible bit patterns (no padding-dependent invariants)
/// - No interior references or pointers
pub unsafe trait Pod: Copy + 'static {}

/// Implement `Pod` for a list of types in one shot.
///
/// Saves you from writing N identical `unsafe impl Pod for T {}` blocks.
///
/// ```rust,ignore
/// impl_pod!(u8, u16, u32, u64, MyReprCStruct);
/// ```
#[macro_export]
macro_rules! impl_pod {
    ($($t:ty),+ $(,)?) => {
        $( unsafe impl $crate::account::Pod for $t {} )+
    };
}

// Blanket impls for primitives.
impl_pod!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, bool);

/// Trait for types with a known compile-time size.
pub trait FixedLayout {
    /// The exact byte size of this type on-chain.
    const SIZE: usize;
}

/// Reinterpret a byte slice as an immutable reference to `T`.
///
/// Returns `AccountDataTooSmall` if the slice is shorter than `T::SIZE`.
/// Returns `InvalidAccountData` if the pointer is misaligned.
///
/// Solana's loader aligns program input to 8-byte boundaries, which is
/// sufficient for most layout types but not for alignments > 8 (e.g.
/// `u128`). Use [`Le128`](crate::abi::Le128) for 16-byte scalars.
/// For alignment-safe access by copy, use [`pod_read`] instead.
#[inline(always)]
pub fn pod_from_bytes<T: Pod + FixedLayout>(data: &[u8]) -> Result<&T, ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    // Always check alignment. Solana loader aligns to 8 bytes, which is
    // not sufficient for types requiring >8 alignment. Treat misalignment
    // as InvalidAccountData rather than risking UB.
    if (data.as_ptr() as usize) % core::mem::align_of::<T>() != 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(unsafe { &*(data.as_ptr() as *const T) })
}

/// Reinterpret a mutable byte slice as a mutable reference to `T`.
///
/// Returns `AccountDataTooSmall` if the slice is shorter than `T::SIZE`.
/// Returns `InvalidAccountData` if the pointer is misaligned.
///
/// See [`pod_from_bytes`] for alignment details.
#[inline(always)]
pub fn pod_from_bytes_mut<T: Pod + FixedLayout>(data: &mut [u8]) -> Result<&mut T, ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if (data.as_ptr() as usize) % core::mem::align_of::<T>() != 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(unsafe { &mut *(data.as_mut_ptr() as *mut T) })
}

/// Read a `T` value from a byte slice by copy (alignment-safe on all targets).
///
/// Unlike [`pod_from_bytes`] this always works regardless of pointer
/// alignment, making it ideal for native tests. Returns an owned `T`.
///
/// ```rust,ignore
/// let header: AccountHeader = pod_read::<AccountHeader>(&data)?;
/// ```
#[inline(always)]
pub fn pod_read<T: Pod + FixedLayout>(data: &[u8]) -> Result<T, ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(unsafe { core::ptr::read_unaligned(data.as_ptr() as *const T) })
}

/// Write a Pod value into a byte slice at offset 0.
#[inline(always)]
pub fn pod_write<T: Pod + FixedLayout>(data: &mut [u8], value: &T) -> Result<(), ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let src = value as *const T as *const u8;
    let dst = data.as_mut_ptr();
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, T::SIZE);
    }
    Ok(())
}
