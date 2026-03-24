//! Fuzz target: zero-copy overlay.
//!
//! Exercises `pod_from_bytes` and `pod_from_bytes_mut` with arbitrary
//! byte slices. Must never panic or produce UB — only return `Err`.

#![no_main]

use libfuzzer_sys::fuzz_target;
use jiminy_core::account::pod::{pod_from_bytes, pod_from_bytes_mut};
use jiminy_core::account::header::AccountHeader;

fuzz_target!(|data: &[u8]| {
    // Attempt to overlay AccountHeader onto random bytes.
    let _ = pod_from_bytes::<AccountHeader>(data);

    // Attempt mutable overlay (on a copy).
    let mut buf = data.to_vec();
    let _ = pod_from_bytes_mut::<AccountHeader>(&mut buf);
});
