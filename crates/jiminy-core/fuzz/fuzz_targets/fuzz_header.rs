//! Fuzz target: header validation.
//!
//! Exercises `check_header`, `read_version`, `read_layout_id`,
//! `AccountHeader::from_bytes` with arbitrary byte slices.
//! Must never panic or produce UB.

#![no_main]

use libfuzzer_sys::fuzz_target;
use jiminy_core::account::header;

fuzz_target!(|data: &[u8]| {
    // Try reading individual header fields — none should panic.
    let _ = header::read_version(data);
    let _ = header::read_layout_id(data);
    let _ = header::read_header_flags(data);
    let _ = header::check_layout_id(data, &[0u8; 8]);
    let _ = header::header_payload(data);
    let _ = header::body(data);

    // Structured header read.
    let _ = header::AccountHeader::from_bytes(data);

    // Full header check against arbitrary expected values.
    let _ = header::check_header(data, 1, 1, &[0u8; 8]);

    // Write header to a mutable copy (if long enough).
    if data.len() >= 16 {
        let mut buf = data.to_vec();
        let _ = header::write_header(&mut buf, 1, 1, &[0u8; 8]);
    }
});
