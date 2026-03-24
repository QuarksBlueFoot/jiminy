//! Zero-copy account header convention.
//!
//! Defines the canonical 16-byte header for Jiminy account layouts:
//!
//! ```text
//! ┌────────────┬─────────┬───────┬────────────────────┬──────────────┐
//! │ disc (1B)  │ ver (1B)│ flags │  layout_id         │  reserved    │
//! │   u8       │   u8    │ u16   │  [u8; 8]           │  [u8; 4]     │
//! └────────────┴─────────┴───────┴────────────────────┴──────────────┘
//! ```
//!
//! Programs that adopt this header can use a single [`check_header`] call
//! to validate discriminator + version + layout_id in one shot, and
//! [`header_payload`] to get the body slice after the header.

use pinocchio::error::ProgramError;

/// Canonical account header size in bytes.
pub const HEADER_LEN: usize = 16;

/// The canonical Jiminy account header.
///
/// All fields are little-endian on wire. The struct is `#[repr(C)]` so
/// its layout matches the on-chain byte representation exactly.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AccountHeader {
    pub discriminator: u8,
    pub version: u8,
    pub flags: u16,
    pub layout_id: [u8; 8],
    pub reserved: [u8; 4],
}

impl AccountHeader {
    /// Read an `AccountHeader` from the first 16 bytes of `data`.
    #[inline(always)]
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < HEADER_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        // SAFETY: AccountHeader is repr(C), Copy, and 16 bytes. We checked length.
        // All bit patterns are valid for u8/u16/[u8;8]/[u8;4].
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

/// Write a full header (discriminator + version + layout_id, flags = 0, reserved = 0).
#[inline(always)]
pub fn write_header(
    data: &mut [u8],
    discriminator: u8,
    version: u8,
    layout_id: &[u8; 8],
) -> Result<(), ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[0] = discriminator;
    data[1] = version;
    // flags = 0
    data[2] = 0;
    data[3] = 0;
    // layout_id
    data[4] = layout_id[0];
    data[5] = layout_id[1];
    data[6] = layout_id[2];
    data[7] = layout_id[3];
    data[8] = layout_id[4];
    data[9] = layout_id[5];
    data[10] = layout_id[6];
    data[11] = layout_id[7];
    // reserved = 0
    data[12] = 0;
    data[13] = 0;
    data[14] = 0;
    data[15] = 0;
    Ok(())
}

/// Validate discriminator, minimum version, and layout_id in one call.
#[inline(always)]
pub fn check_header(
    data: &[u8],
    expected_discriminator: u8,
    min_version: u8,
    layout_id: &[u8; 8],
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
    if data[4..12] != *layout_id {
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

/// Read the layout_id field (bytes 4..12).
#[inline(always)]
pub fn read_layout_id(data: &[u8]) -> Result<[u8; 8], ProgramError> {
    if data.len() < 12 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let mut id = [0u8; 8];
    id.copy_from_slice(&data[4..12]);
    Ok(id)
}

/// Validate only the layout_id at bytes 4..12.
#[inline(always)]
pub fn check_layout_id(data: &[u8], expected: &[u8; 8]) -> Result<(), ProgramError> {
    if data.len() < 12 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[4..12] != *expected {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Return the body slice after the 16-byte header.
#[inline(always)]
pub fn header_payload(data: &[u8]) -> Result<&[u8], ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(&data[HEADER_LEN..])
}

/// Return the mutable body slice after the 16-byte header.
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

// ── Pod / FixedLayout ────────────────────────────────────────────────────────

// SAFETY: AccountHeader is #[repr(C)], Copy, 16 bytes, and all bit patterns
// are valid for u8 / u16 / [u8; 8] / [u8; 4].
unsafe impl super::pod::Pod for AccountHeader {}

impl super::pod::FixedLayout for AccountHeader {
    const SIZE: usize = HEADER_LEN;
}
