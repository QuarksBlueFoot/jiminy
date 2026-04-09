//! State machine transition checks.
//!
//! DeFi programs are state machines: orders (Open → Filled → Cancelled),
//! escrows (Pending → Released → Disputed), listings (Active → Sold → Expired).
//! These functions validate state values and enforce valid transitions.
//!
//! Unlike discriminators (which identify the account *type*), state checks
//! validate the account's *current phase* within its lifecycle.

use hopper_runtime::{ProgramError, ProgramResult};

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

// ══════════════════════════════════════════════════════════════════════
//  State hygiene helpers
// ══════════════════════════════════════════════════════════════════════

/// Zero all bytes in a mutable slice.
///
/// Use when closing accounts or reinitializing data regions.
#[inline(always)]
pub fn zero_bytes(data: &mut [u8]) {
    let mut i = 0;
    while i < data.len() {
        data[i] = 0;
        i += 1;
    }
}

/// Write a version byte at the standard Hopper header offset (byte 1).
#[inline(always)]
pub fn write_version(data: &mut [u8], version: u8) -> ProgramResult {
    if data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[1] = version;
    Ok(())
}

/// Write the discriminator byte at the standard Hopper header offset (byte 0).
#[inline(always)]
pub fn write_disc(data: &mut [u8], disc: u8) -> ProgramResult {
    if data.is_empty() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[0] = disc;
    Ok(())
}

/// Write the 8-byte layout_id at the standard Hopper header offset (bytes 4..12).
#[inline(always)]
pub fn write_layout_id(data: &mut [u8], layout_id: &[u8; 8]) -> ProgramResult {
    if data.len() < 12 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[4..12].copy_from_slice(layout_id);
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════
//  Extension region
// ══════════════════════════════════════════════════════════════════════

/// A reserved byte region for forward-compatible layout expansion.
///
/// Place at the end of a layout struct to claim bytes that future versions
/// can use without requiring realloc.
///
/// ```rust,ignore
/// #[repr(C)]
/// #[derive(Copy, Clone)]
/// pub struct MyLayout {
///     pub authority: [u8; 32],
///     pub balance: hopper_runtime::LeU64,
///     pub _reserved: ExtensionRegion<64>,
/// }
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ExtensionRegion<const N: usize> {
    pub bytes: [u8; N],
}

impl<const N: usize> Default for ExtensionRegion<N> {
    fn default() -> Self {
        Self { bytes: [0u8; N] }
    }
}

impl<const N: usize> ExtensionRegion<N> {
    /// Number of reserved bytes in this region.
    pub const SIZE: usize = N;

    /// Construct a zeroed extension region.
    #[inline(always)]
    pub const fn zeroed() -> Self {
        Self { bytes: [0u8; N] }
    }
}
