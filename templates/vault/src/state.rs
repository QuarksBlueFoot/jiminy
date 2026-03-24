use jiminy::prelude::*;

// ── Account layout ───────────────────────────────────────────────────────────
//
// Define your account with `zero_copy_layout!`. This generates:
//   - `Vault::DISC`, `Vault::VERSION`, `Vault::LEN`, `Vault::LAYOUT_ID`
//   - `Vault::overlay()`, `overlay_mut()` for zero-copy reads/writes
//   - `Vault::load_checked()`, `load_checked_mut()` for validated access
//   - Compile-time `size_of` == `LEN` assertion

zero_copy_layout! {
    /// On-chain vault account.
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}

// Re-export constants for convenience.
pub const VAULT_DISC: u8 = Vault::DISC;
pub const VAULT_LEN: usize = Vault::LEN;
