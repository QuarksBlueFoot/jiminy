//! Type-safe wrappers for validated account data.
//!
//! `VerifiedAccount<T>` holds an immutable borrow whose size and type
//! were checked at construction. `VerifiedAccountMut<T>` is the mutable
//! counterpart backed by `RefMut`. Both make `get()` / `get_mut()`
//! infallible: invariants are enforced once at construction, not on
//! every access.

use core::marker::PhantomData;
use hopper_runtime::{Ref, RefMut};
use hopper_runtime::ProgramError;

use super::{Pod, FixedLayout, pod_from_bytes};

// ── Immutable ────────────────────────────────────────────────────────────────

/// Immutable typed wrapper around validated account data.
///
/// Created by `load()` / `load_foreign()` after all checks pass.
/// `get()` is infallible because size was validated at construction.
pub struct VerifiedAccount<'a, T: Pod + FixedLayout> {
    data: Ref<'a, [u8]>,
    _marker: PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> VerifiedAccount<'a, T> {
    /// Construct after validation. Returns `Err` if `data.len() < T::SIZE`
    /// or alignment is wrong.
    ///
    /// Not public API. Called by macro-generated `load()` / `load_foreign()`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn new(data: Ref<'a, [u8]>) -> Result<Self, ProgramError> {
        // Validate size once at construction so get() can be infallible.
        let _ = pod_from_bytes::<T>(&data)?;
        Ok(Self {
            data,
            _marker: PhantomData,
        })
    }

    /// Get an immutable reference to the typed data.
    ///
    /// Infallible: size and alignment were verified at construction.
    #[inline(always)]
    pub fn get(&self) -> &T {
        // SAFETY: size and alignment were checked in new().
        unsafe { &*(self.data.as_ptr() as *const T) }
    }

    /// Access the raw validated bytes.
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

// ── Mutable ──────────────────────────────────────────────────────────────────

/// Mutable typed wrapper around validated account data.
///
/// Created by `load_mut()` after all checks pass. Backed by `RefMut`
/// so mutable access is sound (no aliasing with `Ref`).
pub struct VerifiedAccountMut<'a, T: Pod + FixedLayout> {
    data: RefMut<'a, [u8]>,
    _marker: PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> VerifiedAccountMut<'a, T> {
    /// Construct after validation. Returns `Err` if `data.len() < T::SIZE`
    /// or alignment is wrong.
    ///
    /// Not public API. Called by macro-generated `load_mut()`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn new(data: RefMut<'a, [u8]>) -> Result<Self, ProgramError> {
        // Validate once at construction.
        let _ = pod_from_bytes::<T>(&data)?;
        Ok(Self {
            data,
            _marker: PhantomData,
        })
    }

    /// Get an immutable reference to the typed data.
    #[inline(always)]
    pub fn get(&self) -> &T {
        // SAFETY: size and alignment were checked in new().
        unsafe { &*(self.data.as_ptr() as *const T) }
    }

    /// Get a mutable reference to the typed data.
    ///
    /// Sound because `self.data` is a `RefMut` (exclusive borrow from
    /// the runtime) and `&mut self` guarantees no aliasing in Rust.
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: size and alignment were checked in new(). RefMut
        // guarantees exclusive access to the underlying bytes.
        unsafe { &mut *(self.data.as_mut_ptr() as *mut T) }
    }

    /// Access the raw validated bytes.
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Access the raw validated bytes mutably.
    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}
