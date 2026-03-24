use jiminy::prelude::*;

// ── Stake entry element ──────────────────────────────────────────────────────
//
// Each stake entry is stored as an element in the `stakes` segment.

/// A single stake entry (48 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StakeEntry {
    /// Staker's public key.
    pub staker: Address,
    /// Amount staked (u64).
    pub amount: u64,
    /// Epoch when the stake was created (u64).
    pub start_epoch: u64,
}

// SAFETY: StakeEntry is #[repr(C)], Copy, all fields are plain data types.
unsafe impl Pod for StakeEntry {}
impl FixedLayout for StakeEntry {
    const SIZE: usize = 48;
}

// ── Pool account ─────────────────────────────────────────────────────────────
//
// The pool has a fixed prefix (header + authority + total_staked)
// followed by a dynamic segment of StakeEntry elements.

segmented_layout! {
    /// On-chain staking pool with variable-length stake list.
    pub struct StakePool, discriminator = 3, version = 1 {
        header:       AccountHeader = 16,
        authority:    Address       = 32,
        total_staked: u64           = 8,
    } segments {
        stakes: StakeEntry = 48,
    }
}

pub const POOL_DISC: u8 = StakePool::DISC;
