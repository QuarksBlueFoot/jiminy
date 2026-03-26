//! Tiered loading helpers for account views.
//!
//! The [`zero_copy_layout!`](crate::zero_copy_layout!) macro generates per-struct tiered loading
//! methods. This module provides the shared validation functions they
//! call.
//!
//! ## Trust Tiers
//!
//! | Tier | Name | Method | Validation | Use When |
//! |------|------|--------|------------|----------|
//! | 1 | **Verified** | `load()` / `load_mut()` | owner + disc + version + layout_id + exact size | Loading your own program's accounts |
//! | 2 | **Foreign Verified** | `load_foreign()` | owner + layout_id + exact size | Reading another program's accounts (cross-program) |
//! | 3 | **Compatibility** | `validate_version_compatible()` | owner + disc + version + min size (no layout_id) | Version migration, explicitly weaker |
//! | 4 | **Unsafe** | `load_unchecked()` | none (`unsafe`) | Hot path — caller assumes all risk |
//! | 5 | **Unverified Overlay** | `load_unverified_overlay()` | header + layout_id if present, fallback to overlay | Indexers, explorers, diagnostic tooling |
//!
//! Tiers 1–2 are the standard paths. Tier 3 is a migration helper that
//! trades `layout_id` verification for version-range flexibility. Tier 4
//! is `unsafe` by design - deliberate friction for unvalidated loads.
//! Tier 5 is for tooling that cannot guarantee account provenance.
//!
//! See [`SAFETY_MODEL.md`](https://github.com/QuarksBlueFoot/jiminy/blob/main/docs/SAFETY_MODEL.md)
//! for the full trust tier model and all safety invariants.

use pinocchio::{AccountView, Address};
use pinocchio::account::{Ref, RefMut};
use pinocchio::error::ProgramError;

use super::{HEADER_LEN, check_header, check_layout_id, pod_from_bytes, Pod, FixedLayout};

/// Validate owner + disc + version + layout_id + exact size on an `AccountView`.
///
/// This is the standard loading path (Tier 1). Returns the borrowed
/// account data on success, avoiding a second `try_borrow()` call.
///
/// # Errors
///
/// - `IllegalOwner`: account is not owned by `program_id`.
/// - `AccountDataTooSmall`: account data does not match `expected_size`.
/// - `InvalidAccountData`: discriminator, version, or layout_id mismatch.
#[inline(always)]
pub fn validate_account<'a>(
    account: &'a AccountView,
    program_id: &Address,
    disc: u8,
    version: u8,
    layout_id: &[u8; 8],
    expected_size: usize,
) -> Result<Ref<'a, [u8]>, ProgramError> {
    if !account.owned_by(program_id) {
        return Err(ProgramError::IllegalOwner);
    }

    let data = account.try_borrow()?;

    if data.len() != expected_size {
        return Err(ProgramError::AccountDataTooSmall);
    }

    check_header(&data, disc, version, layout_id)?;
    Ok(data)
}

/// Validate owner + layout_id + exact size on an `AccountView`.
///
/// Cross-program read path (Tier 2). Skips disc and version checks
/// because the foreign program may use different disc/version conventions.
/// Returns the borrowed account data on success.
///
/// # Errors
///
/// - `IllegalOwner`: account is not owned by `expected_owner`.
/// - `AccountDataTooSmall`: account data does not match `expected_size`.
/// - `InvalidAccountData`: `layout_id` does not match.
#[inline(always)]
pub fn validate_foreign<'a>(
    account: &'a AccountView,
    expected_owner: &Address,
    layout_id: &[u8; 8],
    expected_size: usize,
) -> Result<Ref<'a, [u8]>, ProgramError> {
    if !account.owned_by(expected_owner) {
        return Err(ProgramError::IllegalOwner);
    }

    let data = account.try_borrow()?;

    if data.len() != expected_size {
        return Err(ProgramError::AccountDataTooSmall);
    }

    check_layout_id(&data, layout_id)?;
    Ok(data)
}

/// **Tier 2 — Cross-program read for segmented accounts.**
///
/// Same as [`validate_foreign`] but uses minimum-size checking
/// instead of exact-size. Segmented accounts have variable length
/// depending on capacity, so exact-size matching is not possible.
///
/// Checks:
/// - Owner matches `expected_owner`
/// - Data length ≥ `min_size` (fixed prefix + segment table)
/// - `layout_id` matches (uses `SEGMENTED_LAYOUT_ID`)
///
/// # Errors
///
/// - `IllegalOwner`: account is not owned by `expected_owner`.
/// - `AccountDataTooSmall`: account data is smaller than `min_size`.
/// - `InvalidAccountData`: `layout_id` does not match.
#[inline(always)]
pub fn validate_foreign_segmented<'a>(
    account: &'a AccountView,
    expected_owner: &Address,
    layout_id: &[u8; 8],
    min_size: usize,
) -> Result<Ref<'a, [u8]>, ProgramError> {
    if !account.owned_by(expected_owner) {
        return Err(ProgramError::IllegalOwner);
    }

    let data = account.try_borrow()?;

    if data.len() < min_size {
        return Err(ProgramError::AccountDataTooSmall);
    }

    check_layout_id(&data, layout_id)?;
    Ok(data)
}

/// Mutable variant of [`validate_account`] (Tier 1).
///
/// Same checks as `validate_account` but returns `RefMut` for write access.
#[inline(always)]
pub fn validate_account_mut<'a>(
    account: &'a AccountView,
    program_id: &Address,
    disc: u8,
    version: u8,
    layout_id: &[u8; 8],
    expected_size: usize,
) -> Result<RefMut<'a, [u8]>, ProgramError> {
    if !account.owned_by(program_id) {
        return Err(ProgramError::IllegalOwner);
    }

    let data = account.try_borrow_mut()?;

    if data.len() != expected_size {
        return Err(ProgramError::AccountDataTooSmall);
    }

    check_header(&data, disc, version, layout_id)?;
    Ok(data)
}

/// Validate owner + disc + minimum version + minimum size.
///
/// # ⚠ Migration utility - lower trust level
///
/// **Compatibility validation is a migration helper, not a proof of ABI
/// identity.** It does not validate `layout_id` and must not be treated
/// as equivalent to `load()` or `load_foreign()`. It checks only that
/// the account is owned by the expected program, has the correct
/// discriminator, meets a minimum version, and is large enough.
///
/// Because the layout fingerprint is not verified, the caller must
/// ensure the overlaid struct is compatible with the actual on-chain
/// bytes. A `validate_version_compatible` call passing does **not** prove the
/// account's byte layout matches your Rust struct.
///
/// Use this only for backward-compatible loading during version
/// transitions. For all other paths, prefer:
///
/// - `load()` / `load_checked()`: full ABI-verified standard path
/// - `load_foreign()`: cross-program reads with layout_id proof
///
/// # Disabled in `strict` mode
///
/// When the `strict` feature is enabled this function is unavailable,
/// forcing all loads through layout_id-verified tiers.
///
/// # Errors
///
/// - `IllegalOwner`: account is not owned by `program_id`.
/// - `AccountDataTooSmall`: data shorter than `min_size` or shorter
///   than 2 bytes (cannot read disc + version).
/// - `InvalidAccountData`: discriminator does not match `disc`, or
///   version byte is less than `min_version`.
#[cfg(not(feature = "strict"))]
#[inline(always)]
pub fn validate_version_compatible<'a>(
    account: &'a AccountView,
    program_id: &Address,
    disc: u8,
    min_version: u8,
    min_size: usize,
) -> Result<Ref<'a, [u8]>, ProgramError> {
    if !account.owned_by(program_id) {
        return Err(ProgramError::IllegalOwner);
    }

    let data = account.try_borrow()?;

    // Need at least 2 bytes for disc + version, and at least min_size
    // for the caller's layout.
    if data.len() < min_size || data.len() < 2 {
        return Err(ProgramError::AccountDataTooSmall);
    }

    if data[0] != disc {
        return Err(ProgramError::InvalidAccountData);
    }
    if data[1] < min_version {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(data)
}

/// Try to validate header + layout_id. If the header check fails,
/// fall back to a plain overlay.
///
/// Unverified overlay loading (Tier 5). Useful for indexers and tooling
/// that need to read accounts without knowing whether they use Jiminy
/// headers. Returns `(overlay, validated)` where `validated` is `true`
/// when the header matched.
///
/// No ABI guarantees. The overlay is applied regardless of whether the
/// header validation succeeds. Use this only for diagnostic/tooling
/// purposes, never in on-chain program logic.
///
/// # Errors
///
/// - `AccountDataTooSmall`: data shorter than `T::SIZE`.
#[inline(always)]
pub fn load_unverified_overlay<'a, T: Pod + FixedLayout>(
    data: &'a [u8],
    disc: u8,
    version: u8,
    layout_id: &[u8; 8],
) -> Result<(&'a T, bool), ProgramError> {
    if data.len() < T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let validated = data.len() >= HEADER_LEN
        && check_header(data, disc, version, layout_id).is_ok();

    let overlay = pod_from_bytes::<T>(data)?;
    Ok((overlay, validated))
}
