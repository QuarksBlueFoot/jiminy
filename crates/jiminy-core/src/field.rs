//! Typed field descriptors for Jiminy layouts.
//!
//! `Field<T>` gives Jiminy a first-class, reusable way to talk about
//! fields as named, typed regions inside a zero-copy layout. This is the
//! bridge from raw offsets to inspectable state contracts.

use core::marker::PhantomData;

/// Typed descriptor for a field inside a zero-copy layout.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Field<T> {
    /// Human-readable field name.
    pub name: &'static str,
    /// Byte offset from the start of the containing layout.
    pub offset: usize,
    _marker: PhantomData<T>,
}

impl<T> Field<T> {
    /// Create a new typed field descriptor.
    #[inline(always)]
    pub const fn new(name: &'static str, offset: usize) -> Self {
        Self {
            name,
            offset,
            _marker: PhantomData,
        }
    }

    /// Byte width of the field type.
    #[inline(always)]
    pub const fn size(&self) -> usize {
        core::mem::size_of::<T>()
    }

    /// Exclusive end offset.
    #[inline(always)]
    pub const fn end(&self) -> usize {
        self.offset + self.size()
    }

    /// Read the field with bounds checking.
    #[inline(always)]
    pub fn get<'a>(&self, data: &'a [u8]) -> Option<&'a T> {
        if data.len() < self.end() {
            return None;
        }
        Some(unsafe { &*(data.as_ptr().add(self.offset) as *const T) })
    }

    /// Mutably read the field with bounds checking.
    #[inline(always)]
    pub fn get_mut<'a>(&self, data: &'a mut [u8]) -> Option<&'a mut T> {
        if data.len() < self.end() {
            return None;
        }
        Some(unsafe { &mut *(data.as_mut_ptr().add(self.offset) as *mut T) })
    }

    /// Read the field without bounds checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure `data` is large enough and correctly aligned for `T`.
    #[inline(always)]
    pub unsafe fn get_unchecked<'a>(&self, data: &'a [u8]) -> &'a T {
        unsafe { &*(data.as_ptr().add(self.offset) as *const T) }
    }

    /// Mutably read the field without bounds checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure `data` is large enough and correctly aligned for `T`.
    #[inline(always)]
    pub unsafe fn get_mut_unchecked<'a>(&self, data: &'a mut [u8]) -> &'a mut T {
        unsafe { &mut *(data.as_mut_ptr().add(self.offset) as *mut T) }
    }
}
