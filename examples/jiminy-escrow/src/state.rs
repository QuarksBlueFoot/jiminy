use jiminy::prelude::*;

zero_copy_layout! {
    /// On-chain escrow account.
    pub struct Escrow, discriminator = 2, version = 1 {
        header:    AccountHeader = 16,
        amount:    u64           = 8,
        creator:   Address       = 32,
        recipient: Address       = 32,
        timeout:   i64           = 8,
    }
}

/// Escrow account discriminator.
pub const ESCROW_DISC: u8 = Escrow::DISC;

/// Escrow account version.
pub const ESCROW_VERSION: u8 = Escrow::VERSION;

/// Total size of an escrow account (16 header + 8 + 32 + 32 + 8 = 96).
pub const ESCROW_LEN: usize = Escrow::LEN;

/// Escrow layout ID.
pub const ESCROW_LAYOUT_ID: [u8; 8] = Escrow::LAYOUT_ID;

// Payload offsets (after HEADER_LEN = 16).
pub const AMOUNT_OFFSET: usize = 0;
pub const CREATOR_OFFSET: usize = 8;
pub const RECIPIENT_OFFSET: usize = 40;
pub const TIMEOUT_OFFSET: usize = 72;

// Flag bits (byte 2 of header).
/// Set when the escrow has been accepted by the recipient.
pub const FLAG_ACCEPTED: u8 = 0;
