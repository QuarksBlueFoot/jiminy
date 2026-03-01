use pinocchio::error::ProgramError;

/// Read bit `n` from a byte. Returns `true` if the bit is set.
///
/// Bits are numbered LSB-first: bit 0 is `0x01`, bit 7 is `0x80`.
///
/// ```rust,ignore
/// let is_locked = read_bit(flags, 0);
/// let is_frozen = read_bit(flags, 1);
/// ```
#[inline(always)]
pub fn read_bit(byte: u8, n: u8) -> bool {
    (byte >> n) & 1 == 1
}

/// Set bit `n` in a byte, returning the modified value.
///
/// ```rust,ignore
/// flags = set_bit(flags, 0); // set the locked flag
/// ```
#[inline(always)]
pub fn set_bit(byte: u8, n: u8) -> u8 {
    byte | (1u8 << n)
}

/// Clear bit `n` in a byte, returning the modified value.
///
/// ```rust,ignore
/// flags = clear_bit(flags, 0); // clear the locked flag
/// ```
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
///
/// ```rust,ignore
/// const ACTIVE_AND_VERIFIED: u8 = 0b0000_0011;
/// if check_flags(state_flags, ACTIVE_AND_VERIFIED) { ... }
/// ```
#[inline(always)]
pub fn check_flags(byte: u8, mask: u8) -> bool {
    byte & mask == mask
}

/// Return `true` if ANY bit in `mask` is set in `byte`.
#[inline(always)]
pub fn check_any_flag(byte: u8, mask: u8) -> bool {
    byte & mask != 0
}

/// Read the `flags` byte from a data slice at `offset`, returning the
/// value. Bounds-checked: `AccountDataTooSmall` if offset is out of range.
///
/// Pairs with `SliceCursor::skip` when you want to jump to a flags byte
/// at a known position without reading all preceding fields.
#[inline(always)]
pub fn read_flags_at(data: &[u8], offset: usize) -> Result<u8, ProgramError> {
    data.get(offset)
        .copied()
        .ok_or(ProgramError::AccountDataTooSmall)
}

/// Write `value` to the flags byte at `offset` in a mutable data slice.
/// Bounds-checked: `AccountDataTooSmall` if offset is out of range.
#[inline(always)]
pub fn write_flags_at(data: &mut [u8], offset: usize, value: u8) -> Result<(), ProgramError> {
    let byte = data
        .get_mut(offset)
        .ok_or(ProgramError::AccountDataTooSmall)?;
    *byte = value;
    Ok(())
}
