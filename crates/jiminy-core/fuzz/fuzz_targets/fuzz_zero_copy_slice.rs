//! Fuzz target: ZeroCopySlice parsing.
//!
//! Exercises `ZeroCopySlice::from_bytes` and `ZeroCopySliceMut::from_bytes`
//! with arbitrary byte slices, exercising length parsing, bounds checking,
//! and element access. Must never panic or produce UB.

#![no_main]

use libfuzzer_sys::fuzz_target;
use jiminy_core::account::collection::{ZeroCopySlice, ZeroCopySliceMut};
use jiminy_core::account::header::AccountHeader;

fuzz_target!(|data: &[u8]| {
    // Try parsing as a slice of AccountHeader (16-byte elements).
    if let Ok(slice) = ZeroCopySlice::<AccountHeader>::from_bytes(data) {
        let len = slice.len();
        // Read every element — must not panic.
        for i in 0..len {
            let _ = slice.get(i);
            let _ = slice.read(i);
        }
        // Out-of-bounds must return Err, not panic.
        let _ = slice.get(len);
        let _ = slice.read(len);

        // Iteration must not panic.
        for _item in slice.iter() {}

        let _ = slice.is_empty();
        let _ = slice.byte_len();

        // contains_bytes with various patterns.
        let _ = slice.contains_bytes(&[0u8; 0]);
        let _ = slice.contains_bytes(&[0u8; 1]);
        let _ = slice.contains_bytes(&[0xFF; 8]);
    }

    // Mutable variant (on a copy).
    let mut buf = data.to_vec();
    if let Ok(mut slice_mut) = ZeroCopySliceMut::<AccountHeader>::from_bytes(&mut buf) {
        let len = slice_mut.len();
        // Read and write every element.
        for i in 0..len {
            let _ = slice_mut.get(i);
            let _ = slice_mut.get_mut(i);
        }
        // Out-of-bounds must return Err.
        let _ = slice_mut.get(len);
        let _ = slice_mut.get_mut(len);

        // Set with a zeroed value.
        if len > 0 {
            let zero = AccountHeader {
                discriminator: 0,
                version: 0,
                flags: 0,
                reserved: [0u8; 4],
                layout_id: [0u8; 8],
            };
            let _ = slice_mut.set(0, &zero);
            let _ = slice_mut.set(len, &zero); // out-of-bounds
        }
    }
});
