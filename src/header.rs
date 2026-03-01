use pinocchio::error::ProgramError;

/// Size of the Jiminy standard account header, in bytes.
///
/// Layout:
/// - Byte 0: `u8` discriminator (account type tag)
/// - Byte 1: `u8` version (schema version - bump when layout changes)
/// - Byte 2: `u8` flags (application-defined bitfield)
/// - Byte 3: `u8` reserved (must be zero)
/// - Bytes 4-7: `u32` data_len (optional; payload length for variable-size accounts)
///
/// Total: 8 bytes, naturally aligned.
pub const HEADER_LEN: usize = 8;

/// Write the standard 8-byte Jiminy account header.
///
/// Sets the discriminator, version, and flags. Reserved byte and `data_len`
/// are zeroed. Call this immediately after allocating the account (and after
/// `zero_init` if desired).
///
/// ```rust,ignore
/// let mut raw = account.try_borrow_mut()?;
/// jiminy::write_header(&mut raw, VAULT_DISC, 1, 0)?;
/// let mut w = DataWriter::new(&mut raw[HEADER_LEN..]);
/// w.write_u64(0)?;              // balance
/// w.write_address(&authority)?;  // authority pubkey
/// ```
#[inline(always)]
pub fn write_header(
    data: &mut [u8],
    discriminator: u8,
    version: u8,
    flags: u8,
) -> Result<(), ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[0] = discriminator;
    data[1] = version;
    data[2] = flags;
    data[3] = 0; // reserved
    // data_len = 0 (bytes 4-7)
    data[4..8].copy_from_slice(&0u32.to_le_bytes());
    Ok(())
}

/// Write the full 8-byte header including an explicit `data_len` value.
///
/// Use this when initializing variable-length accounts where the payload
/// size is not implied by the fixed account allocation.
#[inline(always)]
pub fn write_header_with_len(
    data: &mut [u8],
    discriminator: u8,
    version: u8,
    flags: u8,
    data_len: u32,
) -> Result<(), ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[0] = discriminator;
    data[1] = version;
    data[2] = flags;
    data[3] = 0; // reserved
    data[4..8].copy_from_slice(&data_len.to_le_bytes());
    Ok(())
}

/// Validate the discriminator and minimum version of an account header.
///
/// Returns `InvalidAccountData` if the discriminator doesn't match or
/// the stored version is below `min_version`.
#[inline(always)]
pub fn check_header(
    data: &[u8],
    expected_disc: u8,
    min_version: u8,
) -> Result<(), ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[0] != expected_disc {
        return Err(ProgramError::InvalidAccountData);
    }
    if data[1] < min_version {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Read the version byte from an account header.
#[inline(always)]
pub fn read_version(data: &[u8]) -> Result<u8, ProgramError> {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(data[1])
}

/// Read the flags byte from an account header.
#[inline(always)]
pub fn read_header_flags(data: &[u8]) -> Result<u8, ProgramError> {
    if data.len() < 3 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(data[2])
}

/// Read the `data_len` field (bytes 4-7) from an account header.
#[inline(always)]
pub fn read_data_len(data: &[u8]) -> Result<u32, ProgramError> {
    if data.len() < HEADER_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(u32::from_le_bytes(
        data[4..8].try_into().unwrap(),
    ))
}

/// Return the payload slice that follows the 8-byte header.
///
/// ```rust,ignore
/// let data = account.try_borrow()?;
/// check_header(&data, VAULT_DISC, 1)?;
/// let payload = header_payload(&data);
/// let mut cur = SliceCursor::new(payload);
/// ```
#[inline(always)]
pub fn header_payload(data: &[u8]) -> &[u8] {
    if data.len() <= HEADER_LEN {
        &[]
    } else {
        &data[HEADER_LEN..]
    }
}

/// Return the mutable payload slice that follows the 8-byte header.
#[inline(always)]
pub fn header_payload_mut(data: &mut [u8]) -> &mut [u8] {
    if data.len() <= HEADER_LEN {
        &mut []
    } else {
        &mut data[HEADER_LEN..]
    }
}
