//! Compatibility modules for optional external crate integrations.
//!
//! Each sub-module is feature-gated and only compiled when the
//! corresponding feature is enabled.

/// Integration with [`solana-zero-copy`] unaligned primitive types.
///
/// Enabled via the `solana-zero-copy` feature flag.
#[cfg(feature = "solana-zero-copy")]
pub mod szc;
