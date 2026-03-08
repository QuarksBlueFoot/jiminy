//! Cryptographic verification: Ed25519 precompile and Merkle proofs.
//!
//! ```rust,ignore
//! use jiminy_solana::crypto::{check_ed25519_signature, verify_merkle_proof};
//! ```

pub mod ed25519;
pub mod merkle;

// ── Re-exports ───────────────────────────────────────────────────────────────
pub use ed25519::{check_ed25519_signature, check_ed25519_signer, ED25519_PROGRAM};
pub use merkle::{sha256_leaf, verify_merkle_proof};
