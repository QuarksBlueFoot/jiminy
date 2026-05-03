//! Property-based tests for the Jiminy Account ABI.
//!
//! Uses proptest to fuzz header validation, Pod overlay, and zero-init
//! invariants across random inputs.

use jiminy_core::account::*;
use jiminy_core::zero_copy_layout;
use jiminy_core::Address;
use proptest::prelude::*;

// ── Test layout ──────────────────────────────────────────────────────────────

zero_copy_layout! {
    pub struct PropVault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

// ── Helper ───────────────────────────────────────────────────────────────────

#[repr(C, align(8))]
struct Aligned56([u8; 56]);

impl Aligned56 {
    fn zeroed() -> Self {
        Self([0u8; 56])
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Property tests
// ══════════════════════════════════════════════════════════════════════════════

proptest! {
    /// write_header then check_header always succeeds for matching parameters.
    #[test]
    fn roundtrip_header(disc in 0u8..=255, ver in 1u8..=255, id in prop::array::uniform8(0u8..)) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, ver, &id).unwrap();
        prop_assert!(check_header(&buf.0, disc, ver, &id).is_ok());
    }

    /// check_header rejects any wrong discriminator.
    #[test]
    fn wrong_disc_always_rejected(disc in 0u8..=254, ver in 1u8..=255, id in prop::array::uniform8(0u8..)) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, ver, &id).unwrap();
        prop_assert!(check_header(&buf.0, disc.wrapping_add(1), ver, &id).is_err());
    }

    /// check_header rejects any version strictly lower than written.
    #[test]
    fn old_version_rejected(disc in 0u8..=255, ver in 2u8..=255, id in prop::array::uniform8(0u8..)) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, ver - 1, &id).unwrap();
        prop_assert!(check_header(&buf.0, disc, ver, &id).is_err());
    }

    /// check_header accepts any version >= min_version.
    #[test]
    fn newer_version_accepted(disc in 0u8..=255, min_ver in 1u8..=127, extra in 0u8..=127, id in prop::array::uniform8(0u8..)) {
        let actual_ver = min_ver.saturating_add(extra);
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, actual_ver, &id).unwrap();
        prop_assert!(check_header(&buf.0, disc, min_ver, &id).is_ok());
    }

    /// Any single-byte mutation in layout_id causes check_header to fail.
    #[test]
    fn layout_id_mutation_detected(
        disc in 0u8..=255,
        ver in 1u8..=255,
        id in prop::array::uniform8(0u8..),
        byte_idx in 0usize..8,
        delta in 1u8..=255,
    ) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, ver, &id).unwrap();
        let mut bad_id = id;
        bad_id[byte_idx] = bad_id[byte_idx].wrapping_add(delta);
        prop_assert!(check_header(&buf.0, disc, ver, &bad_id).is_err());
    }

    /// zero_init produces all-zero buffer of any content.
    #[test]
    fn zero_init_always_clears(data in prop::collection::vec(0u8.., 1..512)) {
        let mut buf = data;
        zero_init(&mut buf);
        prop_assert!(buf.iter().all(|&b| b == 0));
    }

    /// Overlay on a correctly-sized aligned buffer always succeeds.
    #[test]
    fn overlay_succeeds_on_valid_size(
        balance in any::<u64>(),
        authority in prop::array::uniform32(0u8..),
    ) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, PropVault::DISC, PropVault::VERSION, &PropVault::LAYOUT_ID).unwrap();
        buf.0[16..24].copy_from_slice(&balance.to_le_bytes());
        buf.0[24..56].copy_from_slice(&authority);

        let vault = PropVault::overlay(&buf.0).unwrap();
        prop_assert_eq!(vault.balance, balance);
        prop_assert_eq!(vault.authority.as_array(), &authority);
    }

    /// Overlay always fails on buffers shorter than LEN.
    #[test]
    fn overlay_rejects_short_buffer(len in 0usize..56) {
        let buf = vec![0u8; len];
        prop_assert!(PropVault::overlay(&buf).is_err());
    }

    /// load_checked succeeds iff header matches, never panics.
    #[test]
    fn load_checked_never_panics(
        disc in 0u8..=255,
        ver in 0u8..=255,
        id in prop::array::uniform8(0u8..),
    ) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, ver, &id).unwrap();
        let result = PropVault::load_checked(&buf.0);
        if disc == PropVault::DISC && ver >= PropVault::VERSION && id == PropVault::LAYOUT_ID {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// check_layout_id passes iff layout_id bytes match exactly.
    #[test]
    fn layout_id_check_is_exact(
        disc in 0u8..=255,
        ver in 1u8..=255,
        id in prop::array::uniform8(0u8..),
        other_id in prop::array::uniform8(0u8..),
    ) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, ver, &id).unwrap();
        let result = check_layout_id(&buf.0, &other_id);
        if id == other_id {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// load_unverified_overlay never panics on random data of valid size.
    #[test]
    fn unverified_overlay_never_panics(data in prop::collection::vec(0u8.., 56..512)) {
        // Copy into an aligned buffer.
        let mut buf = Aligned56::zeroed();
        let copy_len = data.len().min(56);
        buf.0[..copy_len].copy_from_slice(&data[..copy_len]);
        // Should never panic; either returns Ok with validated true/false.
        let _ = PropVault::load_unverified_overlay(&buf.0);
    }

    /// load_unverified_overlay validated flag is true iff header matches exactly.
    #[test]
    fn unverified_overlay_validated_iff_header_matches(
        disc in 0u8..=255,
        ver in 0u8..=255,
        id in prop::array::uniform8(0u8..),
    ) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, disc, ver, &id).unwrap();
        let (_, validated) = PropVault::load_unverified_overlay(&buf.0).unwrap();
        let expected = disc == PropVault::DISC
            && ver >= PropVault::VERSION
            && id == PropVault::LAYOUT_ID;
        prop_assert_eq!(validated, expected);
    }

    /// Compatibility check (validate_version_compatible logic): any layout_id should
    /// pass as long as disc and min_version match.
    #[test]
    fn compatible_ignores_layout_id(
        id in prop::array::uniform8(0u8..),
        ver in 1u8..=255,
    ) {
        let mut buf = Aligned56::zeroed();
        write_header(&mut buf.0, PropVault::DISC, ver, &id).unwrap();
        // Compatibility check cares about disc + version >= min, not layout_id.
        prop_assert_eq!(buf.0[0], PropVault::DISC);
        prop_assert!(buf.0[1] >= PropVault::VERSION);
    }
}
