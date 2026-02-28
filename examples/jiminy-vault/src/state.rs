/// Vault account discriminator.
pub const VAULT_DISC: u8 = 1;

/// Vault account version (Jiminy Layout v1).
pub const VAULT_VERSION: u8 = 1;

/// Total size of a vault account.
///
/// Layout (Jiminy Header v1):
///   [0]     u8   discriminator  (= 1)
///   [1]     u8   version        (= 1)
///   [2]     u8   flags          (reserved, 0)
///   [3]     u8   reserved       (0)
///   [4..8]  u32  data_len       (0 for fixed-size)
///   --- payload ---
///   [8..16] u64  balance
///   [16..48] [u8; 32] authority
///
/// Total: 8 (header) + 8 (balance) + 32 (authority) = 48 bytes
pub const VAULT_LEN: usize = 48;

// Field offsets within the payload (after HEADER_LEN).
pub const BALANCE_OFFSET: usize = 0;
pub const AUTHORITY_OFFSET: usize = 8;
