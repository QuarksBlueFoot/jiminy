//! Anchor-compatible IDL JSON generation.
//!
//! Generates an Anchor IDL v0.1.0 `accounts` fragment from a
//! [`LayoutManifest`]. This enables explorer
//! integration and wallet simulation for Jiminy programs without
//! depending on the Anchor framework.
//!
//! ```rust
//! use jiminy_schema::*;
//! use jiminy_schema::idl::anchor_idl_json;
//!
//! let manifest = LayoutManifest {
//!     name: "Vault",
//!     version: 1,
//!     discriminator: 1,
//!     layout_id: [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89],
//!     fields: &[
//!         FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
//!         FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
//!     ],
//!     segments: &[],
//! };
//!
//! let json = anchor_idl_json(&manifest);
//! assert!(json.contains("\"name\": \"Vault\""));
//! assert!(json.contains("\"type\": \"u64\""));
//! ```

use crate::{CanonicalType, LayoutManifest};
use std::fmt::Write;

/// Map a [`CanonicalType`] to the Anchor IDL type name.
fn anchor_type(ct: CanonicalType) -> &'static str {
    match ct {
        CanonicalType::U8 => "u8",
        CanonicalType::U16 => "u16",
        CanonicalType::U32 => "u32",
        CanonicalType::U64 => "u64",
        CanonicalType::U128 => "u128",
        CanonicalType::I8 => "i8",
        CanonicalType::I16 => "i16",
        CanonicalType::I32 => "i32",
        CanonicalType::I64 => "i64",
        CanonicalType::I128 => "i128",
        CanonicalType::Bool => "bool",
        CanonicalType::Pubkey => "publicKey",
        CanonicalType::Header => "bytes",
        CanonicalType::Bytes(_) => "bytes",
    }
}

/// Generate an Anchor IDL v0.1.0 `accounts` fragment for a manifest.
///
/// Returns a JSON string describing the account type in Anchor IDL
/// format. The `header` field is emitted as raw bytes since it has
/// no Anchor equivalent.
pub fn anchor_idl_json(manifest: &LayoutManifest) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    writeln!(s, "  \"name\": \"{}\",", manifest.name).unwrap();
    s.push_str("  \"type\": {\n");
    s.push_str("    \"kind\": \"struct\",\n");
    s.push_str("    \"fields\": [\n");

    for (i, field) in manifest.fields.iter().enumerate() {
        let type_str = anchor_type(field.canonical_type);

        s.push_str("      {\n");
        write!(s, "        \"name\": \"{}\"", field.name).unwrap();

        match field.canonical_type {
            CanonicalType::Bytes(n) => {
                s.push_str(",\n");
                s.push_str("        \"type\": {\n");
                s.push_str("          \"array\": [\n");
                s.push_str("            \"u8\",\n");
                writeln!(s, "            {n}").unwrap();
                s.push_str("          ]\n");
                s.push_str("        }\n");
            }
            CanonicalType::Header => {
                s.push_str(",\n");
                s.push_str("        \"type\": {\n");
                s.push_str("          \"array\": [\n");
                s.push_str("            \"u8\",\n");
                writeln!(s, "            {}", field.size).unwrap();
                s.push_str("          ]\n");
                s.push_str("        }\n");
            }
            _ => {
                s.push_str(",\n");
                writeln!(s, "        \"type\": \"{type_str}\"").unwrap();
            }
        }

        s.push_str("      }");
        if i + 1 < manifest.fields.len() {
            s.push(',');
        }
        s.push('\n');
    }

    s.push_str("    ]\n");
    s.push_str("  }\n");
    s.push('}');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FieldDescriptor;

    #[test]
    fn anchor_idl_basic() {
        let manifest = LayoutManifest {
            name: "Vault",
            version: 1,
            discriminator: 1,
            layout_id: [0; 8],
            fields: &[
                FieldDescriptor { name: "header", canonical_type: CanonicalType::Header, size: 16 },
                FieldDescriptor { name: "balance", canonical_type: CanonicalType::U64, size: 8 },
                FieldDescriptor { name: "owner", canonical_type: CanonicalType::Pubkey, size: 32 },
            ],
            segments: &[],
        };
        let json = anchor_idl_json(&manifest);
        assert!(json.contains("\"name\": \"Vault\""));
        assert!(json.contains("\"kind\": \"struct\""));
        assert!(json.contains("\"type\": \"u64\""));
        assert!(json.contains("\"type\": \"publicKey\""));
        // Header should be bytes array
        assert!(json.contains("\"array\""));
    }

    #[test]
    fn anchor_idl_bytes_field() {
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
        let json = anchor_idl_json(&manifest);
        assert!(json.contains("\"name\": \"data\""));
        assert!(json.contains("64"));
    }
}
