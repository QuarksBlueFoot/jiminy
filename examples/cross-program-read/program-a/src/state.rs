use jiminy::prelude::*;

zero_copy_layout! {
    /// On-chain vault account owned by Program A.
    pub struct Vault, discriminator = 1, version = 1 {
        header:    AccountHeader = 16,
        balance:   u64           = 8,
        authority: Address       = 32,
    }
}
