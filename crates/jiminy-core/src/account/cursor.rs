//! Zero-copy read/write cursors over byte slices.
//!
//! [`SliceCursor`] for sequential reads, [`DataWriter`] for sequential writes.
//! Both are position-tracked and bounds-checked — you get
//! `AccountDataTooSmall` instead of a panic if you read/write past the end.

use pinocchio::{error::ProgramError, Address};

// ── SliceCursor ──────────────────────────────────────────────────────────────

/// Zero-copy read cursor over a byte slice.
///
/// Tracks the current position and reads typed fields sequentially.
/// Every read is bounds-checked.
///
/// ```rust,ignore
/// let data = account.try_borrow()?;
/// let mut cur = SliceCursor::new(&data[1..]); // skip discriminator
/// let balance   = cur.read_u64()?;
/// let recipient = cur.read_address()?;
/// let flags     = cur.read_u8()?;
/// ```
pub struct SliceCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> SliceCursor<'a> {
    #[inline(always)]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Bytes remaining from the current position.
    #[inline(always)]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Current byte offset into the slice.
    #[inline(always)]
    pub fn position(&self) -> usize {
        self.pos
    }

    #[inline(always)]
    pub fn read_u8(&mut self) -> Result<u8, ProgramError> {
        if self.pos >= self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = self.data[self.pos];
        self.pos += 1;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_u16(&mut self) -> Result<u16, ProgramError> {
        let end = self.pos + 2;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = u16::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_u32(&mut self) -> Result<u32, ProgramError> {
        let end = self.pos + 4;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = u32::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_u64(&mut self) -> Result<u64, ProgramError> {
        let end = self.pos + 8;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = u64::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_i64(&mut self) -> Result<i64, ProgramError> {
        let end = self.pos + 8;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = i64::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    /// `0` → `false`, anything else → `true`.
    #[inline(always)]
    pub fn read_bool(&mut self) -> Result<bool, ProgramError> {
        Ok(self.read_u8()? != 0)
    }

    #[inline(always)]
    pub fn read_address(&mut self) -> Result<Address, ProgramError> {
        let end = self.pos + 32;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let arr: [u8; 32] = self.data[self.pos..end].try_into().unwrap();
        self.pos = end;
        Ok(arr.into())
    }

    /// Skip `n` bytes without reading them.
    #[inline(always)]
    pub fn skip(&mut self, n: usize) -> Result<(), ProgramError> {
        let end = self.pos.checked_add(n).ok_or(ProgramError::AccountDataTooSmall)?;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn read_i8(&mut self) -> Result<i8, ProgramError> {
        Ok(self.read_u8()? as i8)
    }

    #[inline(always)]
    pub fn read_i16(&mut self) -> Result<i16, ProgramError> {
        let end = self.pos + 2;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = i16::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_i32(&mut self) -> Result<i32, ProgramError> {
        let end = self.pos + 4;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = i32::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_u128(&mut self) -> Result<u128, ProgramError> {
        let end = self.pos + 16;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = u128::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_i128(&mut self) -> Result<i128, ProgramError> {
        let end = self.pos + 16;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let val = i128::from_le_bytes(self.data[self.pos..end].try_into().unwrap());
        self.pos = end;
        Ok(val)
    }

    /// Return the remaining unread portion of the slice from the current position.
    #[inline(always)]
    pub fn data_from_position(&self) -> &'a [u8] {
        if self.pos >= self.data.len() {
            &[]
        } else {
            &self.data[self.pos..]
        }
    }

    /// Create a cursor for instruction data with minimum length validation.
    #[inline(always)]
    pub fn from_instruction(data: &'a [u8], min_len: usize) -> Result<Self, ProgramError> {
        if data.len() < min_len {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(Self { data, pos: 0 })
    }
}

// ── DataWriter ───────────────────────────────────────────────────────────────

/// Zero-copy write cursor over a mutable byte slice.
///
/// Position-tracked and bounds-checked. Use this when initializing a new
/// account's data layout.
///
/// ```rust,ignore
/// let mut raw = account.try_borrow_mut()?;
/// let mut w = DataWriter::new(&mut *raw);
/// w.write_u8(MY_DISC)?;
/// w.write_u64(0)?;
/// w.write_address(&authority)?;
/// ```
pub struct DataWriter<'a> {
    data: &'a mut [u8],
    pos: usize,
}

impl<'a> DataWriter<'a> {
    #[inline(always)]
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Number of bytes written so far.
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.pos
    }

    #[inline(always)]
    pub fn write_u8(&mut self, val: u8) -> Result<(), ProgramError> {
        if self.pos >= self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos] = val;
        self.pos += 1;
        Ok(())
    }

    #[inline(always)]
    pub fn write_u16(&mut self, val: u16) -> Result<(), ProgramError> {
        let end = self.pos + 2;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn write_u32(&mut self, val: u32) -> Result<(), ProgramError> {
        let end = self.pos + 4;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn write_u64(&mut self, val: u64) -> Result<(), ProgramError> {
        let end = self.pos + 8;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn write_i64(&mut self, val: i64) -> Result<(), ProgramError> {
        let end = self.pos + 8;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }

    /// Writes `1u8` for `true`, `0u8` for `false`.
    #[inline(always)]
    pub fn write_bool(&mut self, val: bool) -> Result<(), ProgramError> {
        self.write_u8(val as u8)
    }

    #[inline(always)]
    pub fn write_address(&mut self, addr: &Address) -> Result<(), ProgramError> {
        let end = self.pos + 32;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(addr.as_array());
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn write_i8(&mut self, val: i8) -> Result<(), ProgramError> {
        self.write_u8(val as u8)
    }

    #[inline(always)]
    pub fn write_i16(&mut self, val: i16) -> Result<(), ProgramError> {
        let end = self.pos + 2;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn write_i32(&mut self, val: i32) -> Result<(), ProgramError> {
        let end = self.pos + 4;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn write_u128(&mut self, val: u128) -> Result<(), ProgramError> {
        let end = self.pos + 16;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }

    #[inline(always)]
    pub fn write_i128(&mut self, val: i128) -> Result<(), ProgramError> {
        let end = self.pos + 16;
        if end > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[self.pos..end].copy_from_slice(&val.to_le_bytes());
        self.pos = end;
        Ok(())
    }
}

// ── Init helpers ─────────────────────────────────────────────────────────────

/// Zero-fill `data` before writing any fields.
///
/// Call this immediately after allocating account space (via system program
/// CPI) and before writing the discriminator or any other fields.
#[inline(always)]
pub fn zero_init(data: &mut [u8]) {
    data.fill(0);
}

/// Write a discriminator byte to `data[0]`.
#[inline(always)]
pub fn write_discriminator(data: &mut [u8], discriminator: u8) -> Result<(), ProgramError> {
    if data.is_empty() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[0] = discriminator;
    Ok(())
}
