//! Transaction composition guards.
//!
//! Higher-level introspection on top of `crate::introspect`. Instead of
//! reading raw instruction bytes, these helpers answer questions like
//! "is any other instruction in this tx from program X?" and "does this
//! tx look like a flash-loan sandwich?".
//!
//! All functions take a borrowed `&[u8]` from the Sysvar Instructions
//! account and the current instruction index.

use pinocchio::{error::ProgramError, Address, ProgramResult};

use crate::cpi_guard::get_num_instructions;
use crate::introspect::read_program_id_at;


/// Fail if any instruction *other than* `current_index` invokes `program_id`.
///
/// Prevents composability attacks where an attacker brackets your
/// instruction with calls to a specific program (flash-loan borrow/repay,
/// oracle manipulation, etc.).
///
/// ```rust,ignore
/// let data = sysvar_ix.try_borrow()?;
/// let me = cpi_guard::get_instruction_index(&data)?;
/// check_no_other_invocation(&data, me, &FLASH_LENDER)?;
/// ```
#[inline(always)]
pub fn check_no_other_invocation(
    sysvar_data: &[u8],
    current_index: u16,
    program_id: &Address,
) -> ProgramResult {
    let count = get_num_instructions(sysvar_data)?;
    let mut i = 0u16;
    while i < count {
        if i != current_index {
            if let Ok(pid) = read_program_id_at(sysvar_data, i) {
                if pid == *program_id {
                    return Err(ProgramError::InvalidArgument);
                }
            }
        }
        i += 1;
    }
    Ok(())
}

/// Fail if any instruction *after* `current_index` invokes `program_id`.
///
/// Weaker than [`check_no_other_invocation`] - the program can appear
/// before yours, just not after. Useful for ensuring an oracle read
/// precedes a trade, not the other way around.
///
/// ```rust,ignore
/// check_no_subsequent_invocation(&data, me, &oracle_program)?;
/// ```
#[inline(always)]
pub fn check_no_subsequent_invocation(
    sysvar_data: &[u8],
    current_index: u16,
    program_id: &Address,
) -> ProgramResult {
    let count = get_num_instructions(sysvar_data)?;
    let mut i = current_index.saturating_add(1);
    while i < count {
        if let Ok(pid) = read_program_id_at(sysvar_data, i) {
            if pid == *program_id {
                return Err(ProgramError::InvalidArgument);
            }
        }
        i += 1;
    }
    Ok(())
}

/// Return `true` if the transaction has instructions from `lender_program`
/// both before *and* after `current_index` (a flash-loan bracket pattern).
///
/// ```rust,ignore
/// if detect_flash_loan_bracket(&data, me, &FLASH_LENDER)? {
///     return Err(MyError::FlashLoanNotAllowed.into());
/// }
/// ```
#[inline(always)]
pub fn detect_flash_loan_bracket(
    sysvar_data: &[u8],
    current_index: u16,
    lender_program: &Address,
) -> Result<bool, ProgramError> {
    let count = get_num_instructions(sysvar_data)?;
    let mut before = false;
    let mut after = false;
    let mut i = 0u16;
    while i < count {
        if i != current_index {
            if let Ok(pid) = read_program_id_at(sysvar_data, i) {
                if pid == *lender_program {
                    if i < current_index {
                        before = true;
                    } else {
                        after = true;
                    }
                    if before && after {
                        return Ok(true);
                    }
                }
            }
        }
        i += 1;
    }
    Ok(before && after)
}

/// Count how many instructions in the transaction invoke `program_id`.
///
/// Detect multi-call patterns, rate-limit per-tx calls to your own
/// program, or require exactly one oracle update per tx.
///
/// ```rust,ignore
/// let n = count_program_invocations(&data, &my_program_id)?;
/// require!(n == 1, ProgramError::InvalidArgument);
/// ```
#[inline(always)]
pub fn count_program_invocations(
    sysvar_data: &[u8],
    program_id: &Address,
) -> Result<u16, ProgramError> {
    let count = get_num_instructions(sysvar_data)?;
    let mut n = 0u16;
    let mut i = 0u16;
    while i < count {
        if let Ok(pid) = read_program_id_at(sysvar_data, i) {
            if pid == *program_id {
                n += 1;
            }
        }
        i += 1;
    }
    Ok(n)
}
