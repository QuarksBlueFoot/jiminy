//! Zero-copy account reader with header awareness.

use pinocchio::error::ProgramError;

use super::cursor::SliceCursor;
use super::header::{AccountHeader, HEADER_LEN};

/// Zero-copy account reader with header awareness.
///
/// Constructs from a borrowed `&[u8]`, validates the header, and exposes
/// a cursor over the body for sequential field reads.
pub struct AccountReader<'a> {
    data: &'a [u8],
    body_offset: usize,
}

impl<'a> AccountReader<'a> {
    /// Create a new reader, validating that the data is at least `HEADER_LEN` bytes.
    #[inline(always)]
    pub fn new(data: &'a [u8]) -> Result<Self, ProgramError> {
        if data.len() < HEADER_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            data,
            body_offset: HEADER_LEN,
        })
    }

    /// Create a reader and validate discriminator + minimum version.
    #[inline(always)]
    pub fn new_checked(
        data: &'a [u8],
        expected_disc: u8,
        min_version: u8,
    ) -> Result<Self, ProgramError> {
        if data.len() < HEADER_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[0] != expected_disc {
            return Err(ProgramError::InvalidAccountData);
        }
        if data[1] < min_version {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self {
            data,
            body_offset: HEADER_LEN,
        })
    }

    /// Read the header as a typed reference.
    #[inline(always)]
    pub fn header(&self) -> &AccountHeader {
        // SAFETY: We checked length in new().
        unsafe { &*(self.data.as_ptr() as *const AccountHeader) }
    }

    /// Get the discriminator byte.
    #[inline(always)]
    pub fn discriminator(&self) -> u8 {
        self.data[0]
    }

    /// Get the version byte.
    #[inline(always)]
    pub fn version(&self) -> u8 {
        self.data[1]
    }

    /// Get the flags field.
    #[inline(always)]
    pub fn flags(&self) -> u16 {
        u16::from_le_bytes([self.data[2], self.data[3]])
    }

    /// Get a cursor positioned at the start of the body (after the header).
    #[inline(always)]
    pub fn body(&self) -> SliceCursor<'a> {
        SliceCursor::new(&self.data[self.body_offset..])
    }

    /// Get the raw body bytes.
    #[inline(always)]
    pub fn body_bytes(&self) -> &'a [u8] {
        &self.data[self.body_offset..]
    }

    /// Get the full raw data including header.
    #[inline(always)]
    pub fn raw(&self) -> &'a [u8] {
        self.data
    }

    /// Read a pubkey from the body at a given byte offset.
    #[inline(always)]
    pub fn pubkey_at(&self, offset: usize) -> Result<&'a [u8; 32], ProgramError> {
        let abs = self.body_offset + offset;
        if abs + 32 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        // SAFETY: bounds checked above.
        Ok(unsafe { &*(self.data.as_ptr().add(abs) as *const [u8; 32]) })
    }

    /// Read a u64 from the body at a given byte offset.
    #[inline(always)]
    pub fn u64_at(&self, offset: usize) -> Result<u64, ProgramError> {
        let abs = self.body_offset + offset;
        if abs + 8 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(u64::from_le_bytes([
            self.data[abs],
            self.data[abs + 1],
            self.data[abs + 2],
            self.data[abs + 3],
            self.data[abs + 4],
            self.data[abs + 5],
            self.data[abs + 6],
            self.data[abs + 7],
        ]))
    }

    /// Read a u32 from the body at a given byte offset.
    #[inline(always)]
    pub fn u32_at(&self, offset: usize) -> Result<u32, ProgramError> {
        let abs = self.body_offset + offset;
        if abs + 4 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(u32::from_le_bytes([
            self.data[abs],
            self.data[abs + 1],
            self.data[abs + 2],
            self.data[abs + 3],
        ]))
    }

    /// Read a u16 from the body at a given byte offset.
    #[inline(always)]
    pub fn u16_at(&self, offset: usize) -> Result<u16, ProgramError> {
        let abs = self.body_offset + offset;
        if abs + 2 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(u16::from_le_bytes([self.data[abs], self.data[abs + 1]]))
    }

    /// Read a u8 from the body at a given byte offset.
    #[inline(always)]
    pub fn u8_at(&self, offset: usize) -> Result<u8, ProgramError> {
        let abs = self.body_offset + offset;
        if abs >= self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(self.data[abs])
    }

    /// Read an i64 from the body at a given byte offset.
    #[inline(always)]
    pub fn i64_at(&self, offset: usize) -> Result<i64, ProgramError> {
        let abs = self.body_offset + offset;
        if abs + 8 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(i64::from_le_bytes([
            self.data[abs],
            self.data[abs + 1],
            self.data[abs + 2],
            self.data[abs + 3],
            self.data[abs + 4],
            self.data[abs + 5],
            self.data[abs + 6],
            self.data[abs + 7],
        ]))
    }

    /// Read a bool from the body at a given byte offset.
    #[inline(always)]
    pub fn bool_at(&self, offset: usize) -> Result<bool, ProgramError> {
        Ok(self.u8_at(offset)? != 0)
    }

    /// Read a fixed-size byte array from the body at a given byte offset.
    #[inline(always)]
    pub fn bytes_at<const N: usize>(&self, offset: usize) -> Result<[u8; N], ProgramError> {
        let abs = self.body_offset + offset;
        if abs + N > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.data[abs..abs + N]);
        Ok(out)
    }

    /// Number of body bytes remaining after the header.
    #[inline(always)]
    pub fn body_len(&self) -> usize {
        self.data.len().saturating_sub(self.body_offset)
    }
}
