//! # jiminy-anchor
//!
//! Interoperability bridge between Anchor-framework accounts and Jiminy
//! programs. No dependency on `anchor-lang` itself - this crate operates
//! purely on raw byte layouts and deterministic hash conventions.
//!
//! ## Anchor Account Format
//!
//! Anchor accounts use an 8-byte discriminator at bytes `[0..8]`,
//! computed as `sha256("account:<TypeName>")[..8]`. The remaining bytes
//! are Borsh-serialized data. Anchor's `zero_copy` attribute uses
//! `#[repr(C)]` overlays but still prefixes the same 8-byte discriminator.
//!
//! Anchor instructions also carry an 8-byte discriminator, computed as
//! `sha256("global:<function_name>")[..8]`, placed at the start of
//! instruction data.
//!
//! ## What This Crate Provides
//!
//! ### Account discriminators
//!
//! - [`anchor_disc`] - compute the 8-byte Anchor account discriminator at compile time
//! - [`check_anchor_disc`] - validate an Anchor discriminator on raw account data
//! - [`AnchorHeader`] - zero-copy overlay for the 8-byte Anchor discriminator
//!
//! ### Instruction discriminators
//!
//! - [`anchor_ix_disc`] - compute the 8-byte Anchor instruction discriminator at compile time
//! - [`check_anchor_ix_disc`] - validate an instruction discriminator on instruction data
//!
//! ### Body access
//!
//! - [`anchor_body`] / [`anchor_body_mut`] - get the body slice (bytes `[8..]`) from Anchor account data
//! - [`check_and_body`] - discriminator check + body slice in one call
//! - [`check_and_overlay`] / [`check_and_overlay_mut`] - discriminator check + Pod overlay on the body
//!
//! ### Cross-framework verification
//!
//! - [`check_anchor_with_layout_id`] - verify both Anchor disc and Jiminy `layout_id`
//! - [`check_anchor_with_version`] - verify disc + Jiminy layout_id + version for versioned interop
//!
//! ### AccountView helpers
//!
//! - [`load_anchor_account`] - validate owner + Anchor disc + borrow from an `AccountView`
//! - [`load_anchor_overlay`] - validate owner + Anchor disc, borrow, then Pod overlay the body
//!
//! ## Integration Pattern: Anchor + Jiminy
//!
//! Use Anchor for orchestration (instruction routing, account
//! deserialization, constraint macros) and Jiminy for the performance-critical
//! hot path (zero-copy reads, math, CPI guards). Jiminy's `zero_copy_layout!`
//! accounts can coexist with Anchor's `#[account(zero_copy)]` by sharing
//! a common `#[repr(C)]` body layout.
//!
//! A typical pattern:
//!
//! 1. Anchor program creates accounts with its discriminator.
//! 2. Jiminy helper program reads those accounts via [`check_and_overlay`].
//! 3. If the Anchor body is itself a Jiminy layout (with `AccountHeader`),
//!    use [`check_anchor_with_layout_id`] for full cross-framework verification.
//!
//! ## Example: Reading an Anchor account from a Jiminy program
//!
//! ```rust,ignore
//! use jiminy_anchor::{anchor_disc, check_anchor_disc, anchor_body};
//! use jiminy_core::account::{pod_from_bytes, Pod, FixedLayout};
//!
//! // Compute Anchor's discriminator for "Vault" at compile time.
//! const VAULT_DISC: [u8; 8] = anchor_disc("Vault");
//!
//! #[repr(C)]
//! #[derive(Clone, Copy)]
//! struct AnchorVaultBody {
//!     balance: [u8; 8],   // u64 LE
//!     authority: [u8; 32],
//! }
//! unsafe impl Pod for AnchorVaultBody {}
//! impl FixedLayout for AnchorVaultBody { const SIZE: usize = 40; }
//!
//! fn read_anchor_vault(data: &[u8]) -> Result<&AnchorVaultBody, jiminy_core::ProgramError> {
//!     check_anchor_disc(data, &VAULT_DISC)?;
//!     pod_from_bytes::<AnchorVaultBody>(&data[8..])
//! }
//! ```
//!
//! ## Example: Routing Anchor instructions from a Jiminy program
//!
//! ```rust,ignore
//! use jiminy_anchor::anchor_ix_disc;
//!
//! const IX_DEPOSIT: [u8; 8] = anchor_ix_disc("deposit");
//! const IX_WITHDRAW: [u8; 8] = anchor_ix_disc("withdraw");
//!
//! fn process_instruction(data: &[u8]) -> ProgramResult {
//!     let (disc, body) = data.split_at(8);
//!     match disc.try_into().unwrap_or(&[0u8; 8]) {
//!         &IX_DEPOSIT  => process_deposit(body),
//!         &IX_WITHDRAW => process_withdraw(body),
//!         _ => Err(ProgramError::InvalidInstructionData),
//!     }
//! }
//! ```

#![no_std]

use pinocchio::error::ProgramError;

/// Compute the Anchor 8-byte discriminator for an account type name
/// at compile time.
///
/// Anchor uses `sha256("account:<TypeName>")[..8]`.
///
/// ```rust
/// use jiminy_anchor::anchor_disc;
///
/// const VAULT_DISC: [u8; 8] = anchor_disc("Vault");
/// // This is deterministic - same input always produces same output.
/// assert_eq!(VAULT_DISC, anchor_disc("Vault"));
/// ```
pub const fn anchor_disc(type_name: &str) -> [u8; 8] {
    // Build "account:<TypeName>" input.
    let prefix = b"account:";
    let name = type_name.as_bytes();
    let total_len = prefix.len() + name.len();

    // Concatenate into a fixed buffer (max 256 bytes should be plenty).
    let mut buf = [0u8; 256];
    let mut i = 0;
    while i < prefix.len() {
        buf[i] = prefix[i];
        i += 1;
    }
    let mut j = 0;
    while j < name.len() {
        buf[i + j] = name[j];
        j += 1;
    }

    // SHA-256 the input.
    let hash = sha2_const_stable::Sha256::new()
        .update(const_slice(&buf, total_len))
        .finalize();

    // Take first 8 bytes.
    [hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]]
}

/// Helper: slice a fixed-size array to `len` bytes in const context.
const fn const_slice(buf: &[u8; 256], len: usize) -> &[u8] {
    // SAFETY: len <= 256 guaranteed by caller.
    // In const context we can use split_at.
    let (head, _) = buf.split_at(len);
    head
}

/// Validate that the first 8 bytes of account data match the expected
/// Anchor discriminator.
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than 8 bytes.
/// - `InvalidAccountData` - discriminator does not match `expected`.
#[inline(always)]
pub fn check_anchor_disc(data: &[u8], expected: &[u8; 8]) -> Result<(), ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[0] != expected[0]
        || data[1] != expected[1]
        || data[2] != expected[2]
        || data[3] != expected[3]
        || data[4] != expected[4]
        || data[5] != expected[5]
        || data[6] != expected[6]
        || data[7] != expected[7]
    {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Zero-copy overlay for the 8-byte Anchor discriminator.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AnchorHeader {
    /// The 8-byte discriminator (`sha256("account:<Type>")[..8]`).
    pub discriminator: [u8; 8],
}

unsafe impl jiminy_core::account::Pod for AnchorHeader {}
impl jiminy_core::account::FixedLayout for AnchorHeader {
    const SIZE: usize = 8;
}

/// Get the body of an Anchor account (everything after the 8-byte discriminator).
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than 8 bytes.
#[inline(always)]
pub fn anchor_body(data: &[u8]) -> Result<&[u8], ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(&data[8..])
}

/// Get the mutable body of an Anchor account.
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than 8 bytes.
#[inline(always)]
pub fn anchor_body_mut(data: &mut [u8]) -> Result<&mut [u8], ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(&mut data[8..])
}

/// Convenience: check discriminator and return the body slice in one call.
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than 8 bytes.
/// - `InvalidAccountData` - discriminator does not match `expected`.
#[inline(always)]
pub fn check_and_body<'a>(data: &'a [u8], expected: &[u8; 8]) -> Result<&'a [u8], ProgramError> {
    check_anchor_disc(data, expected)?;
    Ok(&data[8..])
}

/// Check the Anchor discriminator and overlay a `Pod` type on the body.
///
/// Validates the 8-byte Anchor discriminator, then reinterprets the
/// remaining bytes as an immutable reference to `T`. This is the
/// primary way to read an Anchor `zero_copy` account body as a Jiminy
/// overlay.
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than `8 + T::SIZE`.
/// - `InvalidAccountData` - discriminator mismatch.
#[inline(always)]
pub fn check_and_overlay<'a, T: jiminy_core::account::Pod + jiminy_core::account::FixedLayout>(
    data: &'a [u8],
    expected_disc: &[u8; 8],
) -> Result<&'a T, ProgramError> {
    check_anchor_disc(data, expected_disc)?;
    jiminy_core::account::pod_from_bytes::<T>(&data[8..])
}

/// Check the Anchor discriminator and overlay a mutable `Pod` type on the body.
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than `8 + T::SIZE`.
/// - `InvalidAccountData` - discriminator mismatch.
#[inline(always)]
pub fn check_and_overlay_mut<'a, T: jiminy_core::account::Pod + jiminy_core::account::FixedLayout>(
    data: &'a mut [u8],
    expected_disc: &[u8; 8],
) -> Result<&'a mut T, ProgramError> {
    check_anchor_disc(data, expected_disc)?;
    jiminy_core::account::pod_from_bytes_mut::<T>(&mut data[8..])
}

/// Validate that an Anchor `zero_copy` account body carries a Jiminy
/// `layout_id` at the expected position.
///
/// Anchor `zero_copy` accounts prefix the body with an 8-byte
/// discriminator. If the body is itself a Jiminy layout (starting with
/// an `AccountHeader`), the `layout_id` lives at body offset `4..12`
/// (i.e. account offset `12..20`).
///
/// This enables cross-framework verification: a Jiminy program can
/// confirm that an Anchor account's body matches a known Jiminy schema.
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than `8 + 12` bytes.
/// - `InvalidAccountData` - discriminator or layout_id mismatch.
#[inline(always)]
pub fn check_anchor_with_layout_id(
    data: &[u8],
    expected_disc: &[u8; 8],
    expected_layout_id: &[u8; 8],
) -> Result<(), ProgramError> {
    check_anchor_disc(data, expected_disc)?;
    let body = &data[8..];
    if body.len() < 12 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    // layout_id sits at body offset 4..12 (after disc(1) + version(1) + flags(2))
    if body[4..12] != expected_layout_id[..] {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── Instruction discriminator ─────────────────────────────────────────────────

/// Compute the Anchor 8-byte instruction discriminator at compile time.
///
/// Anchor uses `sha256("global:<function_name>")[..8]` for instruction
/// routing. This matches Anchor's `#[instruction]` attribute behavior.
///
/// ```rust
/// use jiminy_anchor::anchor_ix_disc;
///
/// const DEPOSIT: [u8; 8] = anchor_ix_disc("deposit");
/// const WITHDRAW: [u8; 8] = anchor_ix_disc("withdraw");
/// assert_ne!(DEPOSIT, WITHDRAW);
/// ```
pub const fn anchor_ix_disc(fn_name: &str) -> [u8; 8] {
    let prefix = b"global:";
    let name = fn_name.as_bytes();
    let total_len = prefix.len() + name.len();

    let mut buf = [0u8; 256];
    let mut i = 0;
    while i < prefix.len() {
        buf[i] = prefix[i];
        i += 1;
    }
    let mut j = 0;
    while j < name.len() {
        buf[i + j] = name[j];
        j += 1;
    }

    let hash = sha2_const_stable::Sha256::new()
        .update(const_slice(&buf, total_len))
        .finalize();

    [hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]]
}

/// Validate that instruction data starts with the expected Anchor
/// instruction discriminator.
///
/// # Errors
///
/// - `InvalidInstructionData` - data shorter than 8 bytes or discriminator mismatch.
#[inline(always)]
pub fn check_anchor_ix_disc(data: &[u8], expected: &[u8; 8]) -> Result<(), ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    if data[0] != expected[0]
        || data[1] != expected[1]
        || data[2] != expected[2]
        || data[3] != expected[3]
        || data[4] != expected[4]
        || data[5] != expected[5]
        || data[6] != expected[6]
        || data[7] != expected[7]
    {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

/// Check the instruction discriminator and return the remaining
/// instruction body (bytes after the 8-byte discriminator).
///
/// # Errors
///
/// - `InvalidInstructionData` - data shorter than 8 bytes or discriminator mismatch.
#[inline(always)]
pub fn check_ix_and_body<'a>(
    data: &'a [u8],
    expected: &[u8; 8],
) -> Result<&'a [u8], ProgramError> {
    check_anchor_ix_disc(data, expected)?;
    Ok(&data[8..])
}

// ── Version-aware cross-framework verification ───────────────────────────────

/// Validate Anchor discriminator, Jiminy `layout_id`, and Jiminy
/// `version` on a cross-framework account.
///
/// This is the strongest cross-framework check: it proves the account
/// has the right Anchor type, the right Jiminy ABI fingerprint, *and*
/// the right schema version.
///
/// The Jiminy header sits at the Anchor body start:
/// ```text
/// [0..8]   Anchor disc
/// [8]      Jiminy disc (1 byte)
/// [9]      Jiminy version (1 byte)
/// [10..12] Jiminy flags (2 bytes)
/// [12..20] Jiminy layout_id (8 bytes)
/// [20..24] Jiminy reserved (4 bytes)
/// ```
///
/// # Errors
///
/// - `AccountDataTooSmall` - data shorter than 24 bytes.
/// - `InvalidAccountData` - discriminator, layout_id, or version mismatch.
#[inline(always)]
pub fn check_anchor_with_version(
    data: &[u8],
    expected_disc: &[u8; 8],
    expected_layout_id: &[u8; 8],
    expected_version: u8,
) -> Result<(), ProgramError> {
    check_anchor_disc(data, expected_disc)?;
    let body = &data[8..];
    if body.len() < 16 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    // version at body offset 1
    if body[1] != expected_version {
        return Err(ProgramError::InvalidAccountData);
    }
    // layout_id at body offset 4..12
    if body[4..12] != expected_layout_id[..] {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── AccountView helpers ──────────────────────────────────────────────────────

/// Load an Anchor account from an `AccountView`: validate owner +
/// discriminator, then borrow the data.
///
/// This is the Anchor equivalent of Jiminy's Tier-1 `load()`. It
/// verifies that the account is owned by the expected program and
/// carries the correct Anchor discriminator before returning a
/// borrowed reference to the raw data.
///
/// # Errors
///
/// - `IllegalOwner` - account not owned by `expected_owner`.
/// - `AccountDataTooSmall` - data shorter than 8 bytes.
/// - `InvalidAccountData` - discriminator mismatch.
#[inline(always)]
pub fn load_anchor_account<'a>(
    account: &'a pinocchio::AccountView,
    expected_owner: &pinocchio::Address,
    expected_disc: &[u8; 8],
) -> Result<pinocchio::account::Ref<'a, [u8]>, ProgramError> {
    // SAFETY: owner() returns a pointer to data owned by the runtime;
    // we only read 32 bytes from it for comparison.
    if unsafe { account.owner() } != expected_owner {
        return Err(ProgramError::IllegalOwner);
    }
    let data = account.try_borrow()?;
    if data.len() < 8 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[0] != expected_disc[0]
        || data[1] != expected_disc[1]
        || data[2] != expected_disc[2]
        || data[3] != expected_disc[3]
        || data[4] != expected_disc[4]
        || data[5] != expected_disc[5]
        || data[6] != expected_disc[6]
        || data[7] != expected_disc[7]
    {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(data)
}

/// Load an Anchor account, validate owner + discriminator, then overlay
/// the body as a `Pod` type.
///
/// Combines owner validation, discriminator checking, borrowing, and
/// Pod overlay into a single call - the recommended way to read an
/// Anchor `zero_copy` account from a Jiminy program.
///
/// # Errors
///
/// - `IllegalOwner` - account not owned by `expected_owner`.
/// - `AccountDataTooSmall` - data too short for disc + body.
/// - `InvalidAccountData` - discriminator mismatch or body too small.
#[inline(always)]
pub fn load_anchor_overlay<'a, T: jiminy_core::account::Pod + jiminy_core::account::FixedLayout>(
    account: &'a pinocchio::AccountView,
    expected_owner: &pinocchio::Address,
    expected_disc: &[u8; 8],
) -> Result<pinocchio::account::Ref<'a, [u8]>, ProgramError> {
    // SAFETY: owner() returns a pointer to data owned by the runtime;
    // we only read 32 bytes from it for comparison.
    if unsafe { account.owner() } != expected_owner {
        return Err(ProgramError::IllegalOwner);
    }
    let data = account.try_borrow()?;
    if data.len() < 8 + T::SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[0] != expected_disc[0]
        || data[1] != expected_disc[1]
        || data[2] != expected_disc[2]
        || data[3] != expected_disc[3]
        || data[4] != expected_disc[4]
        || data[5] != expected_disc[5]
        || data[6] != expected_disc[6]
        || data[7] != expected_disc[7]
    {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(data)
}

// ── Anchor event discriminator ───────────────────────────────────────────────

/// Compute the Anchor 8-byte event discriminator at compile time.
///
/// Anchor events use `sha256("event:<EventName>")[..8]`.
///
/// ```rust
/// use jiminy_anchor::anchor_event_disc;
///
/// const DEPOSIT_EVENT: [u8; 8] = anchor_event_disc("DepositEvent");
/// assert_eq!(DEPOSIT_EVENT, anchor_event_disc("DepositEvent"));
/// ```
pub const fn anchor_event_disc(event_name: &str) -> [u8; 8] {
    let prefix = b"event:";
    let name = event_name.as_bytes();
    let total_len = prefix.len() + name.len();

    let mut buf = [0u8; 256];
    let mut i = 0;
    while i < prefix.len() {
        buf[i] = prefix[i];
        i += 1;
    }
    let mut j = 0;
    while j < name.len() {
        buf[i + j] = name[j];
        j += 1;
    }

    let hash = sha2_const_stable::Sha256::new()
        .update(const_slice(&buf, total_len))
        .finalize();

    [hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disc_is_deterministic() {
        let d1 = anchor_disc("Vault");
        let d2 = anchor_disc("Vault");
        assert_eq!(d1, d2);
    }

    #[test]
    fn different_names_different_discs() {
        let v = anchor_disc("Vault");
        let p = anchor_disc("Pool");
        assert_ne!(v, p);
    }

    #[test]
    fn check_disc_succeeds() {
        let disc = anchor_disc("Vault");
        let mut data = [0u8; 48];
        data[..8].copy_from_slice(&disc);
        assert!(check_anchor_disc(&data, &disc).is_ok());
    }

    #[test]
    fn check_disc_rejects_wrong() {
        let disc = anchor_disc("Vault");
        let data = [0u8; 48]; // all zeros
        assert!(check_anchor_disc(&data, &disc).is_err());
    }

    #[test]
    fn check_disc_rejects_short() {
        let disc = anchor_disc("Vault");
        let data = [0u8; 4]; // too short
        assert!(check_anchor_disc(&data, &disc).is_err());
    }

    #[test]
    fn anchor_body_returns_tail() {
        let mut data = [0u8; 16];
        data[8] = 42;
        let body = anchor_body(&data).unwrap();
        assert_eq!(body.len(), 8);
        assert_eq!(body[0], 42);
    }

    #[test]
    fn check_and_body_combined() {
        let disc = anchor_disc("Pool");
        let mut data = [0u8; 32];
        data[..8].copy_from_slice(&disc);
        data[8] = 0xFF;
        let body = check_and_body(&data, &disc).unwrap();
        assert_eq!(body[0], 0xFF);
    }

    #[test]
    fn check_and_overlay_reads_body() {
        // Create an Anchor account with an 8-byte disc + a u64 body.
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Balance { val: [u8; 8] }
        unsafe impl jiminy_core::account::Pod for Balance {}
        impl jiminy_core::account::FixedLayout for Balance {
            const SIZE: usize = 8;
        }

        let disc = anchor_disc("Balance");
        let mut data = [0u8; 16];
        data[..8].copy_from_slice(&disc);
        data[8..16].copy_from_slice(&42u64.to_le_bytes());

        let overlay = check_and_overlay::<Balance>(&data, &disc).unwrap();
        assert_eq!(u64::from_le_bytes(overlay.val), 42);
    }

    #[test]
    fn check_and_overlay_rejects_wrong_disc() {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Dummy { v: [u8; 4] }
        unsafe impl jiminy_core::account::Pod for Dummy {}
        impl jiminy_core::account::FixedLayout for Dummy {
            const SIZE: usize = 4;
        }

        let disc = anchor_disc("X");
        let data = [0u8; 16];
        assert!(check_and_overlay::<Dummy>(&data, &disc).is_err());
    }

    #[test]
    fn check_and_overlay_mut_writes() {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Val { v: [u8; 8] }
        unsafe impl jiminy_core::account::Pod for Val {}
        impl jiminy_core::account::FixedLayout for Val {
            const SIZE: usize = 8;
        }

        let disc = anchor_disc("Val");
        let mut data = [0u8; 16];
        data[..8].copy_from_slice(&disc);

        let overlay = check_and_overlay_mut::<Val>(&mut data, &disc).unwrap();
        overlay.v = 99u64.to_le_bytes();
        assert_eq!(&data[8..16], &99u64.to_le_bytes());
    }

    #[test]
    fn layout_id_check_passes() {
        let disc = anchor_disc("MyLayout");
        let layout_id = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        // Anchor disc (8) + jiminy header body: disc(1) + ver(1) + flags(2) + layout_id(8) = 20 total
        let mut data = [0u8; 24];
        data[..8].copy_from_slice(&disc);
        // body offset 4..12 = account offset 12..20
        data[12..20].copy_from_slice(&layout_id);

        assert!(check_anchor_with_layout_id(&data, &disc, &layout_id).is_ok());
    }

    #[test]
    fn layout_id_check_rejects_mismatch() {
        let disc = anchor_disc("MyLayout");
        let layout_id = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let mut data = [0u8; 24];
        data[..8].copy_from_slice(&disc);
        // leave layout_id area as zeros - mismatch

        assert!(check_anchor_with_layout_id(&data, &disc, &layout_id).is_err());
    }

    #[test]
    fn layout_id_check_rejects_too_short() {
        let disc = anchor_disc("MyLayout");
        let layout_id = [0x11; 8];
        let mut data = [0u8; 14]; // too short for disc + 12 body bytes
        data[..8].copy_from_slice(&disc);

        assert!(check_anchor_with_layout_id(&data, &disc, &layout_id).is_err());
    }

    // ── Instruction discriminator tests ──────────────────────────────

    #[test]
    fn ix_disc_is_deterministic() {
        let d1 = anchor_ix_disc("deposit");
        let d2 = anchor_ix_disc("deposit");
        assert_eq!(d1, d2);
    }

    #[test]
    fn ix_disc_differs_from_account_disc() {
        // "global:Vault" vs "account:Vault" - different prefixes
        let ix = anchor_ix_disc("Vault");
        let acct = anchor_disc("Vault");
        assert_ne!(ix, acct);
    }

    #[test]
    fn different_ix_names_different_discs() {
        let d = anchor_ix_disc("deposit");
        let w = anchor_ix_disc("withdraw");
        assert_ne!(d, w);
    }

    #[test]
    fn check_ix_disc_succeeds() {
        let disc = anchor_ix_disc("deposit");
        let mut data = [0u8; 32];
        data[..8].copy_from_slice(&disc);
        assert!(check_anchor_ix_disc(&data, &disc).is_ok());
    }

    #[test]
    fn check_ix_disc_rejects_wrong() {
        let disc = anchor_ix_disc("deposit");
        let data = [0u8; 32]; // all zeros
        assert!(check_anchor_ix_disc(&data, &disc).is_err());
    }

    #[test]
    fn check_ix_disc_rejects_short() {
        let disc = anchor_ix_disc("deposit");
        let data = [0u8; 4]; // too short
        assert!(check_anchor_ix_disc(&data, &disc).is_err());
    }

    #[test]
    fn check_ix_and_body_returns_tail() {
        let disc = anchor_ix_disc("process");
        let mut data = [0u8; 16];
        data[..8].copy_from_slice(&disc);
        data[8] = 42;
        let body = check_ix_and_body(&data, &disc).unwrap();
        assert_eq!(body.len(), 8);
        assert_eq!(body[0], 42);
    }

    // ── Version-aware cross-framework tests ──────────────────────────

    #[test]
    fn version_check_passes() {
        let disc = anchor_disc("MyLayout");
        let layout_id = [0xAA; 8];
        let mut data = [0u8; 28];
        data[..8].copy_from_slice(&disc);
        data[9] = 2; // version at body offset 1
        data[12..20].copy_from_slice(&layout_id);

        assert!(check_anchor_with_version(&data, &disc, &layout_id, 2).is_ok());
    }

    #[test]
    fn version_check_rejects_wrong_version() {
        let disc = anchor_disc("MyLayout");
        let layout_id = [0xAA; 8];
        let mut data = [0u8; 28];
        data[..8].copy_from_slice(&disc);
        data[9] = 1; // version 1
        data[12..20].copy_from_slice(&layout_id);

        assert!(check_anchor_with_version(&data, &disc, &layout_id, 2).is_err());
    }

    #[test]
    fn version_check_rejects_short() {
        let disc = anchor_disc("MyLayout");
        let layout_id = [0xAA; 8];
        let mut data = [0u8; 20]; // needs 24 (8 disc + 16 header)
        data[..8].copy_from_slice(&disc);

        assert!(check_anchor_with_version(&data, &disc, &layout_id, 1).is_err());
    }

    // ── Event discriminator tests ────────────────────────────────────

    #[test]
    fn event_disc_is_deterministic() {
        let d1 = anchor_event_disc("TransferEvent");
        let d2 = anchor_event_disc("TransferEvent");
        assert_eq!(d1, d2);
    }

    #[test]
    fn event_disc_differs_from_account_and_ix() {
        let event = anchor_event_disc("X");
        let acct = anchor_disc("X");
        let ix = anchor_ix_disc("X");
        assert_ne!(event, acct);
        assert_ne!(event, ix);
    }
}
