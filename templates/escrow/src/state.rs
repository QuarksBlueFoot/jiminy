use jiminy::prelude::*;

// ── Account layout ───────────────────────────────────────────────────────────

zero_copy_layout! {
    /// On-chain escrow account.
    ///
    /// Flags (byte 2-3):
    ///   bit 0 — accepted (set when recipient claims)
    pub struct Escrow, discriminator = 2, version = 1 {
        header:    AccountHeader = 16,
        amount:    u64           = 8,
        creator:   Address       = 32,
        recipient: Address       = 32,
        deadline:  i64           = 8,
    }
}

pub const ESCROW_DISC: u8 = Escrow::DISC;
pub const ESCROW_LEN: usize = Escrow::LEN;

/// Flag bit: escrow has been accepted by the recipient.
pub const FLAG_ACCEPTED: u16 = 1 << 0;
