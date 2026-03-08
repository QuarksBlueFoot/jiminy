//! Opt-in zero-copy POD (Plain Old Data) casting.
//!
//! Provides a safe API for reinterpreting byte slices as typed structs,
//! when the struct is `#[repr(C)]`, `Copy`, and all bit patterns are valid.
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

// Blanket impls for primitives.
unsafe impl Pod for u8 {}
unsafe impl Pod for u16 {}
unsafe impl Pod for u32 {}
unsafe impl Pod for u64 {}
unsafe impl Pod for u128 {}
unsafe impl Pod for i8 {}
unsafe impl Pod for i16 {}
unsafe impl Pod for i32 {}
unsafe impl Pod for i64 {}
unsafe impl Pod for i128 {}
unsafe impl Pod for bool {}

/// Trait for types with a known compile-time size.
pub trait FixedLayout {
    /// The exact byte size of this type on-chain.
    const SIZE: usize;
}

/// Reinterpret a byte slice as an immutable reference to `T`.
///
/// Returns `AccountDataTooSmall` if the slice is shorter than `T::SIZE`.
#[inline(always)]
pub fn pod_from_bytes<T: Pod + FixedLayout>(data: &[u8]) -> Result<&T, ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if (data.as_ptr() as usize) % core::mem::align_of::<T>() != 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(unsafe { &*(data.as_ptr() as *const T) })
}

/// Reinterpret a mutable byte slice as a mutable reference to `T`.
///
/// Returns `AccountDataTooSmall` if the slice is shorter than `T::SIZE`.
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
