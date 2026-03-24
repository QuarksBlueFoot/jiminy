//! End-to-end integration test for the manifest → JSON → verify → decode lifecycle.
//!
//! Exercises the full pipeline: build a segmented manifest, verify it,
//! export JSON, construct synthetic account bytes, verify the account,
//! and decode fields + segments via the indexer.

use jiminy_schema::indexer::{decode_account, matches_manifest, FieldValue};
use jiminy_schema::*;

fn order_book_manifest() -> LayoutManifest {
    LayoutManifest {
        name: "OrderBook",
        version: 1,
        discriminator: 5,
        layout_id: [0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44],
        fields: &[
            FieldDescriptor {
                name: "header",
                canonical_type: CanonicalType::Header,
                size: 16,
            },
            FieldDescriptor {
                name: "market",
                canonical_type: CanonicalType::Pubkey,
                size: 32,
            },
            FieldDescriptor {
                name: "counter",
                canonical_type: CanonicalType::U64,
                size: 8,
            },
        ],
        segments: &[
            SegmentFieldDescriptor {
                name: "bids",
                element_type: "Order",
                element_size: 16,
            },
            SegmentFieldDescriptor {
                name: "asks",
                element_type: "Order",
                element_size: 16,
            },
        ],
    }
}

/// Build synthetic account bytes for an OrderBook.
///
/// Layout:
///   [0..16]   header (disc=5, ver=1, flags=0, layout_id, reserved)
///   [16..48]  market (pubkey)
///   [48..56]  counter (u64 LE)
///   [56..80]  segment table (2 × 12 bytes)
///   [80..112] bids data (2 elements × 16 bytes)
///   [112..144] asks data (2 elements × 16 bytes)
fn build_account_data(manifest: &LayoutManifest) -> Vec<u8> {
    let total = 144; // fixed(56) + table(24) + bids(32) + asks(32)
    let mut data = vec![0u8; total];

    // Header
    data[0] = manifest.discriminator; // disc
    data[1] = manifest.version; // version
    // flags = 0
    data[4..12].copy_from_slice(&manifest.layout_id);
    // reserved = 0

    // market (pubkey)
    data[16..48].copy_from_slice(&[0xDD; 32]);

    // counter
    data[48..56].copy_from_slice(&777u64.to_le_bytes());

    // Segment table at offset 56
    // Segment 0 (bids): offset=80, count=2, capacity=2, element_size=16, flags=0
    data[56..60].copy_from_slice(&80u32.to_le_bytes());
    data[60..62].copy_from_slice(&2u16.to_le_bytes());
    data[62..64].copy_from_slice(&2u16.to_le_bytes());
    data[64..66].copy_from_slice(&16u16.to_le_bytes());
    data[66..68].copy_from_slice(&0u16.to_le_bytes());

    // Segment 1 (asks): offset=112, count=2, capacity=2, element_size=16, flags=0
    data[68..72].copy_from_slice(&112u32.to_le_bytes());
    data[72..74].copy_from_slice(&2u16.to_le_bytes());
    data[74..76].copy_from_slice(&2u16.to_le_bytes());
    data[76..78].copy_from_slice(&16u16.to_le_bytes());
    data[78..80].copy_from_slice(&0u16.to_le_bytes());

    // Bid data: first bid has marker 0xBB, second has 0xB2
    data[80] = 0xBB;
    data[96] = 0xB2;

    // Ask data: first ask has marker 0xAA, second has 0xA2
    data[112] = 0xAA;
    data[128] = 0xA2;

    data
}

#[test]
fn full_lifecycle_segmented_manifest() {
    let manifest = order_book_manifest();

    // Step 1: Verify manifest structural invariants
    assert!(manifest.verify().is_ok(), "manifest verification failed");

    // Step 2: Check size calculations
    assert_eq!(manifest.total_size(), 56); // 16 + 32 + 8
    assert_eq!(manifest.min_size(), 80); // 56 + 2 * 12

    // Step 3: Export JSON and verify it contains all expected fields
    let json = manifest.export_json();
    assert!(json.contains("\"name\": \"OrderBook\""));
    assert!(json.contains("\"total_size\": 56"));
    assert!(json.contains("\"min_size\": 80"));
    assert!(json.contains("\"segments\":"));
    assert!(json.contains("\"name\": \"bids\""));
    assert!(json.contains("\"name\": \"asks\""));
    assert!(json.contains("\"element_type\": \"Order\""));
    assert!(json.contains("\"element_size\": 16"));

    // Step 4: Build synthetic account data
    let data = build_account_data(&manifest);
    assert_eq!(data.len(), 144);

    // Step 5: Verify account data matches manifest
    assert!(
        manifest.verify_account(&data).is_ok(),
        "account verification failed"
    );

    // Step 6: Quick match check
    assert!(matches_manifest(&manifest, &data));

    // Step 7: Full decode
    let decoded = decode_account(&manifest, &data).unwrap();

    // Check metadata
    assert_eq!(decoded.layout_name, "OrderBook");
    assert_eq!(decoded.discriminator, 5);
    assert_eq!(decoded.version, 1);

    // Check fixed fields
    assert_eq!(decoded.fields.len(), 3);
    assert_eq!(decoded.fields[0].name, "header");
    assert_eq!(decoded.fields[1].name, "market");
    if let FieldValue::Bytes(ref b) = decoded.fields[1].value {
        assert_eq!(b, &[0xDD; 32]);
    } else {
        panic!("expected Bytes for market");
    }
    assert_eq!(decoded.fields[2].name, "counter");
    assert_eq!(decoded.fields[2].value, FieldValue::U64(777));

    // Check segments
    assert_eq!(decoded.segments.len(), 2);

    let bids = &decoded.segments[0];
    assert_eq!(bids.name, "bids");
    assert_eq!(bids.element_type, "Order");
    assert_eq!(bids.count, 2);
    assert_eq!(bids.capacity, 2);
    assert_eq!(bids.element_size, 16);
    assert_eq!(bids.flags, 0);
    assert_eq!(bids.offset, 80);
    assert_eq!(bids.data.len(), 32); // 2 × 16
    assert_eq!(bids.data[0], 0xBB);
    assert_eq!(bids.data[16], 0xB2);

    let asks = &decoded.segments[1];
    assert_eq!(asks.name, "asks");
    assert_eq!(asks.count, 2);
    assert_eq!(asks.capacity, 2);
    assert_eq!(asks.data.len(), 32);
    assert_eq!(asks.data[0], 0xAA);
    assert_eq!(asks.data[16], 0xA2);
}

#[test]
fn hash_input_roundtrip() {
    let manifest = order_book_manifest();
    let input = manifest.hash_input();

    // Must contain all field declarations in order
    assert!(input.starts_with("jiminy:v1:OrderBook:1:"));
    assert!(input.contains("header:header:16,"));
    assert!(input.contains("market:pubkey:32,"));
    assert!(input.contains("counter:u64:8,"));
    assert!(input.contains("seg:bids:Order:16,"));
    assert!(input.contains("seg:asks:Order:16,"));
}

#[test]
fn verify_account_rejects_undersized_segmented() {
    let manifest = order_book_manifest();

    // Data large enough for fixed fields (56) but not for table (80)
    let mut data = vec![0u8; 60];
    data[0] = manifest.discriminator;
    data[4..12].copy_from_slice(&manifest.layout_id);

    assert!(manifest.verify_account(&data).is_err());
}

#[test]
fn decode_rejects_undersized_segmented() {
    let manifest = order_book_manifest();

    // Only enough for fixed fields, not the table
    let mut data = vec![0u8; 60];
    data[0] = manifest.discriminator;
    data[4..12].copy_from_slice(&manifest.layout_id);

    assert!(decode_account(&manifest, &data).is_none());
}
