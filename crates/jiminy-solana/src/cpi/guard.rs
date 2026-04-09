//! CPI reentrancy protection via Sysvar Instructions introspection.
//!
//! These functions read the Sysvar Instructions account to determine
//! whether the current instruction is top-level (called directly by the
//! transaction) or invoked via CPI from another program.
//!
//! ## Why this matters
//!
//! Reentrancy-style attacks on Solana work by having a malicious program
//! invoke your instruction via CPI to exploit intermediate state. For
//! example:
//!
//! 1. Attacker's program calls your swap instruction via CPI
//! 2. Your instruction reads token balances and begins execution
//! 3. Attacker's program has already manipulated balances in a prior
//!    instruction in the same transaction
//!
//! By checking that your instruction is top-level (no CPI caller), you
//! prevent this class of attacks entirely.
//!
//! ## Sysvar Instructions data layout
//!
//! The Sysvar Instructions account has a special layout:
//! - Bytes [N*2 - 2 .. N*2]: current instruction index (u16 LE), where N
//!   is the total number of instructions
//! - The instruction data contains serialized instructions that can be
//!   introspected
//!
//! On-chain, the runtime provides the current instruction index at a
//! fixed offset from the end of the sysvar data. We use this to detect
//! CPI depth.
//!
//! ## Important
//!
//! The Sysvar Instructions account must be passed as an account to the
//! instruction for these checks to work. It cannot be fetched via
//! `get()` like Clock.

use hopper_runtime::{ProgramError, AccountView, Address, ProgramResult};

#[cfg(feature = "programs")]
use jiminy_core::programs;

/// Verify the account is the Sysvar Instructions account.
///
/// ```rust,ignore
/// check_sysvar_instructions(instructions_account)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_sysvar_instructions(account: &AccountView) -> ProgramResult {
    if *account.address() != programs::SYSVAR_INSTRUCTIONS {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Read the number of instructions serialized in the Sysvar Instructions data.
///
/// The first 2 bytes of the sysvar data contain the instruction count as u16 LE.
///
/// ```rust,ignore
/// let data = sysvar_instructions.try_borrow()?;
/// let count = get_num_instructions(&data)?;
/// ```
#[inline(always)]
pub fn get_num_instructions(data: &[u8]) -> Result<u16, ProgramError> {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

/// Read the current instruction index from the Sysvar Instructions data.
///
/// The current instruction index is stored as a u16 LE at offset
/// `data.len() - 2` (the last two bytes of the sysvar data).
///
/// ```rust,ignore
/// let data = sysvar_instructions.try_borrow()?;
/// let idx = get_instruction_index(&data)?;
/// ```
#[inline(always)]
pub fn get_instruction_index(data: &[u8]) -> Result<u16, ProgramError> {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let offset = data.len() - 2;
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

/// Verify this instruction was NOT invoked via CPI (it's a top-level instruction).
///
/// Reads the Sysvar Instructions account to inspect the instruction stack.
/// If the program detects that it was called via CPI (by another program),
/// it returns an error.
///
/// This is the primary reentrancy guard for Solana programs. Neither Anchor
/// nor Pinocchio provides this check.
///
/// ## How it works
///
/// On Solana, when a program is invoked via CPI, the runtime tracks the
/// invocation depth. We detect CPI by reading the serialized instruction
/// at the current index from the Sysvar Instructions data and checking
/// whether the program_id matches the expected program_id. If we can verify
/// the instruction at the current index is ours and is a top-level
/// instruction, we're safe.
///
/// The simpler approach used here: we check that the current instruction's
/// program_id_index points to our program. The serialized instruction format
/// stores each instruction as:
/// ```text
/// [num_accounts: u16] [account_metas...] [program_id_index: u8]
///   [data_len: u16] [data...]
/// ```
///
/// We extract the program_id for the current instruction index and verify
/// it matches the expected program.
///
/// ```rust,ignore
/// let sysvar_ix = accs.next_sysvar_instructions()?;
/// check_no_cpi_caller(sysvar_ix, program_id)?;
/// ```
#[cfg(feature = "programs")]
#[inline]
pub fn check_no_cpi_caller(
    sysvar_instructions: &AccountView,
    program_id: &Address,
) -> ProgramResult {
    check_sysvar_instructions(sysvar_instructions)?;
    let data = sysvar_instructions.try_borrow()?;
    if data.len() < 4 {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let num_instructions = get_num_instructions(&data)? as usize;
    let current_index = get_instruction_index(&data)? as usize;

    if current_index >= num_instructions {
        return Err(ProgramError::InvalidAccountData);
    }

    // Walk to the instruction at current_index to read its program_id.
    let program_id_key = read_instruction_program_id(&data, current_index)?;
    if program_id_key != *program_id {
        // The instruction at the current index is NOT our program,
        // we were invoked via CPI.
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

/// Verify the CPI caller is an expected trusted program.
///
/// For programs that ARE designed to be called via CPI, but only from
/// specific trusted callers. Reads the instruction stack to find the
/// outer instruction and verify its program_id matches.
///
/// ```rust,ignore
/// check_cpi_caller(sysvar_ix, &TRUSTED_ROUTER_PROGRAM)?;
/// ```
#[cfg(feature = "programs")]
#[inline]
pub fn check_cpi_caller(
    sysvar_instructions: &AccountView,
    expected_caller: &Address,
) -> ProgramResult {
    check_sysvar_instructions(sysvar_instructions)?;
    let data = sysvar_instructions.try_borrow()?;
    if data.len() < 4 {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let current_index = get_instruction_index(&data)? as usize;

    // Read the program_id of the current instruction.
    // If it matches expected_caller, we were called by the right program.
    let caller_id = read_instruction_program_id(&data, current_index)?;
    if caller_id != *expected_caller {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

/// Read the program_id of the instruction at the given index from serialized
/// Sysvar Instructions data.
///
/// ## Sysvar Instructions serialized layout
///
/// ```text
/// [num_instructions: u16 LE]
/// [offset_0: u16 LE]          // byte offset to instruction 0
/// [offset_1: u16 LE]          // byte offset to instruction 1
/// ...
/// [offset_N-1: u16 LE]
///
/// // At each offset, the instruction is serialized as:
/// [num_accounts: u16 LE]
/// for each account (33 bytes each):
///   [flags: u8]               // bit 0 = is_signer, bit 1 = is_writable
///   [pubkey: 32 bytes]
/// [program_id: 32 bytes]      // the instruction's program_id
/// [data_len: u16 LE]
/// [data: data_len bytes]
///
/// // At the very end of the sysvar data:
/// [current_instruction_index: u16 LE]
/// ```
fn read_instruction_program_id(
    data: &[u8],
    instruction_index: usize,
) -> Result<Address, ProgramError> {
    let num_instructions = get_num_instructions(data)? as usize;
    if instruction_index >= num_instructions {
        return Err(ProgramError::InvalidAccountData);
    }

    // Read the offset for the instruction at this index.
    // Offsets are stored starting at byte 2, each as u16 LE.
    let offset_pos = 2 + instruction_index * 2;
    if offset_pos + 2 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let instr_offset =
        u16::from_le_bytes([data[offset_pos], data[offset_pos + 1]]) as usize;

    // At the instruction offset, the layout is:
    //   [num_accounts: u16 LE]
    //   [accounts: (flags:u8 + pubkey:32) * num_accounts]  -- 33 bytes each
    //   [program_id: 32 bytes]
    //   [data_len: u16 LE]
    //   [data: data_len bytes]
    if instr_offset + 2 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let num_accounts =
        u16::from_le_bytes([data[instr_offset], data[instr_offset + 1]]) as usize;

    // program_id starts after: num_accounts (2 bytes) + accounts (33 bytes each)
    let program_id_offset = instr_offset + 2 + num_accounts * 33;
    if program_id_offset + 32 > data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let mut program_id_bytes = [0u8; 32];
    program_id_bytes.copy_from_slice(&data[program_id_offset..program_id_offset + 32]);
    Ok(Address::new_from_array(program_id_bytes))
}
