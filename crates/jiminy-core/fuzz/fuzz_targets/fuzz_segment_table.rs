//! Fuzz target: segment table parsing.
//!
//! Exercises `SegmentTable::from_bytes`, `descriptor()`, and `validate()`
//! with arbitrary byte slices. Must never panic or produce UB.

#![no_main]

use libfuzzer_sys::fuzz_target;
use jiminy_core::account::segment::{SegmentTable, SegmentTableMut, SEGMENT_DESC_SIZE};

fuzz_target!(|data: &[u8]| {
    // Try parsing segment tables of various sizes.
    for seg_count in 0..=4 {
        let required = seg_count * SEGMENT_DESC_SIZE;
        if data.len() < required {
            continue;
        }

        // Immutable table parse.
        if let Ok(table) = SegmentTable::from_bytes(data, seg_count) {
            // Try reading each descriptor.
            for i in 0..seg_count {
                let _ = table.descriptor(i);
            }
            // Out-of-bounds index should return Err, not panic.
            let _ = table.descriptor(seg_count);

            // Validate with various data lengths and element sizes.
            let sizes: Vec<u16> = (0..seg_count).map(|_| 16u16).collect();
            let _ = table.validate(data.len(), &sizes, required);
        }

        // Mutable table parse (on a copy).
        let mut buf = data.to_vec();
        if let Ok(_table_mut) = SegmentTableMut::from_bytes(&mut buf, seg_count) {
            // Just verify it doesn't panic.
        }
    }
});
