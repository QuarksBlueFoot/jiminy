//! Zero-copy account header convention.
//!
//! Defines the canonical 8-byte header for Jiminy account layouts:
//!
//! ```text
//! ┌────────────┬─────────┬───────┬──────────────┐
//! │ disc (1B)  │ ver (1B)│ flags │  reserved    │
//! │   u8       │   u8    │ u16   │  [u8; 4]     │
//! └────────────┴─────────┴───────┴──────────────┘
//! ```
//!
//! Programs that adopt this header can use a single [`check_header`] call
//! to validate discriminator + version in one shot, and [`header_payload`]
//! to get the body slice after the header.

use pinocchio::error::ProgramError;

/// Canonical account header size in bytes.
pub const HEADER_LEN: usize = 8;

/// The canonical Jiminy account header.
///
/// All fields are little-endian on wire. The struct is `#[repr(C)]` so
/// its layout matches the on-chain byte representation exactly.
///
/// ```rust,ignore
/// let hdr = AccountHeader {
///     discriminator: 1,
///     version: 1,
///     flags: 0,
///     reserved: [0; 4],
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AccountHeader {
    pub discriminator: u8,
    pub version: u8,
    pub flags: u16,
    pub reserved: [u8; 4],
}

impl AccountHeader {
    /// Read an `AccountHeader` from the first 8 bytes of `data`.
    #[inline(always)]
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < HEADER_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        // SAFETY: AccountHeader is repr(C), Copy, and 8 bytes.  We checked length.
        // All bit patterns are valid for u8/u16/[u8;4].
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }

    /// Get a mutable reference to the header in `data`.
    #[inline(always)]
    pub fn from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < HEADER_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }
}

/// Write a full header (discriminator + version, flags = 0, reserved = 0).
#[inline(always)]
pub fn write_header(data: &mut [u8], discriminator: u8, version: u8) -> Result<(), ProgramError> {
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
    Ok(())
}

/// Write header with a data-length field in the reserved bytes.
///
/// Stores `data_len` as a little-endian u32 at bytes 4..8.
#[inline(always)]
pub fn write_header_with_len(
    data: &mut [u8],
    discriminator: u8,
    version: u8,
    data_len: u32,
) -> Result<(), ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[0] = discriminator;
    data[1] = version;
    data[2] = 0;
    data[3] = 0;
    let len_bytes = data_len.to_le_bytes();
    data[4] = len_bytes[0];
    data[5] = len_bytes[1];
    data[6] = len_bytes[2];
    data[7] = len_bytes[3];
    Ok(())
}

/// Validate discriminator and minimum version in one call.
#[inline(always)]
pub fn check_header(
    data: &[u8],
    expected_discriminator: u8,
    min_version: u8,
) -> Result<(), ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[0] != expected_discriminator {
        return Err(ProgramError::InvalidAccountData);
    }
    if data[1] < min_version {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Read the version byte from account data.
#[inline(always)]
pub fn read_version(data: &[u8]) -> Result<u8, ProgramError> {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(data[1])
}

/// Read the flags field (bytes 2..4) as u16 LE.
#[inline(always)]
pub fn read_header_flags(data: &[u8]) -> Result<u16, ProgramError> {
    if data.len() < 4 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(u16::from_le_bytes([data[2], data[3]]))
}

/// Read the data-length field (bytes 4..8) as u32 LE.
#[inline(always)]
pub fn read_data_len(data: &[u8]) -> Result<u32, ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(u32::from_le_bytes([data[4], data[5], data[6], data[7]]))
}

/// Return the body slice after the 8-byte header.
#[inline(always)]
pub fn header_payload(data: &[u8]) -> Result<&[u8], ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(&data[HEADER_LEN..])
}

/// Return the mutable body slice after the 8-byte header.
#[inline(always)]
pub fn header_payload_mut(data: &mut [u8]) -> Result<&mut [u8], ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(&mut data[HEADER_LEN..])
}

/// Return the body slice (alias: everything after the header).
#[inline(always)]
pub fn body(data: &[u8]) -> Result<&[u8], ProgramError> {
    header_payload(data)
}

/// Return the mutable body slice.
#[inline(always)]
pub fn body_mut(data: &mut [u8]) -> Result<&mut [u8], ProgramError> {
    header_payload_mut(data)
}
