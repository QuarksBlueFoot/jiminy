use jiminy::prelude::*;

zero_copy_layout! {
    /// On-chain vault account.
    ///
    /// Uses `LeU64` for the balance field - a little-endian wrapper that
    /// provides `PartialOrd`, `Ord`, `Display`, and `From`/`Into`
    /// conversions while keeping alignment at 1 (no padding).
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   LeU64         = 8,
        authority: Address       = 32,
    }
}

/// Vault account discriminator.
pub const VAULT_DISC: u8 = Vault::DISC;

/// Vault account version.
pub const VAULT_VERSION: u8 = Vault::VERSION;

/// Total size of a vault account (16 header + 8 balance + 32 authority = 56).
pub const VAULT_LEN: usize = Vault::LEN;

/// Vault layout ID (first 8 bytes of SHA-256 of canonical layout string).
pub const VAULT_LAYOUT_ID: [u8; 8] = Vault::LAYOUT_ID;

// Field offsets within the payload (after HEADER_LEN = 16).
pub const BALANCE_OFFSET: usize = 0;
pub const AUTHORITY_OFFSET: usize = 8;
