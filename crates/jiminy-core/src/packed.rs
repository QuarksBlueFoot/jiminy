//! Packed-layout helpers for forward-compatible state.

pub use crate::state::ExtensionRegion;

/// Alias for explicitly reserved bytes at the end of a layout.
pub type ReservedBytes<const N: usize> = ExtensionRegion<N>;
