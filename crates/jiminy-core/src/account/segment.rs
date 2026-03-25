//! Zero-copy segmented account access.
//!
//! Extends the fixed-size `zero_copy_layout!` pattern with support for
//! **multiple variable-length arrays** within a single account. Each
//! array is called a *segment*.
//!
//! ## On-chain layout
//!
//! ```text
//! ┌──────────────┬──────────────────┬──────────────────────────┐
//! │ Fixed Prefix │  Segment Table   │     Segment Data         │
//! │  (N bytes)   │  (S × 8 bytes)   │  (variable per segment)  │
//! └──────────────┴──────────────────┴──────────────────────────┘
//! ```
//!
//! The fixed prefix is a standard `zero_copy_layout!` struct (including
//! the 16-byte `AccountHeader`). Immediately after it comes the segment
//! table: `S` entries of 8 bytes each, describing the offset, count, and
//! element size of each dynamic array. Segment data follows the table.
//!
//! ## Segment Descriptor (8 bytes)
//!
//! ```text
//! Byte   Field          Type      Description
//! ──────────────────────────────────────────────────────────
//! 0-3    offset         u32 LE    Byte offset from account start
//! 4-5    count          u16 LE    Number of elements
//! 6-7    element_size   u16 LE    Size of each element in bytes
//! ──────────────────────────────────────────────────────────
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use jiminy_core::account::segment::*;
//!
//! // Read segment table from account data starting after the fixed prefix.
//! let table = SegmentTable::from_bytes(&data[prefix_len..], 2)?;
//! let desc = table.descriptor(0)?;
//!
//! // Get a typed zero-copy view of the segment.
//! let orders = SegmentSlice::<Order>::from_descriptor(&data, desc)?;
//! for order in orders.iter() {
//!     // order: Order (by copy, alignment-safe)
//! }
//! ```

use pinocchio::error::ProgramError;

use super::pod::{FixedLayout, Pod};

/// Size of a single segment descriptor in bytes.
pub const SEGMENT_DESC_SIZE: usize = 8;

/// Maximum number of segments per account.
///
/// Practical upper bound to prevent excessive rent costs and simplify
/// validation. 8 segments × 8 bytes = 64-byte table overhead.
pub const MAX_SEGMENTS: usize = 8;

/// On-wire segment descriptor.
///
/// Each 8-byte entry describes one variable-length array within a
/// segmented account. The descriptor lives in the segment table region,
/// between the fixed prefix and the segment data.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentDescriptor {
    /// Byte offset from the start of the account data to the first
    /// element of this segment.
    pub offset: [u8; 4],
    /// Number of elements currently stored in this segment.
    pub count: [u8; 2],
    /// Size of each element in bytes.
    pub element_size: [u8; 2],
}

// SAFETY: repr(C), Copy, all fields are byte arrays. All bit patterns valid.
unsafe impl Pod for SegmentDescriptor {}

impl FixedLayout for SegmentDescriptor {
    const SIZE: usize = SEGMENT_DESC_SIZE;
}

const _: () = assert!(core::mem::size_of::<SegmentDescriptor>() == SEGMENT_DESC_SIZE);
const _: () = assert!(core::mem::align_of::<SegmentDescriptor>() == 1);

impl SegmentDescriptor {
    /// Create a new descriptor.
    #[inline(always)]
    pub const fn new(offset: u32, count: u16, element_size: u16) -> Self {
        Self {
            offset: offset.to_le_bytes(),
            count: count.to_le_bytes(),
            element_size: element_size.to_le_bytes(),
        }
    }

    /// Read the byte offset.
    #[inline(always)]
    pub const fn offset(&self) -> u32 {
        u32::from_le_bytes(self.offset)
    }

    /// Read the element count.
    #[inline(always)]
    pub const fn count(&self) -> u16 {
        u16::from_le_bytes(self.count)
    }

    /// Read the element size.
    #[inline(always)]
    pub const fn element_size(&self) -> u16 {
        u16::from_le_bytes(self.element_size)
    }

    /// Total byte footprint of this segment's data (count × element_size).
    #[inline(always)]
    pub const fn data_len(&self) -> usize {
        self.count() as usize * self.element_size() as usize
    }

    /// Byte range `[offset .. offset + data_len)`. Returns `None` on overflow.
    #[inline(always)]
    pub const fn byte_range(&self) -> Option<(usize, usize)> {
        let start = self.offset() as usize;
        let len = self.data_len();
        match start.checked_add(len) {
            Some(end) => Some((start, end)),
            None => None,
        }
    }
}

// ── Segment Table ────────────────────────────────────────────────────────────

/// Immutable view over the segment table region of an account.
///
/// The table starts at a known offset (typically right after the fixed
/// prefix) and contains `segment_count` descriptors of 8 bytes each.
pub struct SegmentTable<'a> {
    /// Slice covering just the segment table bytes.
    data: &'a [u8],
    /// Number of segments.
    segment_count: usize,
}

impl<'a> SegmentTable<'a> {
    /// Parse a segment table from `data`.
    ///
    /// `data` should start at the first descriptor byte.
    /// `segment_count` must be ≤ `MAX_SEGMENTS`.
    #[inline(always)]
    pub fn from_bytes(data: &'a [u8], segment_count: usize) -> Result<Self, ProgramError> {
        if segment_count > MAX_SEGMENTS {
            return Err(ProgramError::InvalidArgument);
        }
        let table_size = segment_count * SEGMENT_DESC_SIZE;
        if data.len() < table_size {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            data: &data[..table_size],
            segment_count,
        })
    }

    /// Number of segments in the table.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.segment_count
    }

    /// Whether the table has no segments.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.segment_count == 0
    }

    /// Get the descriptor at `index`.
    #[inline(always)]
    pub fn descriptor(&self, index: usize) -> Result<SegmentDescriptor, ProgramError> {
        if index >= self.segment_count {
            return Err(ProgramError::InvalidArgument);
        }
        let start = index * SEGMENT_DESC_SIZE;
        Ok(SegmentDescriptor {
            offset: [
                self.data[start],
                self.data[start + 1],
                self.data[start + 2],
                self.data[start + 3],
            ],
            count: [self.data[start + 4], self.data[start + 5]],
            element_size: [self.data[start + 6], self.data[start + 7]],
        })
    }

    /// Validate that all segments are well-formed within `account_len` bytes.
    ///
    /// `min_offset` is the earliest byte at which segment data may start
    /// (typically `DATA_START_OFFSET` - after the fixed prefix + table).
    /// This prevents segment data from overlapping the fixed prefix or
    /// the segment table itself.
    ///
    /// Checks:
    /// - Element size matches `expected_sizes[i]` (if provided).
    /// - Segment data fits within the account.
    /// - No segment data starts before `min_offset`.
    /// - No two segments overlap.
    /// - All segments are ordered by offset.
    #[inline]
    pub fn validate(
        &self,
        account_len: usize,
        expected_sizes: &[u16],
        min_offset: usize,
    ) -> Result<(), ProgramError> {
        let mut prev_end: usize = min_offset;

        for i in 0..self.segment_count {
            let desc = self.descriptor(i)?;

            // Element size must be non-zero.
            if desc.element_size() == 0 {
                return Err(ProgramError::InvalidAccountData);
            }

            // Check expected element size if provided.
            if i < expected_sizes.len() && desc.element_size() != expected_sizes[i] {
                return Err(ProgramError::InvalidAccountData);
            }

            // Compute byte range with overflow check.
            let (start, end) = desc
                .byte_range()
                .ok_or(ProgramError::InvalidAccountData)?;

            // Must fit within account data.
            if end > account_len {
                return Err(ProgramError::AccountDataTooSmall);
            }

            // Must be ordered and non-overlapping.
            if start < prev_end {
                return Err(ProgramError::InvalidAccountData);
            }

            prev_end = end;
        }

        Ok(())
    }

    /// Total byte size of the table itself (segment_count × 8).
    #[inline(always)]
    pub fn byte_len(&self) -> usize {
        self.segment_count * SEGMENT_DESC_SIZE
    }
}

// ── Mutable Segment Table ────────────────────────────────────────────────────

/// Mutable view over the segment table region.
pub struct SegmentTableMut<'a> {
    data: &'a mut [u8],
    segment_count: usize,
}

impl<'a> SegmentTableMut<'a> {
    /// Parse a mutable segment table from `data`.
    #[inline(always)]
    pub fn from_bytes(data: &'a mut [u8], segment_count: usize) -> Result<Self, ProgramError> {
        if segment_count > MAX_SEGMENTS {
            return Err(ProgramError::InvalidArgument);
        }
        let table_size = segment_count * SEGMENT_DESC_SIZE;
        if data.len() < table_size {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            data: &mut data[..table_size],
            segment_count,
        })
    }

    /// Read the descriptor at `index`.
    #[inline(always)]
    pub fn descriptor(&self, index: usize) -> Result<SegmentDescriptor, ProgramError> {
        if index >= self.segment_count {
            return Err(ProgramError::InvalidArgument);
        }
        let start = index * SEGMENT_DESC_SIZE;
        Ok(SegmentDescriptor {
            offset: [
                self.data[start],
                self.data[start + 1],
                self.data[start + 2],
                self.data[start + 3],
            ],
            count: [self.data[start + 4], self.data[start + 5]],
            element_size: [self.data[start + 6], self.data[start + 7]],
        })
    }

    /// Write a descriptor at `index`.
    #[inline(always)]
    pub fn set_descriptor(
        &mut self,
        index: usize,
        desc: &SegmentDescriptor,
    ) -> Result<(), ProgramError> {
        if index >= self.segment_count {
            return Err(ProgramError::InvalidArgument);
        }
        let start = index * SEGMENT_DESC_SIZE;
        self.data[start..start + 4].copy_from_slice(&desc.offset);
        self.data[start + 4..start + 6].copy_from_slice(&desc.count);
        self.data[start + 6..start + 8].copy_from_slice(&desc.element_size);
        Ok(())
    }

    /// Number of segments.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.segment_count
    }

    /// Whether the table has no segments.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.segment_count == 0
    }

    /// Initialize the segment table with descriptors computed from
    /// element sizes and initial counts.
    ///
    /// `specs` is a slice of `(element_size, initial_count)` pairs.
    /// Offsets are computed automatically, starting at `data_start`
    /// (typically `fixed_prefix_len + table_size`).
    #[inline]
    pub fn init(
        data: &'a mut [u8],
        data_start: u32,
        specs: &[(u16, u16)],
    ) -> Result<Self, ProgramError> {
        let segment_count = specs.len();
        if segment_count > MAX_SEGMENTS {
            return Err(ProgramError::InvalidArgument);
        }
        let table_size = segment_count * SEGMENT_DESC_SIZE;
        if data.len() < table_size {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let mut offset = data_start;
        for (i, &(elem_size, count)) in specs.iter().enumerate() {
            let start = i * SEGMENT_DESC_SIZE;
            data[start..start + 4].copy_from_slice(&offset.to_le_bytes());
            data[start + 4..start + 6].copy_from_slice(&count.to_le_bytes());
            data[start + 6..start + 8].copy_from_slice(&elem_size.to_le_bytes());
            // Advance offset with overflow check.
            let seg_len = (count as u32)
                .checked_mul(elem_size as u32)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            offset = offset
                .checked_add(seg_len)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        }

        Ok(Self {
            data: &mut data[..table_size],
            segment_count,
        })
    }
}

// ── Segment Slice (immutable) ────────────────────────────────────────────────

/// Immutable zero-copy view over one segment's element array.
///
/// Similar to `ZeroCopySlice` but driven by a `SegmentDescriptor`
/// rather than a length prefix. Elements are `Pod + FixedLayout`.
pub struct SegmentSlice<'a, T: Pod + FixedLayout> {
    data: &'a [u8],
    count: u16,
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> SegmentSlice<'a, T> {
    /// Create a segment view from a descriptor and the full account data.
    ///
    /// Validates that:
    /// - `descriptor.element_size()` matches `T::SIZE`
    /// - the segment's byte range fits within `account_data`
    #[inline(always)]
    pub fn from_descriptor(
        account_data: &'a [u8],
        descriptor: &SegmentDescriptor,
    ) -> Result<Self, ProgramError> {
        if descriptor.element_size() as usize != T::SIZE {
            return Err(ProgramError::InvalidAccountData);
        }
        let (start, end) = descriptor
            .byte_range()
            .ok_or(ProgramError::InvalidAccountData)?;
        if end > account_data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            data: &account_data[start..end],
            count: descriptor.count(),
            _marker: core::marker::PhantomData,
        })
    }

    /// Number of elements.
    #[inline(always)]
    pub fn len(&self) -> u16 {
        self.count
    }

    /// Whether the segment is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get a reference to element at `index`.
    #[inline(always)]
    pub fn get(&self, index: u16) -> Result<&T, ProgramError> {
        if index >= self.count {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = (index as usize) * T::SIZE;
        #[cfg(target_os = "solana")]
        {
            // SAFETY: bounds checked above, alignment is 1 on SBF.
            Ok(unsafe { &*(self.data.as_ptr().add(offset) as *const T) })
        }
        #[cfg(not(target_os = "solana"))]
        {
            let ptr = self.data.as_ptr();
            // SAFETY: bounds checked above. Alignment checked below.
            if (unsafe { ptr.add(offset) } as usize) % core::mem::align_of::<T>() != 0 {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(unsafe { &*(ptr.add(offset) as *const T) })
        }
    }

    /// Read element at `index` by copy (alignment-safe on all targets).
    #[inline(always)]
    pub fn read(&self, index: u16) -> Result<T, ProgramError> {
        if index >= self.count {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = (index as usize) * T::SIZE;
        // SAFETY: bounds checked above.
        Ok(unsafe {
            core::ptr::read_unaligned(self.data.as_ptr().add(offset) as *const T)
        })
    }

    /// Iterate over all elements by copy.
    #[inline(always)]
    pub fn iter(&self) -> SegmentIter<'a, T> {
        SegmentIter {
            data: self.data,
            index: 0,
            count: self.count,
            _marker: core::marker::PhantomData,
        }
    }

    /// Raw byte slice of the segment data.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.data
    }
}

// ── Segment Slice (mutable) ──────────────────────────────────────────────────

/// Mutable zero-copy view over one segment's element array.
pub struct SegmentSliceMut<'a, T: Pod + FixedLayout> {
    data: &'a mut [u8],
    count: u16,
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> SegmentSliceMut<'a, T> {
    /// Create a mutable segment view from a descriptor and full account data.
    #[inline(always)]
    pub fn from_descriptor(
        account_data: &'a mut [u8],
        descriptor: &SegmentDescriptor,
    ) -> Result<Self, ProgramError> {
        if descriptor.element_size() as usize != T::SIZE {
            return Err(ProgramError::InvalidAccountData);
        }
        let (start, end) = descriptor
            .byte_range()
            .ok_or(ProgramError::InvalidAccountData)?;
        if end > account_data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            data: &mut account_data[start..end],
            count: descriptor.count(),
            _marker: core::marker::PhantomData,
        })
    }

    /// Number of elements.
    #[inline(always)]
    pub fn len(&self) -> u16 {
        self.count
    }

    /// Whether the segment is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get a mutable reference to element at `index`.
    #[inline(always)]
    pub fn get_mut(&mut self, index: u16) -> Result<&mut T, ProgramError> {
        if index >= self.count {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = (index as usize) * T::SIZE;
        #[cfg(target_os = "solana")]
        {
            // SAFETY: bounds checked above, alignment is 1 on SBF.
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

    /// Get an immutable reference to element at `index`.
    #[inline(always)]
    pub fn get(&self, index: u16) -> Result<&T, ProgramError> {
        if index >= self.count {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = (index as usize) * T::SIZE;
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
    pub fn set(&mut self, index: u16, value: &T) -> Result<(), ProgramError> {
        if index >= self.count {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = (index as usize) * T::SIZE;
        let src = value as *const T as *const u8;
        // SAFETY: bounds checked above, copy is non-overlapping.
        unsafe {
            core::ptr::copy_nonoverlapping(
                src,
                self.data.as_mut_ptr().add(offset),
                T::SIZE,
            );
        }
        Ok(())
    }

    /// Read element at `index` by copy (alignment-safe on all targets).
    #[inline(always)]
    pub fn read(&self, index: u16) -> Result<T, ProgramError> {
        if index >= self.count {
            return Err(ProgramError::InvalidArgument);
        }
        let offset = (index as usize) * T::SIZE;
        Ok(unsafe {
            core::ptr::read_unaligned(self.data.as_ptr().add(offset) as *const T)
        })
    }

    /// Raw byte slice of the segment data.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.data
    }
}

// ── Segment Iterator ─────────────────────────────────────────────────────────

/// Iterator over elements in a [`SegmentSlice`], yielding copies.
pub struct SegmentIter<'a, T: Pod + FixedLayout> {
    data: &'a [u8],
    index: u16,
    count: u16,
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Pod + FixedLayout> Iterator for SegmentIter<'a, T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let offset = (self.index as usize) * T::SIZE;
        self.index += 1;
        // SAFETY: bounds checked by constructor + index < count.
        Some(unsafe {
            core::ptr::read_unaligned(self.data.as_ptr().add(offset) as *const T)
        })
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.count - self.index) as usize;
        (remaining, Some(remaining))
    }
}

impl<'a, T: Pod + FixedLayout> ExactSizeIterator for SegmentIter<'a, T> {}

// ── Segment push / swap-remove ───────────────────────────────────────────────

/// Push an element at the end of a segment.
///
/// Reads the current descriptor at `seg_index`, writes `value` after
/// the last entry, then increments the descriptor count.
///
/// # Errors
///
/// - `AccountDataTooSmall` if the account data is too short for the new element.
/// - `InvalidAccountData` if `T::SIZE` doesn't match the descriptor's element size.
/// - `ArithmeticOverflow` if the count would exceed `u16::MAX`.
#[inline]
pub fn segment_push<T: Pod + FixedLayout>(
    data: &mut [u8],
    table_offset: usize,
    segment_count: usize,
    seg_index: usize,
    value: &T,
) -> Result<(), ProgramError> {
    // Read descriptor and upper bound (scoped to release the shared borrow).
    let (desc, upper_bound) = {
        let table = SegmentTable::from_bytes(&data[table_offset..], segment_count)?;
        let d = table.descriptor(seg_index)?;
        // Upper bound: for the last segment, data.len(). For earlier
        // segments, the next segment's offset. This prevents push from
        // writing into an adjacent segment's region.
        let ub = if seg_index + 1 < segment_count {
            table.descriptor(seg_index + 1)?.offset() as usize
        } else {
            data.len()
        };
        (d, ub)
    };

    if desc.element_size() as usize != T::SIZE {
        return Err(ProgramError::InvalidAccountData);
    }

    let current_count = desc.count();
    let write_offset = desc.offset() as usize + (current_count as usize) * T::SIZE;
    let write_end = write_offset + T::SIZE;

    if write_end > upper_bound {
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Write the element bytes.
    let src = value as *const T as *const u8;
    // SAFETY: bounds checked above; copy is non-overlapping (new slot).
    unsafe {
        core::ptr::copy_nonoverlapping(src, data.as_mut_ptr().add(write_offset), T::SIZE);
    }

    // Increment the descriptor count.
    let new_count = current_count
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    let updated = SegmentDescriptor::new(desc.offset(), new_count, desc.element_size());
    let mut table_mut = SegmentTableMut::from_bytes(&mut data[table_offset..], segment_count)?;
    table_mut.set_descriptor(seg_index, &updated)?;

    Ok(())
}

/// Remove element at `index` by swapping it with the last element.
///
/// Returns the removed element by copy. The last slot is zeroed.
/// Order is **not** preserved (O(1) removal).
///
/// # Errors
///
/// - `InvalidArgument` if `index >= count`.
/// - `InvalidAccountData` if `T::SIZE` doesn't match the descriptor's element size.
#[inline]
pub fn segment_swap_remove<T: Pod + FixedLayout>(
    data: &mut [u8],
    table_offset: usize,
    segment_count: usize,
    seg_index: usize,
    index: u16,
) -> Result<T, ProgramError> {
    let desc = {
        let table = SegmentTable::from_bytes(&data[table_offset..], segment_count)?;
        table.descriptor(seg_index)?
    };

    if desc.element_size() as usize != T::SIZE {
        return Err(ProgramError::InvalidAccountData);
    }

    let count = desc.count();
    if index >= count {
        return Err(ProgramError::InvalidArgument);
    }

    let base = desc.offset() as usize;
    let target_offset = base + (index as usize) * T::SIZE;

    // Read the element being removed (by copy).
    // SAFETY: bounds guaranteed by descriptor validation.
    let removed = unsafe {
        core::ptr::read_unaligned(data.as_ptr().add(target_offset) as *const T)
    };

    let last_index = count - 1;
    if index < last_index {
        // Copy last element into the target slot.
        let last_offset = base + (last_index as usize) * T::SIZE;
        data.copy_within(last_offset..last_offset + T::SIZE, target_offset);
    }

    // Zero the now-unused last slot (compiles to sol_memset on SBF).
    let last_offset = base + (last_index as usize) * T::SIZE;
    data[last_offset..last_offset + T::SIZE].fill(0);

    // Decrement the descriptor count.
    let updated = SegmentDescriptor::new(desc.offset(), last_index, desc.element_size());
    let mut table_mut = SegmentTableMut::from_bytes(&mut data[table_offset..], segment_count)?;
    table_mut.set_descriptor(seg_index, &updated)?;

    Ok(removed)
}
