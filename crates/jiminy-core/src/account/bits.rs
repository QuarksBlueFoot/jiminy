//! Bit-level helpers for account flags and bitmask fields.

use pinocchio::error::ProgramError;

/// Read bit `n` from a byte. Returns `true` if the bit is set.
///
/// Bits are numbered LSB-first: bit 0 is `0x01`, bit 7 is `0x80`.
#[inline(always)]
pub fn read_bit(byte: u8, n: u8) -> bool {
    (byte >> n) & 1 == 1
}

/// Set bit `n` in a byte, returning the modified value.
#[inline(always)]
pub fn set_bit(byte: u8, n: u8) -> u8 {
    byte | (1u8 << n)
}

/// Clear bit `n` in a byte, returning the modified value.
#[inline(always)]
pub fn clear_bit(byte: u8, n: u8) -> u8 {
    byte & !(1u8 << n)
}

/// Toggle bit `n` in a byte, returning the modified value.
#[inline(always)]
pub fn toggle_bit(byte: u8, n: u8) -> u8 {
    byte ^ (1u8 << n)
}

/// Return `true` if ALL bits in `mask` are set in `byte`.
#[inline(always)]
pub fn check_flags(byte: u8, mask: u8) -> bool {
    byte & mask == mask
}

/// Return `true` if ANY bit in `mask` is set in `byte`.
#[inline(always)]
pub fn check_any_flag(byte: u8, mask: u8) -> bool {
    byte & mask != 0
}

/// Read the `flags` byte from a data slice at `offset`.
#[inline(always)]
pub fn read_flags_at(data: &[u8], offset: usize) -> Result<u8, ProgramError> {
    data.get(offset)
        .copied()
        .ok_or(ProgramError::AccountDataTooSmall)
}

/// Write `value` to the flags byte at `offset` in a mutable data slice.
#[inline(always)]
pub fn write_flags_at(data: &mut [u8], offset: usize, value: u8) -> Result<(), ProgramError> {
    let byte = data
        .get_mut(offset)
        .ok_or(ProgramError::AccountDataTooSmall)?;
    *byte = value;
    Ok(())
}
