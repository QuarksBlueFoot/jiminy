//! Program upgrade authority verification.
//!
//! Read the upgrade authority from a BPF Upgradeable Loader program
//! data account. Check if a program is immutable (frozen) or who
//! controls upgrades.
//!
//! ## ProgramData account layout
//!
//! The BPF Upgradeable Loader stores program state in a PDA with seeds
//! `[program_id]`. The serialized `UpgradeableLoaderState` for the
//! ProgramData variant is:
//!
//! ```text
//!   0..4    discriminator  (u32 LE, 3 = ProgramData)
//!   4..12   slot           (u64 LE, last deploy slot)
//!  12       option_tag     (u8, 0 = None / 1 = Some)
//!  13..45   authority      (32 bytes, present only if tag == 1)
//! ```

use hopper_runtime::{ProgramError, AccountView, Address, ProgramResult};

/// Discriminator for the ProgramData variant of UpgradeableLoaderState.
const PROGRAMDATA_DISC: u32 = 3;

/// Byte offset of the Option tag for upgrade_authority_address.
const AUTH_OPTION_OFFSET: usize = 12;

/// Byte offset of the authority pubkey (when present).
const AUTH_KEY_OFFSET: usize = 13;

/// Minimum length of a ProgramData account's data prefix.
const MIN_LEN: usize = 45;

/// Read the upgrade authority from a ProgramData account.
///
/// Returns `Some(address)` when the program is upgradeable, `None` when
/// frozen (authority revoked).
///
/// `program_data` is the PDA derived from
/// `find_program_address(&[program_id], &BPF_LOADER)`.
///
/// ```rust,ignore
/// match read_upgrade_authority(program_data)? {
///     Some(auth) => { /* upgradeable */ }
///     None       => { /* immutable   */ }
/// }
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn read_upgrade_authority(
    program_data: &AccountView,
) -> Result<Option<Address>, ProgramError> {
    if !program_data.owned_by(&jiminy_core::programs::BPF_LOADER) {
        return Err(ProgramError::IncorrectProgramId);
    }
    let data = program_data.try_borrow()?;
    if data.len() < MIN_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let disc = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if disc != PROGRAMDATA_DISC {
        return Err(ProgramError::InvalidAccountData);
    }
    if data[AUTH_OPTION_OFFSET] == 0 {
        return Ok(None);
    }
    let mut addr = [0u8; 32];
    addr.copy_from_slice(&data[AUTH_KEY_OFFSET..AUTH_KEY_OFFSET + 32]);
    Ok(Some(Address::new_from_array(addr)))
}

/// Verify a program is immutable (upgrade authority is `None`).
///
/// Use when your protocol integrates with an external program and needs
/// assurance it won't change after deployment.
///
/// ```rust,ignore
/// check_program_immutable(amm_program_data)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_program_immutable(program_data: &AccountView) -> ProgramResult {
    match read_upgrade_authority(program_data)? {
        Some(_) => Err(ProgramError::InvalidArgument),
        None => Ok(()),
    }
}

/// Verify a program's upgrade authority matches `expected`.
///
/// For governance-controlled programs: allow integration only if a
/// known DAO multisig controls upgrades.
///
/// ```rust,ignore
/// check_upgrade_authority(program_data, &dao_multisig)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_upgrade_authority(
    program_data: &AccountView,
    expected: &Address,
) -> ProgramResult {
    match read_upgrade_authority(program_data)? {
        Some(auth) if auth == *expected => Ok(()),
        _ => Err(ProgramError::InvalidArgument),
    }
}
