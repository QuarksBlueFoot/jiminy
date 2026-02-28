/// Escrow account discriminator.
pub const ESCROW_DISC: u8 = 2;

/// Escrow account version (Jiminy Layout v1).
pub const ESCROW_VERSION: u8 = 1;

/// Total size of an escrow account.
///
/// Layout (Jiminy Header v1):
///   [0]      u8      discriminator  (= 2)
///   [1]      u8      version        (= 1)
///   [2]      u8      flags          (bit 0 = accepted)
///   [3]      u8      reserved       (0)
///   [4..8]   u32     data_len       (0 for fixed-size)
///   --- payload ---
///   [8..16]  u64     amount         (lamports locked)
///   [16..48] Address creator        (who created the escrow)
///   [48..80] Address recipient      (who can accept)
///   [80..88] i64     timeout_ts     (unix timestamp; 0 = no timeout)
///
/// Total: 8 (header) + 8 + 32 + 32 + 8 = 88 bytes
pub const ESCROW_LEN: usize = 88;

// Payload offsets (after HEADER_LEN = 8).
pub const AMOUNT_OFFSET: usize = 0;
pub const CREATOR_OFFSET: usize = 8;
pub const RECIPIENT_OFFSET: usize = 40;
pub const TIMEOUT_OFFSET: usize = 72;

// Flag bits (byte 2 of header).
/// Set when the escrow has been accepted by the recipient.
pub const FLAG_ACCEPTED: u8 = 0;
