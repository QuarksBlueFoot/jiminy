//! Transaction introspection via Sysvar Instructions.
//!
//! Goes beyond the CPI guard. Read any instruction in the current
//! transaction: program IDs, instruction data, account keys. Verify
//! the transaction shape before touching any state.
//!
//! Use cases:
//! - Verify a Compute Budget instruction exists (frontrun protection)
//! - Read instruction data at a known index (Ed25519 sig verification)
//! - Walk the full transaction to enforce ordering constraints
//!
//! All reads are zero-copy from the sysvar account data. No alloc.
//!
//! ## Sysvar Instructions layout (for reference)
//!
//! ```text
//! [num_instructions: u16 LE]
//! [offset_0: u16 LE] [offset_1: u16 LE] ... [offset_N-1: u16 LE]
//!
//! At each offset:
//!   [num_accounts: u16 LE]
//!   [accounts: 33 bytes each (flags:u8 + pubkey:32)]
//!   [program_id: 32 bytes]
//!   [data_len: u16 LE]
//!   [data: data_len bytes]
//!
//! At end of sysvar:
//!   [current_instruction_index: u16 LE]
//! ```

use pinocchio::{error::ProgramError, Address};

/// Read the program_id of the instruction at `index` from raw sysvar data.
///
/// Returns a 32-byte address. This is the public, reusable version of the
/// internal helper in `cpi_guard`.
///
/// ```rust,ignore
/// let data = sysvar_ix.try_borrow()?;
/// let prog = read_program_id_at(&data, 0)?;
/// if prog == programs::COMPUTE_BUDGET { ... }
/// ```
#[inline(always)]
pub fn read_program_id_at(data: &[u8], index: u16) -> Result<Address, ProgramError> {
    let (offset, _num_accounts) = instruction_meta(data, index)?;
    let num_accounts = _num_accounts as usize;
    let program_id_offset = offset + 2 + num_accounts * 33;
    if program_id_offset + 32 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&data[program_id_offset..program_id_offset + 32]);
    Ok(Address::new_from_array(out))
}

/// Read the instruction data slice at `index` from raw sysvar data.
///
/// Returns the byte offset and length of the instruction data within the
/// sysvar data buffer. Callers can then slice directly into the borrowed
/// sysvar data without any copies.
///
/// ```rust,ignore
/// let data = sysvar_ix.try_borrow()?;
/// let (offset, len) = read_instruction_data_range(&data, 0)?;
/// let ix_data = &data[offset..offset + len];
/// ```
#[inline(always)]
pub fn read_instruction_data_range(
    data: &[u8],
    index: u16,
) -> Result<(usize, usize), ProgramError> {
    let (offset, num_accounts) = instruction_meta(data, index)?;
    let num_accounts = num_accounts as usize;
    // program_id is after accounts
    let after_program_id = offset + 2 + num_accounts * 33 + 32;
    if after_program_id + 2 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let data_len =
        u16::from_le_bytes([data[after_program_id], data[after_program_id + 1]]) as usize;
    let data_start = after_program_id + 2;
    if data_start + data_len > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok((data_start, data_len))
}

/// Read the account pubkey at position `account_index` within instruction `index`.
///
/// ```rust,ignore
/// let data = sysvar_ix.try_borrow()?;
/// let signer_key = read_instruction_account_key(&data, 0, 1)?;
/// ```
#[inline(always)]
pub fn read_instruction_account_key(
    data: &[u8],
    index: u16,
    account_index: u16,
) -> Result<Address, ProgramError> {
    let (offset, num_accounts) = instruction_meta(data, index)?;
    if account_index >= num_accounts {
        return Err(ProgramError::InvalidArgument);
    }
    // Each account meta: 1 byte flags + 32 bytes pubkey
    let key_offset = offset + 2 + (account_index as usize) * 33 + 1;
    if key_offset + 32 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&data[key_offset..key_offset + 32]);
    Ok(Address::new_from_array(out))
}

/// Check that a Compute Budget instruction exists in the transaction.
///
/// Walks every instruction in the sysvar looking for one whose program_id
/// matches `ComputeBudget111...`. If the user didn't set a compute budget,
/// this returns an error - useful for enforcing priority fee inclusion or
/// frontrun protection.
///
/// ```rust,ignore
/// let data = sysvar_ix.try_borrow()?;
/// check_has_compute_budget(&data)?;
/// ```
#[cfg(feature = "programs")]
#[inline]
pub fn check_has_compute_budget(data: &[u8]) -> Result<(), ProgramError> {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let count = u16::from_le_bytes([data[0], data[1]]);
    let mut i = 0u16;
    while i < count {
        if let Ok(pid) = read_program_id_at(data, i) {
            if pid == jiminy_core::programs::COMPUTE_BUDGET {
                return Ok(());
            }
        }
        i += 1;
    }
    Err(ProgramError::InvalidArgument)
}

/// Internal: read instruction offset and num_accounts for instruction at `index`.
#[inline(always)]
fn instruction_meta(data: &[u8], index: u16) -> Result<(usize, u16), ProgramError> {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let num_instructions = u16::from_le_bytes([data[0], data[1]]);
    if index >= num_instructions {
        return Err(ProgramError::InvalidAccountData);
    }

    let offset_pos = 2 + (index as usize) * 2;
    if offset_pos + 2 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let instr_offset =
        u16::from_le_bytes([data[offset_pos], data[offset_pos + 1]]) as usize;

    if instr_offset + 2 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let num_accounts =
        u16::from_le_bytes([data[instr_offset], data[instr_offset + 1]]);

    Ok((instr_offset, num_accounts))
}
