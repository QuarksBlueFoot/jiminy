//! Explorer and indexer integration utilities.
//!
//! Provides functions to decode raw Jiminy account data using a
//! [`LayoutManifest`] without knowing the
//! account type at compile time. This is the runtime equivalent of
//! `pod_from_bytes`: useful for explorers, indexers, and monitoring tools.
//!
//! ## Example
//!
//! ```rust
//! use jiminy_schema::*;
//! use jiminy_schema::indexer::*;
//!
//! let manifest = LayoutManifest {
//!     name: "Vault",
//!     version: 1,
//!     discriminator: 1,
//!     layout_id: [0; 8],
//!     fields: &[
//!         FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
//!         FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
//!     ],
//!     segments: &[],
//! };
//!
//! let mut data = vec![0u8; 24];
//! data[0] = 1; // disc
//! data[16..24].copy_from_slice(&42u64.to_le_bytes()); // balance
//!
//! let decoded = decode_account(&manifest, &data).unwrap();
//! assert_eq!(decoded.fields[1].name, "balance");
//! if let FieldValue::U64(v) = decoded.fields[1].value {
//!     assert_eq!(v, 42);
//! }
//! ```

use crate::{CanonicalType, LayoutManifest};

/// A decoded field value from raw account data.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Bool(bool),
    /// Raw bytes (pubkey, header, or byte array).
    Bytes(Vec<u8>),
}

/// A named decoded field.
#[derive(Debug, Clone)]
pub struct DecodedField {
    pub name: &'static str,
    pub value: FieldValue,
}

/// A decoded segment descriptor from a segmented account.
#[derive(Debug, Clone)]
pub struct DecodedSegment {
    /// Segment name from the manifest.
    pub name: &'static str,
    /// Element type name from the manifest.
    pub element_type: &'static str,
    /// Number of active elements.
    pub count: u16,
    /// Maximum element capacity.
    pub capacity: u16,
    /// Size of each element in bytes.
    pub element_size: u16,
    /// Reserved flags.
    pub flags: u16,
    /// Byte offset of the segment data within the account.
    pub offset: u32,
    /// Raw element data (count × element_size bytes).
    pub data: Vec<u8>,
}

/// A fully decoded account.
#[derive(Debug, Clone)]
pub struct DecodedAccount {
    pub layout_name: &'static str,
    pub version: u8,
    pub discriminator: u8,
    pub fields: Vec<DecodedField>,
    /// Decoded segments (empty for non-segmented accounts).
    pub segments: Vec<DecodedSegment>,
}

/// Decode raw account data using a layout manifest.
///
/// Returns `None` if the data is too short for the manifest's total size.
pub fn decode_account(manifest: &LayoutManifest, data: &[u8]) -> Option<DecodedAccount> {
    if data.len() < manifest.min_size() {
        return None;
    }

    let mut fields = Vec::with_capacity(manifest.fields.len());
    let mut offset = 0;

    for field in manifest.fields {
        let end = offset + field.size;
        let slice = &data[offset..end];

        let value = match field.canonical_type {
            CanonicalType::U8 => FieldValue::U8(slice[0]),
            CanonicalType::U16 => FieldValue::U16(u16::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::U32 => FieldValue::U32(u32::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::U64 => FieldValue::U64(u64::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::U128 => FieldValue::U128(u128::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::I8 => FieldValue::I8(slice[0] as i8),
            CanonicalType::I16 => FieldValue::I16(i16::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::I32 => FieldValue::I32(i32::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::I64 => FieldValue::I64(i64::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::I128 => FieldValue::I128(i128::from_le_bytes(slice.try_into().ok()?)),
            CanonicalType::Bool => FieldValue::Bool(slice[0] != 0),
            CanonicalType::Pubkey | CanonicalType::Header | CanonicalType::Bytes(_) => {
                FieldValue::Bytes(slice.to_vec())
            }
        };

        fields.push(DecodedField {
            name: field.name,
            value,
        });
        offset = end;
    }

    let mut segments = Vec::new();

    // Decode segments if the manifest declares any.
    // Segment table sits immediately after the fixed fields.
    if !manifest.segments.is_empty() {
        let table_offset = offset; // end of fixed fields
        let seg_count = manifest.segments.len();
        let table_end = table_offset + seg_count * 12; // 12 bytes per descriptor

        if data.len() >= table_end {
            for (i, seg_desc) in manifest.segments.iter().enumerate() {
                let desc_start = table_offset + i * 12;
                let desc_bytes = &data[desc_start..desc_start + 12];

                let seg_offset = u32::from_le_bytes(desc_bytes[0..4].try_into().ok()?);
                let count = u16::from_le_bytes(desc_bytes[4..6].try_into().ok()?);
                let capacity = u16::from_le_bytes(desc_bytes[6..8].try_into().ok()?);
                let element_size = u16::from_le_bytes(desc_bytes[8..10].try_into().ok()?);
                let flags = u16::from_le_bytes(desc_bytes[10..12].try_into().ok()?);

                let data_start = seg_offset as usize;
                let data_end = data_start + (count as usize) * (element_size as usize);

                let seg_data = if data_end <= data.len() {
                    data[data_start..data_end].to_vec()
                } else {
                    Vec::new() // truncated data - return empty
                };

                segments.push(DecodedSegment {
                    name: seg_desc.name,
                    element_type: seg_desc.element_type,
                    count,
                    capacity,
                    element_size,
                    flags,
                    offset: seg_offset,
                    data: seg_data,
                });
            }
        }
    }

    Some(DecodedAccount {
        layout_name: manifest.name,
        version: manifest.version,
        discriminator: manifest.discriminator,
        fields,
        segments,
    })
}

/// Check if raw account data matches a manifest's discriminator and layout_id.
///
/// Quick pre-check before full decoding. Useful for filtering accounts
/// in bulk indexing operations.
pub fn matches_manifest(manifest: &LayoutManifest, data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    // Check discriminator (byte 0)
    if data[0] != manifest.discriminator {
        return false;
    }
    // Check layout_id (bytes 4..12)
    data[4..12] == manifest.layout_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FieldDescriptor;

    fn test_manifest() -> LayoutManifest {
        LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0xAA; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
                FieldDescriptor { name: "authority", canonical_type: CanonicalType::Pubkey, size: 32 },
            ],
            segments: &[],
        }
    }

    #[test]
    fn decode_account_basic() {
        let manifest = test_manifest();
        let mut data = vec![0u8; 56];
        data[0] = 1; // disc
        data[1] = 1; // version
        data[16..24].copy_from_slice(&999u64.to_le_bytes());
        data[24..56].copy_from_slice(&[0xBB; 32]);

        let decoded = decode_account(&manifest, &data).unwrap();
        assert_eq!(decoded.layout_name, "Vault");
        assert_eq!(decoded.fields.len(), 3);
        assert_eq!(decoded.fields[1].name, "balance");
        assert_eq!(decoded.fields[1].value, FieldValue::U64(999));
        assert_eq!(decoded.fields[2].name, "authority");
    }

    #[test]
    fn decode_rejects_short_data() {
        let manifest = test_manifest();
        let data = vec![0u8; 10]; // too short
        assert!(decode_account(&manifest, &data).is_none());
    }

    #[test]
    fn matches_manifest_checks_disc_and_layout_id() {
        let manifest = test_manifest();
        let mut data = vec![0u8; 16];
        data[0] = 1; // correct disc
        data[4..12].copy_from_slice(&[0xAA; 8]); // correct layout_id
        assert!(matches_manifest(&manifest, &data));

        data[0] = 2; // wrong disc
        assert!(!matches_manifest(&manifest, &data));

        data[0] = 1; // correct disc
        data[4] = 0; // wrong layout_id
        assert!(!matches_manifest(&manifest, &data));
    }

    #[test]
    fn matches_rejects_short_data() {
        let manifest = test_manifest();
        assert!(!matches_manifest(&manifest, &[0u8; 4]));
    }

    // ── Segment decoding tests ───────────────────────────────────────────

    fn segmented_manifest() -> LayoutManifest {
        use crate::SegmentFieldDescriptor;
        LayoutManifest {
            name: "OrderBook",
            version: 1,
            discriminator: 2,
            layout_id: [0xBB; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "counter", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "bids", element_type: "Order", element_size: 16 },
                SegmentFieldDescriptor { name: "asks", element_type: "Order", element_size: 16 },
            ],
        }
    }

    #[test]
    fn decode_segments_basic() {
        let manifest = segmented_manifest();
        // Fixed: header(16) + counter(8) = 24 bytes
        // Segment table: 2 descriptors × 12 = 24 bytes (at offset 24)
        // Data starts at offset 48
        let mut data = vec![0u8; 112]; // 24 + 24 + 64 (4 elements total)
        data[0] = 2; // disc
        data[1] = 1; // version
        data[16..24].copy_from_slice(&42u64.to_le_bytes()); // counter

        // Segment 0 (bids): offset=48, count=2, capacity=4, element_size=16, flags=0
        data[24..28].copy_from_slice(&48u32.to_le_bytes());
        data[28..30].copy_from_slice(&2u16.to_le_bytes());
        data[30..32].copy_from_slice(&4u16.to_le_bytes());
        data[32..34].copy_from_slice(&16u16.to_le_bytes());
        data[34..36].copy_from_slice(&0u16.to_le_bytes());

        // Segment 1 (asks): offset=80, count=2, capacity=4, element_size=16, flags=0
        data[36..40].copy_from_slice(&80u32.to_le_bytes());
        data[40..42].copy_from_slice(&2u16.to_le_bytes());
        data[42..44].copy_from_slice(&4u16.to_le_bytes());
        data[44..46].copy_from_slice(&16u16.to_le_bytes());
        data[46..48].copy_from_slice(&0u16.to_le_bytes());

        // Write some element data
        data[48] = 0xAA; // first bid
        data[80] = 0xCC; // first ask

        let decoded = decode_account(&manifest, &data).unwrap();
        assert_eq!(decoded.segments.len(), 2);
        assert_eq!(decoded.segments[0].name, "bids");
        assert_eq!(decoded.segments[0].count, 2);
        assert_eq!(decoded.segments[0].capacity, 4);
        assert_eq!(decoded.segments[0].element_size, 16);
        assert_eq!(decoded.segments[0].flags, 0);
        assert_eq!(decoded.segments[0].data.len(), 32);
        assert_eq!(decoded.segments[0].data[0], 0xAA);
        assert_eq!(decoded.segments[1].name, "asks");
        assert_eq!(decoded.segments[1].data[0], 0xCC);
    }

    #[test]
    fn decode_non_segmented_has_empty_segments() {
        let manifest = test_manifest();
        let data = vec![0u8; 56];
        let decoded = decode_account(&manifest, &data).unwrap();
        assert!(decoded.segments.is_empty());
    }

    #[test]
    fn decode_segments_truncated_data_returns_empty_vec() {
        let manifest = segmented_manifest();
        // Enough for fixed fields + segment table, but not for segment data
        let mut data = vec![0u8; 48]; // 24 fixed + 24 table (2 × 12)
        data[0] = 2;
        // Segment 0: offset=100, count=2, capacity=4, elem_size=16, flags=0
        data[24..28].copy_from_slice(&100u32.to_le_bytes());
        data[28..30].copy_from_slice(&2u16.to_le_bytes());
        data[30..32].copy_from_slice(&4u16.to_le_bytes());
        data[32..34].copy_from_slice(&16u16.to_le_bytes());
        data[34..36].copy_from_slice(&0u16.to_le_bytes());
        // Segment 1: offset=200, count=1, capacity=4, elem_size=16, flags=0
        data[36..40].copy_from_slice(&200u32.to_le_bytes());
        data[40..42].copy_from_slice(&1u16.to_le_bytes());
        data[42..44].copy_from_slice(&4u16.to_le_bytes());
        data[44..46].copy_from_slice(&16u16.to_le_bytes());
        data[46..48].copy_from_slice(&0u16.to_le_bytes());

        let decoded = decode_account(&manifest, &data).unwrap();
        assert_eq!(decoded.segments.len(), 2);
        assert!(decoded.segments[0].data.is_empty());
        assert!(decoded.segments[1].data.is_empty());
    }
}
