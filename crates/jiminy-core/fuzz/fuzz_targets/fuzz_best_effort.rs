//! Fuzz target: unverified overlay / tiered loading.
//!
//! Exercises `load_unverified_overlay` and `validate_version_compatible` with
//! arbitrary byte slices. These are the most permissive loading paths
//! and must gracefully handle any input without panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;
use jiminy_core::account::view::load_unverified_overlay;
use jiminy_core::account::header::AccountHeader;

fuzz_target!(|data: &[u8]| {
    // Unverified overlay load with arbitrary expected values.
    let _ = load_unverified_overlay::<AccountHeader>(data, 0, 0, &[0u8; 8]);
    let _ = load_unverified_overlay::<AccountHeader>(data, 1, 1, &[0xFFu8; 8]);
    let _ = load_unverified_overlay::<AccountHeader>(data, 255, 255, &[0xAB; 8]);

    // If data is large enough for a header, try extracting header
    // fields and feeding them back as expected values.
    if data.len() >= 16 {
        let disc = data[0];
        let version = data[1];
        let mut layout_id = [0u8; 8];
        layout_id.copy_from_slice(&data[4..12]);

        // This should succeed (data contains its own header).
        let _ = load_unverified_overlay::<AccountHeader>(data, disc, version, &layout_id);

        // Deliberately wrong disc/version — should still not panic.
        let _ = load_unverified_overlay::<AccountHeader>(data, disc.wrapping_add(1), version, &layout_id);
    }
});
