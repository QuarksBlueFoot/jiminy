//! High-level zero-copy account IO.
//!
//! [`AccountReader`] and [`AccountWriter`] wrap the low-level cursor types
//! ([`SliceCursor`] / [`DataWriter`]) with header awareness, providing a
//! clean API for reading and writing account data in the canonical Jiminy
//! layout (8-byte header + body).
//!
//! # Reading
//!
//! ```rust,ignore
//! let data = account.try_borrow()?;
//! let reader = AccountReader::new(&data)?;
//! let hdr = reader.header();
//! let value = reader.body().read_u64()?;
//! let owner = reader.body().read_address()?;
//! ```
//!
//! # Writing
//!
//! ```rust,ignore
//! let mut data = account.try_borrow_mut()?;
//! let mut writer = AccountWriter::new(&mut data, MY_DISC, 1)?;
//! writer.body().write_u64(amount)?;
//! writer.body().write_address(owner)?;
//! ```

use pinocchio::{error::ProgramError, Address};

use crate::cursor::SliceCursor;
use crate::header::{AccountHeader, HEADER_LEN};

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
}

/// Zero-copy account writer with header awareness.
///
/// Constructs from a borrowed `&mut [u8]`, writes the header, then
/// exposes a [`DataWriter`] cursor for the body.
pub struct AccountWriter<'a> {
    data: &'a mut [u8],
    body_offset: usize,
    body_pos: usize,
}

impl<'a> AccountWriter<'a> {
    /// Create a new writer, initializing the header.
    ///
    /// Writes discriminator + version + zeroed flags/reserved.
    #[inline(always)]
    pub fn new(
        data: &'a mut [u8],
        discriminator: u8,
        version: u8,
    ) -> Result<Self, ProgramError> {
        if data.len() < HEADER_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[0] = discriminator;
        data[1] = version;
        data[2] = 0;
        data[3] = 0;
        data[4] = 0;
        data[5] = 0;
        data[6] = 0;
        data[7] = 0;
        Ok(Self {
            data,
            body_offset: HEADER_LEN,
            body_pos: 0,
        })
    }

    /// Create a writer over existing data without touching the header.
    #[inline(always)]
    pub fn from_existing(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        if data.len() < HEADER_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(Self {
            data,
            body_offset: HEADER_LEN,
            body_pos: 0,
        })
    }

    /// Set the flags field in the header.
    #[inline(always)]
    pub fn set_flags(&mut self, flags: u16) {
        let bytes = flags.to_le_bytes();
        self.data[2] = bytes[0];
        self.data[3] = bytes[1];
    }

    /// Write a u8 to the body at the current position.
    #[inline(always)]
    pub fn write_u8(&mut self, val: u8) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs >= self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[abs] = val;
        self.body_pos += 1;
        Ok(())
    }

    /// Write a u16 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_u16(&mut self, val: u16) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + 2 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let bytes = val.to_le_bytes();
        self.data[abs] = bytes[0];
        self.data[abs + 1] = bytes[1];
        self.body_pos += 2;
        Ok(())
    }

    /// Write a u32 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_u32(&mut self, val: u32) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + 4 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let bytes = val.to_le_bytes();
        self.data[abs] = bytes[0];
        self.data[abs + 1] = bytes[1];
        self.data[abs + 2] = bytes[2];
        self.data[abs + 3] = bytes[3];
        self.body_pos += 4;
        Ok(())
    }

    /// Write a u64 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_u64(&mut self, val: u64) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + 8 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let bytes = val.to_le_bytes();
        self.data[abs] = bytes[0];
        self.data[abs + 1] = bytes[1];
        self.data[abs + 2] = bytes[2];
        self.data[abs + 3] = bytes[3];
        self.data[abs + 4] = bytes[4];
        self.data[abs + 5] = bytes[5];
        self.data[abs + 6] = bytes[6];
        self.data[abs + 7] = bytes[7];
        self.body_pos += 8;
        Ok(())
    }

    /// Write a 32-byte address to the body at the current position.
    #[inline(always)]
    pub fn write_address(&mut self, addr: &Address) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + 32 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[abs..abs + 32].copy_from_slice(addr.as_ref());
        self.body_pos += 32;
        Ok(())
    }

    /// Write a bool to the body at the current position.
    #[inline(always)]
    pub fn write_bool(&mut self, val: bool) -> Result<(), ProgramError> {
        self.write_u8(val as u8)
    }

    /// Number of body bytes written so far.
    #[inline(always)]
    pub fn written(&self) -> usize {
        self.body_pos
    }

    /// Get the mutable body slice for direct manipulation.
    #[inline(always)]
    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.data[self.body_offset..]
    }

    /// Get the full data slice including header.
    #[inline(always)]
    pub fn raw(&self) -> &[u8] {
        self.data
    }
}
