//! VerifiedAccount<T>
//!
//! Typed wrapper around validated account data.
//! Prevents type confusion after validation.

use core::marker::PhantomData;
use pinocchio::account::Ref;
use super::{Pod, FixedLayout, pod_from_bytes};
use pinocchio::error::ProgramError;

pub struct VerifiedAccount<'a, T: Pod + FixedLayout> {
    data: Ref<'a, [u8]>,
    _marker: PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> VerifiedAccount<'a, T> {
    #[inline(always)]
    pub(crate) fn new(data: Ref<'a, [u8]>) -> Self {
        Self {
            data,
            _marker: PhantomData,
        }
    }

    /// Get immutable reference to typed data
    #[inline(always)]
    pub fn get(&self) -> Result<&T, ProgramError> {
        if self.data.len() != T::SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        pod_from_bytes::<T>(&self.data)
    }

    /// Get mutable reference to typed data
    #[inline(always)]
    pub fn get_mut(&mut self) -> Result<&mut T, ProgramError> {
        if self.data.len() != T::SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        // SAFETY: mutable access requires exclusive borrow by caller
        unsafe { super::pod::pod_from_bytes_mut::<T>(&mut self.data) }
    }
}
