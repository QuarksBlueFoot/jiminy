//! Zero-copy dynamic-length collections overlaid on account data.
//!
//! Provides [`ZeroCopySlice`] and [`ZeroCopySliceMut`]: length-prefixed
//! arrays of `Pod` items that read directly from borrowed account bytes
//! without deserialization or allocation.
//!
//! ## On-chain layout
//!
//! ```text
//! [len: u32 LE] [item_0] [item_1] ... [item_{len-1}]
//! ```
//!
//! Each item occupies exactly `T::SIZE` bytes.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use jiminy_core::account::collection::{ZeroCopySlice, ZeroCopySliceMut};
//! use pinocchio::Address;
//!
//! // Read a whitelist from account data at some offset:
//! let whitelist = ZeroCopySlice::<Address>::from_bytes(&data[offset..])?;
//! let third: &Address = whitelist.get(2)?;
//!
//! // Write:
//! let mut list = ZeroCopySliceMut::<Address>::from_bytes(&mut data[offset..])?;
//! *list.get_mut(0)? = new_address;
//! ```

use pinocchio::error::ProgramError;

use super::pod::{FixedLayout, Pod};

/// Length prefix size (u32 LE = 4 bytes).
const LEN_PREFIX: usize = 4;

/// Immutable zero-copy view over a length-prefixed array in account data.
///
/// `T` must implement [`Pod`] + [`FixedLayout`]. No allocations, no copies.
pub struct ZeroCopySlice<'a, T: Pod + FixedLayout> {
    len: u32,
    data: &'a [u8],
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> ZeroCopySlice<'a, T> {
    /// Create a view over `[len: u32][T; len]` at the start of `data`.
    ///
    /// Returns `AccountDataTooSmall` if the slice is too short for the
    /// declared length.
    #[inline(always)]
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, ProgramError> {
        if data.len() < LEN_PREFIX {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let required = LEN_PREFIX + (len as usize) * T::SIZE;
        if data.len() < required {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            len,
            data,
            _marker: core::marker::PhantomData,
        })
    }

    /// Number of items in the collection.
    #[inline(always)]
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Whether the collection is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total byte footprint: 4 (len prefix) + len * T::SIZE.
    #[inline(always)]
    pub fn byte_len(&self) -> usize {
        LEN_PREFIX + (self.len as usize) * T::SIZE
    }

    /// Get an immutable reference to the item at `index`.
    ///
    /// Returns `InvalidArgument` if out of bounds.
    #[inline(always)]
    pub fn get(&self, index: u32) -> Result<&T, ProgramError> {
        if index >= self.len {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = LEN_PREFIX + (index as usize) * T::SIZE;
        // On SBF alignment is always 1. On native we rely on T::SIZE being
        // the actual mem size for repr(C) Pod types.
        #[cfg(target_os = "solana")]
        {
            Ok(unsafe { &*(self.data.as_ptr().add(offset) as *const T) })
        }
        #[cfg(not(target_os = "solana"))]
        {
            let ptr = self.data.as_ptr();
            if (unsafe { ptr.add(offset) } as usize) % core::mem::align_of::<T>() != 0 {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(unsafe { &*(ptr.add(offset) as *const T) })
        }
    }

    /// Read item at `index` by copy (alignment-safe on all targets).
    #[inline(always)]
    pub fn read(&self, index: u32) -> Result<T, ProgramError> {
        if index >= self.len {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = LEN_PREFIX + (index as usize) * T::SIZE;
        Ok(unsafe {
            core::ptr::read_unaligned(self.data.as_ptr().add(offset) as *const T)
        })
    }

    /// Iterate over all items as references.
    #[inline(always)]
    pub fn iter(&self) -> ZeroCopyIter<'a, T> {
        ZeroCopyIter {
            data: self.data,
            index: 0,
            len: self.len,
            _marker: core::marker::PhantomData,
        }
    }

    /// Check if `needle` exists in the collection (linear scan).
    ///
    /// Compares raw bytes, works for any Pod type.
    #[inline(always)]
    pub fn contains_bytes(&self, needle: &[u8]) -> bool {
        if needle.len() != T::SIZE {
            return false;
        }
        let mut i = 0u32;
        while i < self.len {
            let offset = LEN_PREFIX + (i as usize) * T::SIZE;
            if &self.data[offset..offset + T::SIZE] == needle {
                return true;
            }
            i += 1;
        }
        false
    }
}

/// Mutable zero-copy view over a length-prefixed array in account data.
pub struct ZeroCopySliceMut<'a, T: Pod + FixedLayout> {
    len: u32,
    data: &'a mut [u8],
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> ZeroCopySliceMut<'a, T> {
    /// Create a mutable view over `[len: u32][T; len]` at the start of `data`.
    #[inline(always)]
    pub fn from_bytes(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        if data.len() < LEN_PREFIX {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let required = LEN_PREFIX + (len as usize) * T::SIZE;
        if data.len() < required {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            len,
            data,
            _marker: core::marker::PhantomData,
        })
    }

    /// Create a new collection in `data`, writing `len` as the prefix and
    /// zeroing the item region.
    #[inline(always)]
    pub fn init(data: &'a mut [u8], len: u32) -> Result<Self, ProgramError> {
        let required = LEN_PREFIX + (len as usize) * T::SIZE;
        if data.len() < required {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[0..4].copy_from_slice(&len.to_le_bytes());
        // Zero the item region (compiles to sol_memset on SBF).
        let item_region = &mut data[LEN_PREFIX..required];
        item_region.fill(0);
        Ok(Self {
            len,
            data,
            _marker: core::marker::PhantomData,
        })
    }

    /// Number of items.
    #[inline(always)]
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Whether the collection is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a mutable reference to the item at `index`.
    #[inline(always)]
    pub fn get_mut(&mut self, index: u32) -> Result<&mut T, ProgramError> {
        if index >= self.len {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = LEN_PREFIX + (index as usize) * T::SIZE;
        #[cfg(target_os = "solana")]
        {
            Ok(unsafe { &mut *(self.data.as_mut_ptr().add(offset) as *mut T) })
        }
        #[cfg(not(target_os = "solana"))]
        {
            let ptr = self.data.as_mut_ptr();
            if (unsafe { ptr.add(offset) } as usize) % core::mem::align_of::<T>() != 0 {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(unsafe { &mut *(ptr.add(offset) as *mut T) })
        }
    }

    /// Get an immutable reference to the item at `index`.
    #[inline(always)]
    pub fn get(&self, index: u32) -> Result<&T, ProgramError> {
        if index >= self.len {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = LEN_PREFIX + (index as usize) * T::SIZE;
        #[cfg(target_os = "solana")]
        {
            Ok(unsafe { &*(self.data.as_ptr().add(offset) as *const T) })
        }
        #[cfg(not(target_os = "solana"))]
        {
            let ptr = self.data.as_ptr();
            if (unsafe { ptr.add(offset) } as usize) % core::mem::align_of::<T>() != 0 {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(unsafe { &*(ptr.add(offset) as *const T) })
        }
    }

    /// Write a value at `index` via byte copy (alignment-safe).
    #[inline(always)]
    pub fn set(&mut self, index: u32, value: &T) -> Result<(), ProgramError> {
        if index >= self.len {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = LEN_PREFIX + (index as usize) * T::SIZE;
        let src = value as *const T as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(
                src,
                self.data.as_mut_ptr().add(offset),
                T::SIZE,
            );
        }
        Ok(())
    }
}

/// Iterator over items in a [`ZeroCopySlice`].
pub struct ZeroCopyIter<'a, T: Pod + FixedLayout> {
    data: &'a [u8],
    index: u32,
    len: u32,
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> Iterator for ZeroCopyIter<'a, T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        let offset = LEN_PREFIX + (self.index as usize) * T::SIZE;
        self.index += 1;
        Some(unsafe {
            core::ptr::read_unaligned(self.data.as_ptr().add(offset) as *const T)
        })
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.len - self.index) as usize;
        (remaining, Some(remaining))
    }
}

impl<'a, T: Pod + FixedLayout> ExactSizeIterator for ZeroCopyIter<'a, T> {}
