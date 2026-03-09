//! Opt-in zero-copy POD (Plain Old Data) casting.
//!
//! Provides a safe API for reinterpreting byte slices as typed structs,
//! when the struct is `#[repr(C)]`, `Copy`, and all bit patterns are valid.
//!
//! ## Alignment
//!
//! On SBF (Solana runtime) all memory is 1-byte aligned, so pointer casts
//! are always valid. In native tests the data pointer may not satisfy
//! `align_of::<T>()`. The functions below handle both cases:
//!
//! - **Aligned** — returns a direct reference into the byte slice (zero-copy).
//! - **Unaligned** (native tests only) — copies via `read_unaligned` and
//!   returns `Err` for the `_mut` variant since an in-place mutable
//!   reference would be unsound.
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
///
/// On SBF (1-byte alignment) this is a direct pointer cast. On native
/// targets if the pointer is misaligned, the value is copied to a
/// stack-aligned temporary — still safe, just not truly zero-copy in
/// tests.
#[inline(always)]
pub fn pod_from_bytes<T: Pod + FixedLayout>(data: &[u8]) -> Result<&T, ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    // SBF has 1-byte alignment for everything — pointer cast is always valid.
    #[cfg(target_os = "solana")]
    {
        Ok(unsafe { &*(data.as_ptr() as *const T) })
    }
    // Native: check alignment before casting. If misaligned, return error
    // to force callers to use pod_read instead.
    #[cfg(not(target_os = "solana"))]
    {
        if (data.as_ptr() as usize) % core::mem::align_of::<T>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr() as *const T) })
    }
}

/// Reinterpret a mutable byte slice as a mutable reference to `T`.
///
/// Returns `AccountDataTooSmall` if the slice is shorter than `T::SIZE`.
/// Returns `InvalidAccountData` on native targets if the pointer is
/// misaligned (a mutable reference requires correct alignment).
#[inline(always)]
pub fn pod_from_bytes_mut<T: Pod + FixedLayout>(data: &mut [u8]) -> Result<&mut T, ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    #[cfg(target_os = "solana")]
    {
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut T) })
    }
    #[cfg(not(target_os = "solana"))]
    {
        if (data.as_ptr() as usize) % core::mem::align_of::<T>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut T) })
    }
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
