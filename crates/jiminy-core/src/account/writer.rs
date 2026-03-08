//! Zero-copy account writer with header awareness.

use pinocchio::{error::ProgramError, Address};

use super::header::HEADER_LEN;

/// Zero-copy account writer with header awareness.
///
/// Constructs from a borrowed `&mut [u8]`, writes the header, then
/// exposes sequential typed writes for the body.
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

    /// Write an i8 to the body at the current position.
    #[inline(always)]
    pub fn write_i8(&mut self, val: i8) -> Result<(), ProgramError> {
        self.write_u8(val as u8)
    }

    /// Write an i16 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_i16(&mut self, val: i16) -> Result<(), ProgramError> {
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

    /// Write an i32 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_i32(&mut self, val: i32) -> Result<(), ProgramError> {
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

    /// Write an i64 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_i64(&mut self, val: i64) -> Result<(), ProgramError> {
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

    /// Write a u128 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_u128(&mut self, val: u128) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + 16 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[abs..abs + 16].copy_from_slice(&val.to_le_bytes());
        self.body_pos += 16;
        Ok(())
    }

    /// Write an i128 (LE) to the body at the current position.
    #[inline(always)]
    pub fn write_i128(&mut self, val: i128) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + 16 > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[abs..abs + 16].copy_from_slice(&val.to_le_bytes());
        self.body_pos += 16;
        Ok(())
    }

    /// Write a variable-length byte slice to the body at the current position.
    #[inline(always)]
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + bytes.len() > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.data[abs..abs + bytes.len()].copy_from_slice(bytes);
        self.body_pos += bytes.len();
        Ok(())
    }

    /// Skip `n` bytes in the body without writing. Useful for padding.
    #[inline(always)]
    pub fn skip(&mut self, n: usize) -> Result<(), ProgramError> {
        let abs = self.body_offset + self.body_pos;
        if abs + n > self.data.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        self.body_pos += n;
        Ok(())
    }

    /// Current body write position.
    #[inline(always)]
    pub fn position(&self) -> usize {
        self.body_pos
    }
}
