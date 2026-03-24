use jiminy::prelude::*;

// This struct is declared independently - no import from Program A.
// Because the field names, types, sizes, and version are identical,
// the computed LAYOUT_ID will match Program A's Vault.
zero_copy_layout! {
    /// Read-only view of Program A's Vault account.
    ///
    /// Same byte layout as Program A's `Vault`. The `LAYOUT_ID` fingerprint
    /// is deterministic: same fields → same hash → `load_foreign` succeeds.
    pub struct VaultView, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}
