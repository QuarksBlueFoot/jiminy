//! State machine transition checks.
//!
//! DeFi programs are state machines: orders (Open → Filled → Cancelled),
//! escrows (Pending → Released → Disputed), listings (Active → Sold → Expired).
//! These functions validate state values and enforce valid transitions.
//!
//! Unlike discriminators (which identify the account *type*), state checks
//! validate the account's *current phase* within its lifecycle.

use pinocchio::{error::ProgramError, ProgramResult};

/// Verify the state byte at `offset` in account data equals `expected`.
///
/// ```rust,ignore
/// let data = order_account.try_borrow()?;
/// check_state(&data, 2, ORDER_STATE_OPEN)?; // byte 2 is our state field
/// ```
#[inline(always)]
pub fn check_state(data: &[u8], offset: usize, expected: u8) -> ProgramResult {
    if offset >= data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[offset] != expected {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify the state byte at `offset` is NOT a specific rejected value.
///
/// Use this when multiple states are acceptable but one specific state
/// is invalid (e.g., "anything except Cancelled").
///
/// ```rust,ignore
/// let data = order_account.try_borrow()?;
/// check_state_not(&data, 2, ORDER_STATE_CANCELLED)?;
/// ```
#[inline(always)]
pub fn check_state_not(data: &[u8], offset: usize, rejected: u8) -> ProgramResult {
    if offset >= data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[offset] == rejected {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify the state byte at `offset` is one of the allowed values.
///
/// Use this when an instruction is valid in multiple states. For example,
/// a cancel instruction might be valid when state is Open OR Pending.
///
/// ```rust,ignore
/// let data = order_account.try_borrow()?;
/// check_state_in(&data, 2, &[ORDER_STATE_OPEN, ORDER_STATE_PENDING])?;
/// ```
#[inline(always)]
pub fn check_state_in(data: &[u8], offset: usize, allowed: &[u8]) -> ProgramResult {
    if offset >= data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let current = data[offset];
    let mut i = 0;
    while i < allowed.len() {
        if current == allowed[i] {
            return Ok(());
        }
        i += 1;
    }
    Err(ProgramError::InvalidAccountData)
}

/// Verify a state transition is valid given a table of allowed transitions.
///
/// Takes the current state, the desired next state, and a table of
/// `(from, to)` pairs representing valid transitions. Prevents invalid
/// transitions like Cancelled → Filled.
///
/// ```rust,ignore
/// const TRANSITIONS: &[(u8, u8)] = &[
///     (ORDER_OPEN, ORDER_FILLED),
///     (ORDER_OPEN, ORDER_CANCELLED),
///     (ORDER_FILLED, ORDER_SETTLED),
/// ];
/// check_state_transition(current_state, next_state, TRANSITIONS)?;
/// ```
#[inline(always)]
pub fn check_state_transition(
    current: u8,
    next: u8,
    valid_transitions: &[(u8, u8)],
) -> ProgramResult {
    let mut i = 0;
    while i < valid_transitions.len() {
        if valid_transitions[i].0 == current && valid_transitions[i].1 == next {
            return Ok(());
        }
        i += 1;
    }
    Err(ProgramError::InvalidAccountData)
}

/// Bounds-checked write of a new state byte at `offset`.
///
/// Use this after validating the transition to atomically update the state.
///
/// ```rust,ignore
/// let mut data = order_account.try_borrow_mut()?;
/// check_state(&data, 2, ORDER_STATE_OPEN)?;
/// check_state_transition(ORDER_STATE_OPEN, ORDER_STATE_FILLED, TRANSITIONS)?;
/// write_state(&mut data, 2, ORDER_STATE_FILLED)?;
/// ```
#[inline(always)]
pub fn write_state(data: &mut [u8], offset: usize, new_state: u8) -> ProgramResult {
    if offset >= data.len() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[offset] = new_state;
    Ok(())
}
