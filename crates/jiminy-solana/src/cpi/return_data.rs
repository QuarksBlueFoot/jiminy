//! CPI return data reader.
//!
//! Wraps the `sol_get_return_data` syscall so callers can read values
//! returned by a CPI target (swap output amounts, oracle prices, etc.)
//! and verify they came from the expected program.

use pinocchio::{error::ProgramError, Address};

extern "C" {
    /// BPF syscall: read return data set by a CPI callee.
    ///
    /// Writes `program_id` (32 bytes) into `program_id_out` and the
    /// return data into `buf[..min(length, actual_len)]`.
    /// Returns the actual data length, or 0 if no return data.
    fn sol_get_return_data(buf: *mut u8, length: u64, program_id_out: *mut u8) -> u64;
}

/// Maximum return data the runtime supports (1024 bytes).
pub const MAX_RETURN_DATA: usize = 1024;

/// Read CPI return data into `buf` and return `(program_id, bytes_written)`.
///
/// Returns `InvalidAccountData` when no return data is present.
///
/// ```rust,ignore
/// let mut buf = [0u8; 64];
/// let (who, len) = read_return_data(&mut buf)?;
/// let amount = u64::from_le_bytes(buf[..8].try_into().unwrap());
/// ```
#[inline(always)]
pub fn read_return_data(buf: &mut [u8]) -> Result<(Address, usize), ProgramError> {
    let mut program_id = [0u8; 32];
    let actual_len = unsafe {
        sol_get_return_data(buf.as_mut_ptr(), buf.len() as u64, program_id.as_mut_ptr())
    };
    if actual_len == 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    let written = (actual_len as usize).min(buf.len());
    Ok((Address::new_from_array(program_id), written))
}

/// Read return data and verify it came from `expected_program`.
///
/// Prevents a malicious intermediate CPI from overwriting the return value
/// you expected from a specific program.
///
/// ```rust,ignore
/// let mut buf = [0u8; 32];
/// let len = read_return_data_from(&mut buf, &SWAP_ROUTER)?;
/// ```
#[inline(always)]
pub fn read_return_data_from(
    buf: &mut [u8],
    expected_program: &Address,
) -> Result<usize, ProgramError> {
    let (program, len) = read_return_data(buf)?;
    if program != *expected_program {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(len)
}

/// Read a single `u64` from CPI return data, verified from `expected_program`.
///
/// Covers the most common case: reading a swap output amount, a price,
/// or any single u64 returned by a CPI call.
///
/// ```rust,ignore
/// let output_amount = read_return_u64(&SWAP_PROGRAM)?;
/// ```
#[inline(always)]
pub fn read_return_u64(expected_program: &Address) -> Result<u64, ProgramError> {
    let mut buf = [0u8; 8];
    let len = read_return_data_from(&mut buf, expected_program)?;
    if len < 8 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(u64::from_le_bytes(buf))
}
