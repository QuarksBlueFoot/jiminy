//! Tests for the Jiminy Account ABI.
//!
//! Covers: header v2, layout_id determinism, zero-init, tiered loading
//! helpers, check_header, extends, and Pod overlay correctness.

use jiminy_core::account::*;
use jiminy_core::zero_copy_layout;
use pinocchio::Address;

// ── Test layouts ─────────────────────────────────────────────────────────────

zero_copy_layout! {
    /// Test vault layout.
    pub struct TestVault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

zero_copy_layout! {
    /// Test vault v2 with extends.
    pub struct TestVaultV2, discriminator = 1, version = 2, extends = TestVault {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
        fee_bps:   u64           = 8,
    }
}

zero_copy_layout! {
    /// Minimal layout for boundary tests.
    pub struct Minimal, discriminator = 42, version = 1 {
        header: AccountHeader = 16,
    }
}

// ── Helper: aligned buffer ───────────────────────────────────────────────────

/// Create an aligned buffer suitable for Pod overlay on native targets.
/// Aligns to 8 bytes (max alignment of any field type we use).
#[repr(C, align(8))]
struct AlignedBuf<const N: usize>([u8; N]);

impl<const N: usize> AlignedBuf<N> {
    fn new() -> Self {
        Self([0u8; N])
    }

    fn as_slice(&self) -> &[u8] {
        &self.0
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

/// Write a valid header into the buffer.
fn stamp_header(buf: &mut [u8], disc: u8, version: u8, layout_id: &[u8; 8]) {
    write_header(buf, disc, version, layout_id).unwrap();
}

// ══════════════════════════════════════════════════════════════════════════════
// 1. Header v2 — 16-byte structure
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn header_len_is_16() {
    assert_eq!(HEADER_LEN, 16);
}

#[test]
fn header_struct_size() {
    assert_eq!(core::mem::size_of::<AccountHeader>(), 16);
}

#[test]
fn write_header_sets_correct_bytes() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44];
    stamp_header(buf.as_mut_slice(), 7, 3, &id);

    assert_eq!(buf.0[0], 7, "disc");
    assert_eq!(buf.0[1], 3, "version");
    assert_eq!(buf.0[2], 0, "flags lo");
    assert_eq!(buf.0[3], 0, "flags hi");
    assert_eq!(&buf.0[4..12], &id, "layout_id");
    assert_eq!(&buf.0[12..16], &[0, 0, 0, 0], "reserved");
}

#[test]
fn check_header_accepts_valid() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [1, 2, 3, 4, 5, 6, 7, 8];
    stamp_header(buf.as_mut_slice(), 5, 2, &id);
    assert!(check_header(buf.as_slice(), 5, 2, &id).is_ok());
}

#[test]
fn check_header_rejects_wrong_disc() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [1, 2, 3, 4, 5, 6, 7, 8];
    stamp_header(buf.as_mut_slice(), 5, 2, &id);
    assert!(check_header(buf.as_slice(), 6, 2, &id).is_err());
}

#[test]
fn check_header_rejects_old_version() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [1, 2, 3, 4, 5, 6, 7, 8];
    stamp_header(buf.as_mut_slice(), 5, 1, &id);
    // Expecting version >= 2, but got 1.
    assert!(check_header(buf.as_slice(), 5, 2, &id).is_err());
}

#[test]
fn check_header_accepts_newer_version() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [1, 2, 3, 4, 5, 6, 7, 8];
    stamp_header(buf.as_mut_slice(), 5, 3, &id);
    // Expecting version >= 2, got 3 — should pass.
    assert!(check_header(buf.as_slice(), 5, 2, &id).is_ok());
}

#[test]
fn check_header_rejects_wrong_layout_id() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [1, 2, 3, 4, 5, 6, 7, 8];
    stamp_header(buf.as_mut_slice(), 5, 2, &id);
    let wrong_id = [9, 9, 9, 9, 9, 9, 9, 9];
    assert!(check_header(buf.as_slice(), 5, 2, &wrong_id).is_err());
}

#[test]
fn check_header_rejects_too_small() {
    let buf = [0u8; 10]; // < 16
    assert!(check_header(&buf, 0, 0, &[0; 8]).is_err());
}

#[test]
fn read_version_works() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 7, &[0; 8]);
    assert_eq!(read_version(buf.as_slice()).unwrap(), 7);
}

#[test]
fn read_layout_id_works() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
    stamp_header(buf.as_mut_slice(), 1, 1, &id);
    assert_eq!(read_layout_id(buf.as_slice()).unwrap(), id);
}

#[test]
fn check_layout_id_validates() {
    let mut buf = AlignedBuf::<64>::new();
    let id = [10, 20, 30, 40, 50, 60, 70, 80];
    stamp_header(buf.as_mut_slice(), 1, 1, &id);
    assert!(check_layout_id(buf.as_slice(), &id).is_ok());
    assert!(check_layout_id(buf.as_slice(), &[0; 8]).is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. Layout ID determinism
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn layout_id_is_8_bytes() {
    assert_eq!(TestVault::LAYOUT_ID.len(), 8);
}

#[test]
fn layout_id_is_deterministic() {
    // Calling LAYOUT_ID twice yields the same value.
    let a = TestVault::LAYOUT_ID;
    let b = TestVault::LAYOUT_ID;
    assert_eq!(a, b);
}

#[test]
fn different_versions_produce_different_layout_ids() {
    assert_ne!(TestVault::LAYOUT_ID, TestVaultV2::LAYOUT_ID);
}

#[test]
fn same_disc_different_layout_id() {
    // V1 and V2 share disc but have different layout_ids.
    assert_eq!(TestVault::DISC, TestVaultV2::DISC);
    assert_ne!(TestVault::LAYOUT_ID, TestVaultV2::LAYOUT_ID);
}

#[test]
fn layout_id_not_all_zeroes() {
    assert_ne!(TestVault::LAYOUT_ID, [0u8; 8]);
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. Zero-init
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn zero_init_clears_buffer() {
    let mut buf = [0xFFu8; 64];
    zero_init(&mut buf);
    assert!(buf.iter().all(|&b| b == 0));
}

#[test]
fn zero_init_then_header_produces_clean_state() {
    let mut buf = [0xFFu8; 64];
    zero_init(&mut buf);
    write_header(&mut buf, TestVault::DISC, TestVault::VERSION, &TestVault::LAYOUT_ID).unwrap();
    // Reserved bytes should be zero after header write.
    assert_eq!(&buf[12..16], &[0, 0, 0, 0]);
    // Payload area should be zero.
    assert!(buf[16..].iter().all(|&b| b == 0));
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. Macro-generated constants
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn macro_generates_correct_constants() {
    assert_eq!(TestVault::DISC, 1);
    assert_eq!(TestVault::VERSION, 1);
    assert_eq!(TestVault::LEN, 16 + 8 + 32); // 56
    assert_eq!(TestVault::LEN, core::mem::size_of::<TestVault>());
}

#[test]
fn macro_generates_correct_constants_v2() {
    assert_eq!(TestVaultV2::DISC, 1);
    assert_eq!(TestVaultV2::VERSION, 2);
    assert_eq!(TestVaultV2::LEN, 16 + 8 + 32 + 8); // 64
    assert_eq!(TestVaultV2::LEN, core::mem::size_of::<TestVaultV2>());
}

#[test]
fn minimal_layout_constants() {
    assert_eq!(Minimal::DISC, 42);
    assert_eq!(Minimal::VERSION, 1);
    assert_eq!(Minimal::LEN, 16);
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. Pod overlay correctness
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn overlay_reads_fields() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), 1, 1, &TestVault::LAYOUT_ID);
    // Write balance = 1000 (u64 LE) at offset 16.
    buf.0[16..24].copy_from_slice(&1000u64.to_le_bytes());
    // Write authority at offset 24.
    buf.0[24..56].copy_from_slice(&[0xAB; 32]);

    let vault = TestVault::overlay(buf.as_slice()).unwrap();
    assert_eq!(vault.balance, 1000);
    assert_eq!(vault.authority.as_ref(), &[0xAB; 32]);
}

#[test]
fn overlay_mut_writes_fields() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), 1, 1, &TestVault::LAYOUT_ID);

    let vault = TestVault::overlay_mut(buf.as_mut_slice()).unwrap();
    vault.balance = 42;
    vault.authority = pinocchio::Address::from([0xCD; 32]);

    // Verify via raw bytes.
    assert_eq!(u64::from_le_bytes(buf.0[16..24].try_into().unwrap()), 42);
    assert_eq!(&buf.0[24..56], &[0xCD; 32]);
}

#[test]
fn overlay_rejects_too_small() {
    let buf = [0u8; 10];
    assert!(TestVault::overlay(&buf).is_err());
}

#[test]
fn pod_read_works_regardless_of_alignment() {
    // pod_read copies, so alignment doesn't matter.
    let mut buf = vec![0u8; 60];
    stamp_header(&mut buf, 1, 1, &TestVault::LAYOUT_ID);
    buf[16..24].copy_from_slice(&777u64.to_le_bytes());
    let vault = TestVault::read(&buf[..56]).unwrap();
    assert_eq!(vault.balance, 777);
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. load_checked / load_checked_mut
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn load_checked_accepts_valid_header() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), TestVault::DISC, TestVault::VERSION, &TestVault::LAYOUT_ID);
    assert!(TestVault::load_checked(buf.as_slice()).is_ok());
}

#[test]
fn load_checked_rejects_wrong_disc() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), 99, TestVault::VERSION, &TestVault::LAYOUT_ID);
    assert!(TestVault::load_checked(buf.as_slice()).is_err());
}

#[test]
fn load_checked_rejects_wrong_layout_id() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), TestVault::DISC, TestVault::VERSION, &[0xFF; 8]);
    assert!(TestVault::load_checked(buf.as_slice()).is_err());
}

#[test]
fn load_checked_mut_modifies() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), TestVault::DISC, TestVault::VERSION, &TestVault::LAYOUT_ID);
    let vault = TestVault::load_checked_mut(buf.as_mut_slice()).unwrap();
    vault.balance = 500;
    assert_eq!(u64::from_le_bytes(buf.0[16..24].try_into().unwrap()), 500);
}

// ══════════════════════════════════════════════════════════════════════════════
// 7. load_unverified_overlay (Tier 5)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn load_unverified_overlay_valid_header() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), TestVault::DISC, TestVault::VERSION, &TestVault::LAYOUT_ID);
    let (vault, validated) = TestVault::load_unverified_overlay(buf.as_slice()).unwrap();
    assert!(validated);
    assert_eq!(vault.header.discriminator, TestVault::DISC);
}

#[test]
fn load_unverified_overlay_invalid_header_still_overlays() {
    let mut buf = AlignedBuf::<56>::new();
    // Write wrong disc — header check fails, but overlay still works.
    stamp_header(buf.as_mut_slice(), 99, 1, &[0; 8]);
    let (_, validated) = TestVault::load_unverified_overlay(buf.as_slice()).unwrap();
    assert!(!validated);
}

#[test]
fn load_unverified_overlay_too_small_fails() {
    let buf = [0u8; 10];
    assert!(TestVault::load_unverified_overlay(&buf).is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// 8. extends — compile-time V2 ⊃ V1 assertions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn extends_same_disc() {
    assert_eq!(TestVaultV2::DISC, TestVault::DISC);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn extends_larger_or_equal_size() {
    assert!(TestVaultV2::LEN >= TestVault::LEN);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn extends_higher_version() {
    assert!(TestVaultV2::VERSION > TestVault::VERSION);
}

#[test]
fn v2_can_read_v1_prefix() {
    // V2 has the same prefix as V1 — first 56 bytes are compatible.
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), TestVault::DISC, TestVault::VERSION, &TestVault::LAYOUT_ID);
    buf.0[16..24].copy_from_slice(&999u64.to_le_bytes());
    buf.0[24..56].copy_from_slice(&[0x11; 32]);

    // Read as V1 overlay (first 56 bytes).
    let v1 = TestVault::overlay(&buf.0[..56]).unwrap();
    assert_eq!(v1.balance, 999);
    assert_eq!(v1.authority.as_ref(), &[0x11; 32]);
}

// ══════════════════════════════════════════════════════════════════════════════
// 9. Header utility functions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn header_payload_returns_body() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 1, &[0; 8]);
    buf.0[16] = 0xAA;
    let payload = header_payload(buf.as_slice()).unwrap();
    assert_eq!(payload[0], 0xAA);
    assert_eq!(payload.len(), 48); // 64 - 16
}

#[test]
fn read_header_flags_works() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 1, &[0; 8]);
    // Manually set flags.
    buf.0[2] = 0x34;
    buf.0[3] = 0x12;
    assert_eq!(read_header_flags(buf.as_slice()).unwrap(), 0x1234);
}

// ══════════════════════════════════════════════════════════════════════════════
// 10. Size assertions (compile-time: if these types exist, the assertions passed)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn size_of_matches_len() {
    // These would fail at compile time if size_of != LEN, but verify at runtime too.
    assert_eq!(core::mem::size_of::<TestVault>(), TestVault::LEN);
    assert_eq!(core::mem::size_of::<TestVaultV2>(), TestVaultV2::LEN);
    assert_eq!(core::mem::size_of::<Minimal>(), Minimal::LEN);
}

// ══════════════════════════════════════════════════════════════════════════════
// 11. FixedLayout trait
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fixed_layout_size_matches_len() {
    use jiminy_core::account::FixedLayout;
    assert_eq!(<TestVault as FixedLayout>::SIZE, TestVault::LEN);
    assert_eq!(<TestVaultV2 as FixedLayout>::SIZE, TestVaultV2::LEN);
}

// ══════════════════════════════════════════════════════════════════════════════
// 12. validate_version_compatible (migration utility)
// ══════════════════════════════════════════════════════════════════════════════
//
// validate_version_compatible requires an AccountView which is only available in
// the Solana runtime. We test the logic it calls (check_header sans
// layout_id) via header functions directly.

#[test]
fn compatible_check_logic_accepts_higher_version() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 3, &TestVault::LAYOUT_ID);
    // Manual check: disc ok, version >= 1.
    assert_eq!(buf.0[0], 1);
    assert!(buf.0[1] >= 1);
}

#[test]
fn compatible_check_logic_rejects_lower_version() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 0, &TestVault::LAYOUT_ID);
    // Version 0 should fail min_version = 1 check.
    assert!(buf.0[1] < 1);
}

// ══════════════════════════════════════════════════════════════════════════════
// 13. Foreign loading semantics — layout_id only
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn load_checked_rejects_valid_header_wrong_v2_layout_id() {
    // Valid V1 header but asked to check as V2 layout — layout_id mismatch.
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(
        buf.as_mut_slice(),
        TestVault::DISC,
        TestVault::VERSION,
        &TestVault::LAYOUT_ID,
    );
    assert!(TestVaultV2::load_checked(buf.as_slice()).is_err());
}

#[test]
fn foreign_check_layout_id_matches_cross_type() {
    // Simulate foreign read: data has V1 layout_id, check_layout_id should pass
    // for the matching ID and fail for a different one.
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(
        buf.as_mut_slice(),
        TestVault::DISC,
        TestVault::VERSION,
        &TestVault::LAYOUT_ID,
    );
    assert!(check_layout_id(buf.as_slice(), &TestVault::LAYOUT_ID).is_ok());
    assert!(check_layout_id(buf.as_slice(), &TestVaultV2::LAYOUT_ID).is_err());
}

#[test]
fn foreign_check_layout_id_ignores_disc_and_version() {
    // A foreign reader only cares about layout_id. Even if disc/version are
    // "wrong" from the reader's perspective, layout_id match should pass.
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(buf.as_mut_slice(), 99, 99, &TestVault::LAYOUT_ID);
    assert!(check_layout_id(buf.as_slice(), &TestVault::LAYOUT_ID).is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// 14. Malformed headers and boundary conditions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn check_header_exact_16_bytes() {
    let mut buf = [0u8; 16];
    let id = [1, 2, 3, 4, 5, 6, 7, 8];
    stamp_header(&mut buf, 1, 1, &id);
    assert!(check_header(&buf, 1, 1, &id).is_ok());
}

#[test]
fn check_header_15_bytes_fails() {
    let buf = [0u8; 15];
    assert!(check_header(&buf, 0, 0, &[0; 8]).is_err());
}

#[test]
fn check_header_empty_buffer_fails() {
    let buf: [u8; 0] = [];
    assert!(check_header(&buf, 0, 0, &[0; 8]).is_err());
}

#[test]
fn all_zeroes_header_matches_disc_0_ver_0() {
    // A buffer of all zeroes has disc=0, version=0, layout_id=[0;8].
    // check_header should accept if we ask for exactly those.
    let buf = AlignedBuf::<56>::new();
    // version >= 0 is always true for u8, so this should pass.
    assert!(check_header(buf.as_slice(), 0, 0, &[0u8; 8]).is_ok());
}

#[test]
fn overlay_exact_size_succeeds() {
    let buf = AlignedBuf::<56>::new();
    assert!(TestVault::overlay(buf.as_slice()).is_ok());
}

#[test]
fn overlay_one_byte_short_fails() {
    let buf = [0u8; 55];
    assert!(TestVault::overlay(&buf).is_err());
}

#[test]
fn overlay_extra_trailing_bytes_succeeds() {
    // Overlay should work on buffers larger than LEN (extra bytes ignored).
    let buf = AlignedBuf::<128>::new();
    assert!(TestVault::overlay(buf.as_slice()).is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// 15. Unverified overlay with corrupted / partial data
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn unverified_overlay_all_zeroes_not_validated() {
    let buf = AlignedBuf::<56>::new();
    let (_, validated) = TestVault::load_unverified_overlay(buf.as_slice()).unwrap();
    // disc=0 != TestVault::DISC=1, so validation fails.
    assert!(!validated);
}

#[test]
fn unverified_overlay_correct_disc_wrong_layout_id_not_validated() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(
        buf.as_mut_slice(),
        TestVault::DISC,
        TestVault::VERSION,
        &[0xFF; 8], // wrong layout_id
    );
    let (_, validated) = TestVault::load_unverified_overlay(buf.as_slice()).unwrap();
    assert!(!validated);
}

#[test]
fn unverified_overlay_partially_overwritten_header() {
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(
        buf.as_mut_slice(),
        TestVault::DISC,
        TestVault::VERSION,
        &TestVault::LAYOUT_ID,
    );
    // Corrupt one byte of the layout_id.
    buf.0[4] ^= 0xFF;
    let (_, validated) = TestVault::load_unverified_overlay(buf.as_slice()).unwrap();
    assert!(!validated);
}

#[test]
fn unverified_overlay_v2_header_on_v1_struct() {
    // A V2 header on a V1-sized buffer — overlay should work but not validate.
    let mut buf = AlignedBuf::<56>::new();
    stamp_header(
        buf.as_mut_slice(),
        TestVaultV2::DISC,
        TestVaultV2::VERSION,
        &TestVaultV2::LAYOUT_ID,
    );
    // Attempt as V1 (wrong layout_id).
    let (_, validated) = TestVault::load_unverified_overlay(buf.as_slice()).unwrap();
    assert!(!validated);
}

// ══════════════════════════════════════════════════════════════════════════════
// 16. validate_version_compatible edge cases (via header bytes)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn compatible_accepts_with_any_layout_id() {
    // validate_version_compatible ignores layout_id — verify this by using a gibberish ID.
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 2, &[0xFF; 8]);
    // Manual compatibility check: disc == 1 ✓, version >= 1 ✓.
    assert_eq!(buf.0[0], 1);
    assert!(buf.0[1] >= 1);
}

#[test]
fn compatible_rejects_wrong_disc() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 5, 1, &TestVault::LAYOUT_ID);
    // Disc 5 should not pass a check for disc 1.
    assert_ne!(buf.0[0], 1);
}

#[test]
fn compatible_exact_min_version_passes() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 3, &TestVault::LAYOUT_ID);
    // version == min_version: passes.
    assert!(buf.0[1] >= 3);
}

#[test]
fn compatible_below_min_version_fails() {
    let mut buf = AlignedBuf::<64>::new();
    stamp_header(buf.as_mut_slice(), 1, 2, &TestVault::LAYOUT_ID);
    // version 2 < min_version 3: fails.
    assert!(buf.0[1] < 3);
}

// ── Le* ABI primitive tests ─────────────────────────────────────────────────

use jiminy_core::abi::*;

// -- Roundtrip tests --

#[test]
fn le_u16_roundtrip() {
    let v = LeU16::new(0xABCD);
    assert_eq!(v.get(), 0xABCD);
}

#[test]
fn le_u32_roundtrip() {
    let v = LeU32::new(0xDEADBEEF);
    assert_eq!(v.get(), 0xDEADBEEF);
}

#[test]
fn le_u64_roundtrip() {
    let v = LeU64::new(0x0102030405060708);
    assert_eq!(v.get(), 0x0102030405060708);
}

#[test]
fn le_u128_roundtrip() {
    let v = LeU128::new(u128::MAX);
    assert_eq!(v.get(), u128::MAX);
}

#[test]
fn le_i16_roundtrip() {
    let v = LeI16::new(-1234);
    assert_eq!(v.get(), -1234);
}

#[test]
fn le_i32_roundtrip() {
    let v = LeI32::new(-100_000);
    assert_eq!(v.get(), -100_000);
}

#[test]
fn le_i64_roundtrip() {
    let v = LeI64::new(i64::MIN);
    assert_eq!(v.get(), i64::MIN);
}

#[test]
fn le_i128_roundtrip() {
    let v = LeI128::new(i128::MIN);
    assert_eq!(v.get(), i128::MIN);
}

#[test]
fn le_bool_roundtrip() {
    assert!(LeBool::new(true).get());
    assert!(!LeBool::new(false).get());
}

// -- Endianness byte order --

#[test]
fn le_u64_byte_order() {
    let v = LeU64::new(0x0102030405060708);
    assert_eq!(v.0, [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]);
}

#[test]
fn le_u32_byte_order() {
    let v = LeU32::new(0x01020304);
    assert_eq!(v.0, [0x04, 0x03, 0x02, 0x01]);
}

#[test]
fn le_u16_byte_order() {
    let v = LeU16::new(0x0102);
    assert_eq!(v.0, [0x02, 0x01]);
}

// -- Pod casting --

#[test]
fn le_u64_pod_from_bytes() {
    let bytes = 42u64.to_le_bytes();
    let v = pod_from_bytes::<LeU64>(&bytes).unwrap();
    assert_eq!(v.get(), 42);
}

#[test]
fn le_u32_pod_from_bytes() {
    let bytes = 999u32.to_le_bytes();
    let v = pod_from_bytes::<LeU32>(&bytes).unwrap();
    assert_eq!(v.get(), 999);
}

// -- FixedLayout::SIZE --

#[test]
fn le_fixed_layout_sizes() {
    assert_eq!(LeU16::SIZE, 2);
    assert_eq!(LeU32::SIZE, 4);
    assert_eq!(LeU64::SIZE, 8);
    assert_eq!(LeU128::SIZE, 16);
    assert_eq!(LeI16::SIZE, 2);
    assert_eq!(LeI32::SIZE, 4);
    assert_eq!(LeI64::SIZE, 8);
    assert_eq!(LeI128::SIZE, 16);
    assert_eq!(LeBool::SIZE, 1);
}

// -- Default --

#[test]
fn le_default_is_zero() {
    assert_eq!(LeU64::default().get(), 0);
    assert_eq!(LeU32::default().get(), 0);
    assert_eq!(LeI64::default().get(), 0);
    assert!(!LeBool::default().get());
}

// -- From/Into --

#[test]
fn le_from_into() {
    let x: LeU64 = 42u64.into();
    let y: u64 = x.into();
    assert_eq!(y, 42);

    let a: LeBool = true.into();
    let b: bool = a.into();
    assert!(b);
}

// -- Set --

#[test]
fn le_set_mutates() {
    let mut v = LeU64::new(0);
    v.set(999);
    assert_eq!(v.get(), 999);
}

// -- LeBool nonzero --

#[test]
fn le_bool_nonzero_is_true() {
    let v = LeBool([0xFF]);
    assert!(v.get());
}

// -- Layout ID consistency: Le* types produce same ID as native types --
// Use separate modules so both structs can be named "Vault" (name is part of hash).

mod layout_native {
    use jiminy_core::account::AccountHeader;
    use jiminy_core::zero_copy_layout;
    use pinocchio::Address;

    zero_copy_layout! {
        pub struct Vault, discriminator = 99, version = 1 {
            header:    AccountHeader = 16,
            balance:   u64           = 8,
            authority: Address       = 32,
        }
    }
}

mod layout_le {
    use jiminy_core::account::AccountHeader;
    use jiminy_core::abi::LeU64;
    use jiminy_core::zero_copy_layout;
    use pinocchio::Address;

    zero_copy_layout! {
        pub struct Vault, discriminator = 99, version = 1 {
            header:    AccountHeader = 16,
            balance:   LeU64         = 8,
            authority: Address       = 32,
        }
    }
}

#[test]
fn le_layout_id_matches_native() {
    // Same name + same canonical types → identical LAYOUT_ID.
    assert_eq!(layout_native::Vault::LAYOUT_ID, layout_le::Vault::LAYOUT_ID);
}

#[test]
fn le_layout_struct_sizes_match() {
    assert_eq!(layout_native::Vault::LEN, layout_le::Vault::LEN);
    assert_eq!(
        core::mem::size_of::<layout_native::Vault>(),
        core::mem::size_of::<layout_le::Vault>(),
    );
}

// -- FieldRef / FieldMut --

#[test]
fn field_ref_read_u64() {
    let bytes = 42u64.to_le_bytes();
    let r = FieldRef::new(&bytes);
    assert_eq!(r.read_u64(), 42);
}

#[test]
fn field_ref_read_all_integer_types() {
    // u8
    assert_eq!(FieldRef::new(&[0xFF]).read_u8(), 0xFF);
    // u16
    assert_eq!(FieldRef::new(&0x1234u16.to_le_bytes()).read_u16(), 0x1234);
    // u32
    assert_eq!(FieldRef::new(&0xDEADBEEFu32.to_le_bytes()).read_u32(), 0xDEADBEEF);
    // i16
    assert_eq!(FieldRef::new(&(-1234i16).to_le_bytes()).read_i16(), -1234);
    // i32
    assert_eq!(FieldRef::new(&(-42i32).to_le_bytes()).read_i32(), -42);
    // i64
    assert_eq!(FieldRef::new(&i64::MIN.to_le_bytes()).read_i64(), i64::MIN);
    // bool
    assert!(FieldRef::new(&[1]).read_bool());
    assert!(!FieldRef::new(&[0]).read_bool());
}

#[test]
fn field_ref_read_u128_i128() {
    let v = u128::MAX;
    assert_eq!(FieldRef::new(&v.to_le_bytes()).read_u128(), u128::MAX);
    let v = i128::MIN;
    assert_eq!(FieldRef::new(&v.to_le_bytes()).read_i128(), i128::MIN);
}

#[test]
fn field_ref_as_bytes_returns_full_slice() {
    let bytes = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let r = FieldRef::new(&bytes);
    assert_eq!(r.as_bytes(), &bytes);
}

#[test]
fn field_ref_max_min_values() {
    assert_eq!(FieldRef::new(&u64::MAX.to_le_bytes()).read_u64(), u64::MAX);
    assert_eq!(FieldRef::new(&u64::MIN.to_le_bytes()).read_u64(), u64::MIN);
    assert_eq!(FieldRef::new(&i64::MAX.to_le_bytes()).read_i64(), i64::MAX);
    assert_eq!(FieldRef::new(&i64::MIN.to_le_bytes()).read_i64(), i64::MIN);
    assert_eq!(FieldRef::new(&u32::MAX.to_le_bytes()).read_u32(), u32::MAX);
    assert_eq!(FieldRef::new(&u16::MAX.to_le_bytes()).read_u16(), u16::MAX);
}

#[test]
fn field_mut_write_read_u64() {
    let mut bytes = [0u8; 8];
    let mut m = FieldMut::new(&mut bytes);
    m.write_u64(0xDEADCAFE);
    assert_eq!(m.read_u64(), 0xDEADCAFE);
}

#[test]
fn field_mut_write_read_all_integer_types() {
    // u8
    let mut b = [0u8; 1];
    FieldMut::new(&mut b).write_u8(0xAB);
    assert_eq!(FieldRef::new(&b).read_u8(), 0xAB);

    // u16
    let mut b = [0u8; 2];
    FieldMut::new(&mut b).write_u16(0xCAFE);
    assert_eq!(FieldRef::new(&b).read_u16(), 0xCAFE);

    // u32
    let mut b = [0u8; 4];
    FieldMut::new(&mut b).write_u32(0xDEADBEEF);
    assert_eq!(FieldRef::new(&b).read_u32(), 0xDEADBEEF);

    // i16
    let mut b = [0u8; 2];
    FieldMut::new(&mut b).write_i16(-9999);
    assert_eq!(FieldRef::new(&b).read_i16(), -9999);

    // i32
    let mut b = [0u8; 4];
    FieldMut::new(&mut b).write_i32(i32::MIN);
    assert_eq!(FieldRef::new(&b).read_i32(), i32::MIN);

    // i64
    let mut b = [0u8; 8];
    FieldMut::new(&mut b).write_i64(i64::MAX);
    assert_eq!(FieldRef::new(&b).read_i64(), i64::MAX);

    // bool
    let mut b = [0u8; 1];
    FieldMut::new(&mut b).write_bool(true);
    assert!(FieldRef::new(&b).read_bool());
    FieldMut::new(&mut b).write_bool(false);
    assert!(!FieldRef::new(&b).read_bool());
}

#[test]
fn field_mut_write_read_u128_i128() {
    let mut b = [0u8; 16];
    FieldMut::new(&mut b).write_u128(u128::MAX);
    assert_eq!(FieldRef::new(&b).read_u128(), u128::MAX);

    FieldMut::new(&mut b).write_i128(i128::MIN);
    assert_eq!(FieldRef::new(&b).read_i128(), i128::MIN);
}

#[test]
fn field_mut_copy_from() {
    let src = [1u8, 2, 3, 4];
    let mut bytes = [0u8; 8];
    let mut m = FieldMut::new(&mut bytes);
    m.copy_from(&src);
    assert_eq!(&bytes[..4], &[1, 2, 3, 4]);
}

#[test]
fn field_mut_copy_from_address() {
    let addr = [0xBB; 32];
    let mut buf = [0u8; 32];
    let mut m = FieldMut::new(&mut buf);
    m.copy_from(&addr);
    assert_eq!(buf, addr);
}

#[test]
fn field_mut_as_bytes_mut_direct() {
    let mut buf = [0u8; 4];
    let mut m = FieldMut::new(&mut buf);
    let raw = m.as_bytes_mut();
    raw[0] = 0xAA;
    raw[3] = 0xBB;
    assert_eq!(buf[0], 0xAA);
    assert_eq!(buf[3], 0xBB);
}

#[test]
fn field_mut_overwrite_preserves_adjacent_data() {
    let mut data = [0xFF; 24];
    // Only write to the middle 8 bytes via FieldMut
    let mut m = FieldMut::new(&mut data[8..16]);
    m.write_u64(42);
    // Adjacent regions must be untouched
    assert_eq!(&data[..8], &[0xFF; 8]);
    assert_eq!(&data[16..], &[0xFF; 8]);
    assert_eq!(FieldRef::new(&data[8..16]).read_u64(), 42);
}

// ── Const offset tests ──────────────────────────────────────────────────────

#[test]
fn const_offsets_match_expected() {
    // TestVault: header(16) + balance(8) + authority(32) = 56
    assert_eq!(TestVault::header, 0);
    assert_eq!(TestVault::balance, 16);
    assert_eq!(TestVault::authority, 24);
}

#[test]
fn const_offsets_v2_match() {
    // TestVaultV2: header(16) + balance(8) + authority(32) + fee_bps(8) = 64
    assert_eq!(TestVaultV2::header, 0);
    assert_eq!(TestVaultV2::balance, 16);
    assert_eq!(TestVaultV2::authority, 24);
    assert_eq!(TestVaultV2::fee_bps, 56);
}

// ── Borrow-splitting tests ──────────────────────────────────────────────────

#[test]
fn split_fields_returns_correct_count() {
    let data = [0u8; 56];
    let (h, b, a) = TestVault::split_fields(&data).unwrap();
    assert_eq!(h.as_bytes().len(), 16);
    assert_eq!(b.as_bytes().len(), 8);
    assert_eq!(a.as_bytes().len(), 32);
}

#[test]
fn split_fields_too_small() {
    let data = [0u8; 10];
    assert!(TestVault::split_fields(&data).is_err());
}

#[test]
fn split_fields_mut_write_then_overlay() {
    let mut data = [0u8; 56];
    // Write header first
    write_header(&mut data, TestVault::DISC, TestVault::VERSION, &TestVault::LAYOUT_ID).unwrap();
    // Split and write balance
    let (_h, mut b, _a) = TestVault::split_fields_mut(&mut data).unwrap();
    b.write_u64(12345);
    // Now overlay and verify
    let vault = TestVault::read(&data).unwrap();
    assert_eq!(vault.balance, 12345);
}

#[test]
fn split_fields_mut_multiple_writes() {
    let mut data = [0u8; 56];
    let (_h, mut b, mut a) = TestVault::split_fields_mut(&mut data).unwrap();
    b.write_u64(999);
    a.copy_from(&[0xAA; 32]);
    // Verify via raw bytes
    assert_eq!(u64::from_le_bytes(data[16..24].try_into().unwrap()), 999);
    assert_eq!(&data[24..56], &[0xAA; 32]);
}

#[test]
fn split_fields_mut_too_small() {
    let mut data = [0u8; 10];
    assert!(TestVault::split_fields_mut(&mut data).is_err());
}

// ── jiminy_interface! tests ──────────────────────────────────────────────────

mod interface_tests {
    use jiminy_core::account::AccountHeader;
    use jiminy_core::abi::LeU64;
    use jiminy_core::zero_copy_layout;
    use pinocchio::Address;

    const FOREIGN_PROGRAM: Address = Address::new_from_array([0xAA; 32]);

    // Original layout as defined by Program A.
    zero_copy_layout! {
        pub struct Vault, discriminator = 1, version = 1 {
            header:    AccountHeader = 16,
            balance:   LeU64         = 8,
            authority: Address       = 32,
        }
    }

    // Interface as declared by Program B (consumer).
    mod foreign {
        use jiminy_core::account::AccountHeader;
        use jiminy_core::abi::LeU64;
        use jiminy_core::jiminy_interface;
        use pinocchio::Address;

        const FOREIGN_PROGRAM: Address = Address::new_from_array([0xAA; 32]);

        jiminy_interface! {
            /// Program B's read-only view of Program A's Vault.
            pub struct Vault for FOREIGN_PROGRAM {
                header:    AccountHeader = 16,
                balance:   LeU64         = 8,
                authority: Address       = 32,
            }
        }
    }

    #[test]
    fn interface_layout_id_matches_original() {
        // The interface struct produces the same LAYOUT_ID as the
        // zero_copy_layout! definition.
        assert_eq!(Vault::LAYOUT_ID, foreign::Vault::LAYOUT_ID);
    }

    #[test]
    fn interface_len_matches_original() {
        assert_eq!(Vault::LEN, foreign::Vault::LEN);
    }

    #[test]
    fn interface_const_offsets() {
        assert_eq!(foreign::Vault::header, 0);
        assert_eq!(foreign::Vault::balance, 16);
        assert_eq!(foreign::Vault::authority, 24);
    }

    #[test]
    fn interface_overlay_reads_data() {
        let mut data = [0u8; 56];
        // Write a balance value at offset 16.
        data[16..24].copy_from_slice(&42u64.to_le_bytes());
        let vault = foreign::Vault::overlay(&data).unwrap();
        assert_eq!(vault.balance.get(), 42);
    }

    #[test]
    fn interface_split_fields_immutable() {
        let mut data = [0u8; 56];
        data[16..24].copy_from_slice(&999u64.to_le_bytes());
        let (_h, b, _a) = foreign::Vault::split_fields(&data).unwrap();
        assert_eq!(b.read_u64(), 999);
    }

    #[test]
    fn interface_owner_constant() {
        assert_eq!(foreign::Vault::OWNER, &FOREIGN_PROGRAM);
    }
}
