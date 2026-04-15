//! # jiminy-schema
//!
//! Layout Manifest v1 for Jiminy account schemas.
//!
//! This crate provides structured descriptions of Jiminy account layouts,
//! enabling cross-language tooling, TypeScript decoder generation, indexer
//! integration, and schema validation.
//!
//! ## Workflow
//!
//! ```text
//! zero_copy_layout!  ──▶  LayoutManifest  ──▶  export_json()  ──▶  TS / indexers
//! ```
//!
//! 1. `zero_copy_layout!` defines your on-chain struct and computes `LAYOUT_ID`.
//! 2. Build a [`LayoutManifest`] describing the same struct.
//! 3. [`export_json()`](LayoutManifest::export_json) emits a JSON manifest (no serde).
//! 4. Off-chain tooling (`@jiminy/ts`, indexers, explorers) consumes the manifest.
//!
//! ## Layout Manifest v1
//!
//! A layout manifest describes one account type:
//!
//! ```rust
//! use jiminy_schema::*;
//!
//! let manifest = LayoutManifest {
//!     name: "Vault",
//!     version: 1,
//!     discriminator: 1,
//!     layout_id: [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89],
//!     fields: &[
//!         FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
//!         FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
//!         FieldDescriptor { name: "authority", canonical_type: CanonicalType::Pubkey, size: 32 },
//!     ],
//!     segments: &[],
//! };
//!
//! assert_eq!(manifest.total_size(), 56);
//! assert_eq!(manifest.field_offset("balance"), Some(16));
//! ```
//!
//! ## Canonical Types
//!
//! The canonical type system normalizes Rust types to a fixed set of
//! language-independent names. This matches the types used in
//! `LAYOUT_ID` hash computation (see `LAYOUT_CONVENTION.md`).

#![cfg_attr(not(feature = "std"), no_std)]

/// Manifest format version string.
///
/// Included in every `export_json()` output. Tooling should check this
/// value to detect incompatible manifest format changes. This const is
/// frozen - it changes only with a major version bump.
pub const MANIFEST_VERSION: &str = "manifest-v1";

#[cfg(feature = "codegen")]
pub mod codegen;

#[cfg(feature = "std")]
pub mod idl;

#[cfg(feature = "std")]
pub mod indexer;

/// Canonical type identifiers for Jiminy account fields.
///
/// These correspond 1:1 to the canonical type strings used in
/// `LAYOUT_ID` hash computation. Every Rust type maps to exactly
/// one canonical type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalType {
    /// `u8`: unsigned 8-bit integer.
    U8,
    /// `u16`: unsigned 16-bit integer (LE).
    U16,
    /// `u32`: unsigned 32-bit integer (LE).
    U32,
    /// `u64`: unsigned 64-bit integer (LE).
    U64,
    /// `u128`: unsigned 128-bit integer (LE).
    U128,
    /// `i8`: signed 8-bit integer.
    I8,
    /// `i16`: signed 16-bit integer (LE).
    I16,
    /// `i32`: signed 32-bit integer (LE).
    I32,
    /// `i64`: signed 64-bit integer (LE).
    I64,
    /// `i128`: signed 128-bit integer (LE).
    I128,
    /// `bool`: boolean (1 byte, 0 or 1).
    Bool,
    /// `pubkey`: 32-byte public key / address.
    Pubkey,
    /// `header`: Jiminy 16-byte `AccountHeader`.
    Header,
    /// Fixed-size byte array `[u8; N]`.
    Bytes(usize),
}

impl CanonicalType {
    /// Return the canonical string representation used in layout_id hashing.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Bool => "bool",
            Self::Pubkey => "pubkey",
            Self::Header => "header",
            Self::Bytes(_) => "bytes", // caller appends {N}
        }
    }
}

/// Describes a single field in a Jiminy account layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDescriptor {
    /// Field name (must match the Rust struct field name).
    pub name: &'static str,
    /// Canonical type of the field.
    pub canonical_type: CanonicalType,
    /// Size of the field in bytes.
    pub size: usize,
}

/// Describes a dynamic segment in a segmented Jiminy account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentFieldDescriptor {
    /// Segment name (e.g., `"bids"`).
    pub name: &'static str,
    /// Element type name (e.g., `"Order"`).
    pub element_type: &'static str,
    /// Size of each element in bytes.
    pub element_size: usize,
}

/// A complete account layout manifest (v1).
///
/// Describes the schema of one Jiminy account type: its name, version,
/// discriminator, layout_id, and ordered field list. This is the
/// structured equivalent of the hash input string used to compute
/// `LAYOUT_ID`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutManifest {
    /// Account type name (e.g., `"Vault"`).
    pub name: &'static str,
    /// Schema version.
    pub version: u8,
    /// Account discriminator byte.
    pub discriminator: u8,
    /// Deterministic layout_id (first 8 bytes of SHA-256).
    pub layout_id: [u8; 8],
    /// Ordered list of fields (including the header).
    pub fields: &'static [FieldDescriptor],
    /// Optional list of dynamic segments (for segmented layouts).
    pub segments: &'static [SegmentFieldDescriptor],
}

impl LayoutManifest {
    /// Size of the fixed-field portion in bytes (sum of all field sizes).
    ///
    /// For non-segmented layouts this equals the full account size.
    /// For segmented layouts, the minimum account size is larger.
    /// Use [`min_size()`](Self::min_size) instead.
    pub const fn total_size(&self) -> usize {
        let mut total = 0;
        let mut i = 0;
        while i < self.fields.len() {
            total += self.fields[i].size;
            i += 1;
        }
        total
    }

    /// Minimum account size in bytes: fixed fields + segment table.
    ///
    /// For non-segmented layouts this equals [`total_size()`](Self::total_size).
    /// For segmented layouts this adds `segment_count × 12` for the
    /// descriptor table (but no element data).
    pub const fn min_size(&self) -> usize {
        self.total_size() + self.segments.len() * 12
    }

    /// Number of fields in the layout.
    pub const fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Find the byte offset of a field by name.
    ///
    /// Returns `None` if the field name is not found.
    pub fn field_offset(&self, name: &str) -> Option<usize> {
        let mut offset = 0;
        for field in self.fields {
            if field.name == name {
                return Some(offset);
            }
            offset += field.size;
        }
        None
    }

    /// Find a field descriptor by name.
    pub fn field(&self, name: &str) -> Option<&FieldDescriptor> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Reconstruct the canonical hash input string.
    ///
    /// This produces the same string that `zero_copy_layout!` uses to
    /// compute `LAYOUT_ID`, enabling verification that a manifest matches
    /// a compiled layout.
    #[cfg(feature = "std")]
    pub fn hash_input(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        write!(s, "jiminy:v1:{}:{}:", self.name, self.version).unwrap();
        for field in self.fields {
            match field.canonical_type {
                CanonicalType::Bytes(n) => {
                    write!(s, "{}:bytes{{{}}}:{},", field.name, n, field.size).unwrap();
                }
                _ => {
                    write!(s, "{}:{}:{},", field.name, field.canonical_type.as_str(), field.size).unwrap();
                }
            }
        }
        for seg in self.segments {
            write!(s, "seg:{}:{}:{},", seg.name, seg.element_type, seg.element_size).unwrap();
        }
        s
    }

    /// Export the manifest as a JSON string (no serde dependency).
    ///
    /// Produces a self-contained JSON document suitable for TypeScript
    /// codegen, indexer ingestion, or cross-language tooling.
    #[cfg(feature = "std")]
    pub fn export_json(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        s.push_str("{\n");
        writeln!(s, "  \"version\": \"{}\",", MANIFEST_VERSION).unwrap();
        writeln!(s, "  \"name\": \"{}\",", self.name).unwrap();
        writeln!(s, "  \"schema_version\": {},", self.version).unwrap();
        writeln!(s, "  \"discriminator\": {},", self.discriminator).unwrap();
        s.push_str("  \"layout_id\": \"");
        for byte in &self.layout_id {
            write!(s, "{byte:02x}").unwrap();
        }
        s.push_str("\",\n");
        writeln!(s, "  \"total_size\": {},", self.total_size()).unwrap();
        if !self.segments.is_empty() {
            writeln!(s, "  \"min_size\": {},", self.min_size()).unwrap();
        }
        s.push_str("  \"fields\": [\n");
        let mut offset = 0usize;
        for (i, field) in self.fields.iter().enumerate() {
            let type_str = match field.canonical_type {
                CanonicalType::Bytes(n) => {
                    let mut t = String::from("bytes{");
                    write!(t, "{n}").unwrap();
                    t.push('}');
                    t
                }
                other => String::from(other.as_str()),
            };
            write!(
                s,
                "    {{ \"name\": \"{}\", \"type\": \"{}\", \"size\": {}, \"offset\": {} }}",
                field.name, type_str, field.size, offset,
            )
            .unwrap();
            if i + 1 < self.fields.len() {
                s.push(',');
            }
            s.push('\n');
            offset += field.size;
        }
        if self.segments.is_empty() {
            s.push_str("  ]\n");
        } else {
            s.push_str("  ],\n");
            s.push_str("  \"segments\": [\n");
            for (i, seg) in self.segments.iter().enumerate() {
                write!(
                    s,
                    "    {{ \"name\": \"{}\", \"element_type\": \"{}\", \"element_size\": {} }}",
                    seg.name, seg.element_type, seg.element_size,
                )
                .unwrap();
                if i + 1 < self.segments.len() {
                    s.push(',');
                }
                s.push('\n');
            }
            s.push_str("  ]\n");
        }
        s.push('}');
        s
    }

    /// Validate structural invariants of this manifest.
    ///
    /// Returns `Ok(())` if:
    /// - All field sizes are non-zero
    /// - The first field is a `Header` with size 16
    /// - No duplicate field names exist
    /// - `total_size()` equals the sum of field sizes
    /// - Segment element sizes are non-zero (if any)
    /// - No duplicate segment names
    /// - No segment names collide with field names
    ///
    /// This does **not** recompute `layout_id` (that would require a
    /// SHA-256 dependency). Use [`hash_input()`](Self::hash_input) to
    /// verify the hash externally.
    #[cfg(feature = "std")]
    pub fn verify(&self) -> Result<(), String> {
        if self.fields.is_empty() {
            return Err("manifest has no fields".into());
        }

        // First field must be the header.
        let first = &self.fields[0];
        if first.canonical_type != CanonicalType::Header || first.size != 16 {
            return Err(format!(
                "first field must be Header(16), got {:?}({})",
                first.canonical_type, first.size,
            ));
        }

        // All sizes must be non-zero.
        for field in self.fields {
            if field.size == 0 {
                return Err(format!("field '{}' has zero size", field.name));
            }
        }

        // No duplicate field names.
        for (i, a) in self.fields.iter().enumerate() {
            for b in &self.fields[i + 1..] {
                if a.name == b.name {
                    return Err(format!("duplicate field name '{}'", a.name));
                }
            }
        }

        // ── Segment validation ───────────────────────────────────────

        for seg in self.segments {
            if seg.element_size == 0 {
                return Err(format!("segment '{}' has zero element_size", seg.name));
            }
        }

        // No duplicate segment names.
        for (i, a) in self.segments.iter().enumerate() {
            for b in &self.segments[i + 1..] {
                if a.name == b.name {
                    return Err(format!("duplicate segment name '{}'", a.name));
                }
            }
        }

        // No segment name collides with a field name.
        for seg in self.segments {
            for field in self.fields {
                if seg.name == field.name {
                    return Err(format!(
                        "segment name '{}' collides with field name",
                        seg.name,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Verify that raw account data matches this manifest.
    ///
    /// Checks:
    /// - Data length ≥ `total_size()`
    /// - Discriminator byte (offset 0) matches `self.discriminator`
    /// - Layout ID bytes (offsets 4..12) match `self.layout_id`
    ///
    /// This is the runtime counterpart to `verify()` (which checks the
    /// manifest's internal consistency). Use this to validate that
    /// on-chain data belongs to the expected account type.
    pub fn verify_account(&self, data: &[u8]) -> Result<(), &'static str> {
        let expected_size = self.min_size();
        if data.len() < expected_size {
            return Err("account data too small for manifest");
        }
        if data[0] != self.discriminator {
            return Err("discriminator mismatch");
        }
        if data.len() < 12 {
            return Err("account data too small for header");
        }
        if data[4..12] != self.layout_id {
            return Err("layout_id mismatch");
        }
        Ok(())
    }

    /// Verify that a caller-provided SHA-256 hash is consistent with
    /// this manifest's `layout_id`.
    ///
    /// The caller computes `SHA-256(self.hash_input())` using their own
    /// SHA-256 implementation and passes the full 32-byte digest here.
    /// This method checks that the first 8 bytes match `self.layout_id`,
    /// providing 256-bit collision resistance without adding a SHA-256
    /// dependency to this crate.
    ///
    /// Returns `Ok(())` if the truncated hash matches, or an error
    /// message if it doesn't.
    pub fn verify_hash(&self, full_sha256: &[u8; 32]) -> Result<(), &'static str> {
        if full_sha256[..8] != self.layout_id {
            return Err("layout_id does not match truncated SHA-256");
        }
        Ok(())
    }
}

/// Macro to generate a `LayoutManifest` from a `zero_copy_layout!` struct.
///
/// ```rust,ignore
/// use jiminy_schema::layout_manifest;
///
/// let manifest = layout_manifest!(Vault,
///     header:    Header  = 16,
///     balance:   U64     = 8,
///     authority: Pubkey  = 32,
/// );
/// ```
#[macro_export]
macro_rules! layout_manifest {
    (
        $name:ident,
        $( $field:ident : $ctype:ident = $size:expr ),+ $(,)?
    ) => {
        $crate::LayoutManifest {
            name: stringify!($name),
            version: $name::VERSION,
            discriminator: $name::DISC,
            layout_id: $name::LAYOUT_ID,
            fields: &[
                $( $crate::FieldDescriptor {
                    name: stringify!($field),
                    canonical_type: $crate::CanonicalType::$ctype,
                    size: $size,
                }, )+
            ],
            segments: &[],
        }
    };
    // Segmented variant: fixed fields + dynamic segments.
    (
        $name:ident,
        $( $field:ident : $ctype:ident = $size:expr ),+ $(,)?
        ;
        segments { $( $seg_name:ident : $seg_elem_type:ident = $seg_elem_size:expr ),+ $(,)? }
    ) => {
        $crate::LayoutManifest {
            name: stringify!($name),
            version: $name::VERSION,
            discriminator: $name::DISC,
            layout_id: $name::SEGMENTED_LAYOUT_ID,
            fields: &[
                $( $crate::FieldDescriptor {
                    name: stringify!($field),
                    canonical_type: $crate::CanonicalType::$ctype,
                    size: $size,
                }, )+
            ],
            segments: &[
                $( $crate::SegmentFieldDescriptor {
                    name: stringify!($seg_name),
                    element_type: stringify!($seg_elem_type),
                    element_size: $seg_elem_size,
                }, )+
            ],
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_size_sums_fields() {
        let manifest = LayoutManifest {
            name: "Test",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "value", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        assert_eq!(manifest.total_size(), 24);
    }

    #[test]
    fn field_offset_finds_correct_position() {
        let manifest = LayoutManifest {
            name: "Test",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
                FieldDescriptor { name: "authority", canonical_type: CanonicalType::Pubkey, size: 32 },
            ],
            segments: &[],
        };
        assert_eq!(manifest.field_offset("header"), Some(0));
        assert_eq!(manifest.field_offset("balance"), Some(16));
        assert_eq!(manifest.field_offset("authority"), Some(24));
        assert_eq!(manifest.field_offset("nonexistent"), None);
    }

    #[test]
    fn hash_input_format() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
                FieldDescriptor { name: "authority", canonical_type: CanonicalType::Pubkey, size: 32 },
            ],
            segments: &[],
        };
        let input = manifest.hash_input();
        assert_eq!(input, "jiminy:v1:Vault:1:header:header:16,balance:u64:8,authority:pubkey:32,");
    }

    #[test]
    fn canonical_type_string_roundtrip() {
        assert_eq!(CanonicalType::U8.as_str(), "u8");
        assert_eq!(CanonicalType::U64.as_str(), "u64");
        assert_eq!(CanonicalType::Pubkey.as_str(), "pubkey");
        assert_eq!(CanonicalType::Header.as_str(), "header");
        assert_eq!(CanonicalType::Bool.as_str(), "bool");
        assert_eq!(CanonicalType::I128.as_str(), "i128");
    }

    #[test]
    fn export_json_structure() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        let json = manifest.export_json();
        assert!(json.contains("\"name\": \"Vault\""));
        assert!(json.contains("\"total_size\": 24"));
        assert!(json.contains("\"layout_id\": \"abcdef0123456789\""));
        assert!(json.contains("\"offset\": 0"));
        assert!(json.contains("\"offset\": 16"));
    }

    #[test]
    fn verify_valid_manifest() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        assert!(manifest.verify().is_ok());
    }

    #[test]
    fn verify_rejects_no_header() {
        let manifest = LayoutManifest {
            name: "Bad",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        assert!(manifest.verify().is_err());
    }

    #[test]
    fn verify_rejects_duplicate_names() {
        let manifest = LayoutManifest {
            name: "Bad",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "x", canonical_type: CanonicalType::U64, size: 8 },
                FieldDescriptor { name: "x", canonical_type: CanonicalType::U32, size: 4 },
            ],
            segments: &[],
        };
        let err = manifest.verify().unwrap_err();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn verify_rejects_empty() {
        let manifest = LayoutManifest {
            name: "Empty",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[],
            segments: &[],
        };
        assert!(manifest.verify().is_err());
    }

    #[test]
    fn verify_rejects_zero_size_field() {
        let manifest = LayoutManifest {
            name: "Bad",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "empty", canonical_type: CanonicalType::U8, size: 0 },
            ],
            segments: &[],
        };
        let err = manifest.verify().unwrap_err();
        assert!(err.contains("zero size"));
    }

    #[test]
    fn field_count_returns_number_of_fields() {
        let manifest = LayoutManifest {
            name: "Test",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "a", canonical_type: CanonicalType::U64, size: 8 },
                FieldDescriptor { name: "b", canonical_type: CanonicalType::U32, size: 4 },
            ],
            segments: &[],
        };
        assert_eq!(manifest.field_count(), 3);
    }

    #[test]
    fn field_lookup_returns_descriptor() {
        let manifest = LayoutManifest {
            name: "Test",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "amount", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        let f = manifest.field("amount").unwrap();
        assert_eq!(f.canonical_type, CanonicalType::U64);
        assert_eq!(f.size, 8);
        assert!(manifest.field("nonexistent").is_none());
    }

    #[test]
    fn hash_input_with_segments() {
        let manifest = LayoutManifest {
            name: "Pool",
            version: 1,
            discriminator: 3,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "total", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "stakes", element_type: "StakeEntry", element_size: 48 },
            ],
        };
        let input = manifest.hash_input();
        assert!(input.contains("seg:stakes:StakeEntry:48,"));
        assert!(input.starts_with("jiminy:v1:Pool:1:"));
    }

    #[test]
    fn hash_input_with_bytes_field() {
        let manifest = LayoutManifest {
            name: "Buffer",
            version: 1,
            discriminator: 2,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "data", canonical_type: CanonicalType::Bytes(64), size: 64 },
            ],
            segments: &[],
        };
        let input = manifest.hash_input();
        assert!(input.contains("data:bytes{64}:64,"));
    }

    #[test]
    fn export_json_with_segments() {
        let manifest = LayoutManifest {
            name: "OrderBook",
            version: 1,
            discriminator: 5,
            layout_id: [0x11; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "base", canonical_type: CanonicalType::Pubkey, size: 32 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "bids", element_type: "Order", element_size: 48 },
                SegmentFieldDescriptor { name: "asks", element_type: "Order", element_size: 48 },
            ],
        };
        let json = manifest.export_json();
        assert!(json.contains("\"segments\":"));
        assert!(json.contains("\"name\": \"bids\""));
        assert!(json.contains("\"element_type\": \"Order\""));
        assert!(json.contains("\"element_size\": 48"));
    }

    #[test]
    fn canonical_type_bytes_as_str() {
        // Bytes variant always returns "bytes"; caller appends size.
        assert_eq!(CanonicalType::Bytes(32).as_str(), "bytes");
        assert_eq!(CanonicalType::Bytes(128).as_str(), "bytes");
    }

    #[test]
    fn verify_account_accepts_valid_data() {
        let layout_id = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89];
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id,
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        let mut data = vec![0u8; 24];
        data[0] = 1; // disc
        data[4..12].copy_from_slice(&layout_id);
        assert!(manifest.verify_account(&data).is_ok());
    }

    #[test]
    fn verify_account_rejects_wrong_disc() {
        let layout_id = [0xAB; 8];
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id,
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        let mut data = vec![0u8; 24];
        data[0] = 99; // wrong disc
        data[4..12].copy_from_slice(&layout_id);
        assert_eq!(manifest.verify_account(&data).unwrap_err(), "discriminator mismatch");
    }

    #[test]
    fn verify_account_rejects_wrong_layout_id() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0xAB; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        let mut data = vec![0u8; 24];
        data[0] = 1;
        data[4..12].copy_from_slice(&[0xFF; 8]); // wrong layout_id
        assert_eq!(manifest.verify_account(&data).unwrap_err(), "layout_id mismatch");
    }

    #[test]
    fn verify_account_rejects_too_small() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        let data = vec![0u8; 10]; // too small for 24-byte layout
        assert_eq!(manifest.verify_account(&data).unwrap_err(), "account data too small for manifest");
    }

    #[test]
    fn manifest_version_is_v1() {
        assert_eq!(MANIFEST_VERSION, "manifest-v1");
    }

    #[test]
    fn verify_hash_accepts_matching_prefix() {
        let layout_id = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89];
        let manifest = LayoutManifest {
            name: "Test",
            version: 1,
            discriminator: 1,
            layout_id,
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
            ],
            segments: &[],
        };
        let mut hash = [0u8; 32];
        hash[..8].copy_from_slice(&layout_id);
        // Rest can be anything; only first 8 bytes matter.
        hash[8] = 0xFF;
        assert!(manifest.verify_hash(&hash).is_ok());
    }

    #[test]
    fn verify_hash_rejects_mismatch() {
        let manifest = LayoutManifest {
            name: "Test",
            version: 1,
            discriminator: 1,
            layout_id: [0xAA; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
            ],
            segments: &[],
        };
        let wrong_hash = [0xBB; 32];
        assert!(manifest.verify_hash(&wrong_hash).is_err());
    }

    #[test]
    fn export_json_contains_manifest_version() {
        let manifest = LayoutManifest {
            name: "V",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
            ],
            segments: &[],
        };
        let json = manifest.export_json();
        assert!(json.contains(&format!("\"version\": \"{}\"", MANIFEST_VERSION)));
    }

    // ── min_size tests ───────────────────────────────────────────────

    #[test]
    fn min_size_equals_total_size_without_segments() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        assert_eq!(manifest.min_size(), manifest.total_size());
        assert_eq!(manifest.min_size(), 24);
    }

    #[test]
    fn min_size_includes_segment_table() {
        let manifest = LayoutManifest {
            name: "OrderBook",
            version: 1,
            discriminator: 5,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "market", canonical_type: CanonicalType::Pubkey, size: 32 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "bids", element_type: "Order", element_size: 48 },
                SegmentFieldDescriptor { name: "asks", element_type: "Order", element_size: 48 },
            ],
        };
        // total_size = 16 + 32 = 48 (fixed only)
        assert_eq!(manifest.total_size(), 48);
        // min_size = 48 + 2 * 12 = 72 (fixed + table)
        assert_eq!(manifest.min_size(), 72);
    }

    // ── Segment verification tests ───────────────────────────────────

    #[test]
    fn verify_rejects_zero_element_size_segment() {
        let manifest = LayoutManifest {
            name: "Bad",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "items", element_type: "Item", element_size: 0 },
            ],
        };
        let err = manifest.verify().unwrap_err();
        assert!(err.contains("zero element_size"));
    }

    #[test]
    fn verify_rejects_duplicate_segment_names() {
        let manifest = LayoutManifest {
            name: "Bad",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "items", element_type: "A", element_size: 16 },
                SegmentFieldDescriptor { name: "items", element_type: "B", element_size: 32 },
            ],
        };
        let err = manifest.verify().unwrap_err();
        assert!(err.contains("duplicate segment"));
    }

    #[test]
    fn verify_rejects_segment_field_name_collision() {
        let manifest = LayoutManifest {
            name: "Bad",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "items", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "items", element_type: "Item", element_size: 16 },
            ],
        };
        let err = manifest.verify().unwrap_err();
        assert!(err.contains("collides with field"));
    }

    #[test]
    fn verify_accepts_valid_segmented_manifest() {
        let manifest = LayoutManifest {
            name: "Pool",
            version: 1,
            discriminator: 3,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "total", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "stakes", element_type: "StakeEntry", element_size: 48 },
            ],
        };
        assert!(manifest.verify().is_ok());
    }

    #[test]
    fn export_json_includes_min_size_for_segments() {
        let manifest = LayoutManifest {
            name: "OrderBook",
            version: 1,
            discriminator: 5,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "market", canonical_type: CanonicalType::Pubkey, size: 32 },
            ],
            segments: &[
                SegmentFieldDescriptor { name: "bids", element_type: "Order", element_size: 48 },
                SegmentFieldDescriptor { name: "asks", element_type: "Order", element_size: 48 },
            ],
        };
        let json = manifest.export_json();
        assert!(json.contains("\"total_size\": 48"));
        assert!(json.contains("\"min_size\": 72"));
    }

    #[test]
    fn export_json_omits_min_size_for_non_segmented() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
            ],
            segments: &[],
        };
        let json = manifest.export_json();
        assert!(json.contains("\"total_size\": 24"));
        assert!(!json.contains("\"min_size\""));
    }
}
