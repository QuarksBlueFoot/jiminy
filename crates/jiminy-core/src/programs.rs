use hopper_runtime::Address;

/// The system program — where lamports come from and where rent goes.
///
/// `11111111111111111111111111111111`
pub const SYSTEM: Address = Address::new_from_array([0u8; 32]);

/// SPL Token (original) program.
///
/// Handles mint/burn/transfer for standard tokens. If you're not sure
/// which token program a given mint uses, check the mint account's owner.
///
/// `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
pub const TOKEN: Address = Address::new_from_array([
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172,
    28, 180, 133, 237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
]);

/// SPL Token-2022 (Token Extensions) program.
///
/// The newer token program with optional extensions: transfer fees,
/// confidential transfers, metadata, interest-bearing, and more.
///
/// `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`
pub const TOKEN_2022: Address = Address::new_from_array([
    6, 221, 246, 225, 238, 117, 143, 222, 24, 66, 93, 188, 228, 108, 205, 218,
    182, 26, 252, 77, 131, 185, 13, 39, 254, 189, 249, 40, 216, 161, 139, 252,
]);

/// Associated Token Account (ATA) program.
///
/// Derives and creates the canonical token account for a wallet + mint pair.
///
/// `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bTu`
pub const ASSOCIATED_TOKEN: Address = Address::new_from_array([
    140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131,
    11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216, 238, 183, 148, 144,
]);

/// Metaplex Token Metadata program.
///
/// Manages on-chain NFT and fungible token metadata (name, symbol, URI,
/// creators, royalties). Owner of the metadata account PDA.
///
/// `metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s`
pub const METADATA: Address = Address::new_from_array([
    11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205,
    88, 184, 108, 115, 26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70,
]);

/// BPF Loader Upgradeable.
///
/// All deployed programs are owned by this. Useful for verifying that
/// an account passed as a `program` parameter actually is a program.
///
/// `BPFLoaderUpgradeab1e11111111111111111111111`
pub const BPF_LOADER: Address = Address::new_from_array([
    2, 168, 246, 145, 78, 136, 161, 176, 226, 16, 21, 62, 247, 99, 174, 43,
    0, 194, 185, 61, 22, 193, 36, 210, 192, 83, 122, 16, 4, 128, 0, 0,
]);

/// Compute Budget program.
///
/// Used to set `ComputeUnitLimit` and `ComputeUnitPrice` via instructions
/// prepended to a transaction. Not something you typically CPI into from
/// an on-chain program, but useful for address checks.
///
/// `ComputeBudget111111111111111111111111111111`
pub const COMPUTE_BUDGET: Address = Address::new_from_array([
    3, 6, 70, 111, 229, 33, 23, 50, 255, 236, 173, 186, 114, 195, 155, 231,
    188, 140, 229, 187, 197, 247, 18, 107, 44, 67, 155, 58, 64, 0, 0, 0,
]);

/// Sysvar: Clock (slot, epoch, unix_timestamp, leader_schedule_epoch).
///
/// `SysvarC1ock11111111111111111111111111111111`
pub const SYSVAR_CLOCK: Address = Address::new_from_array([
    6, 167, 213, 23, 24, 199, 116, 201, 40, 86, 99, 152, 105, 29, 94, 182,
    139, 94, 184, 163, 155, 75, 109, 92, 115, 85, 91, 33, 0, 0, 0, 0,
]);

/// Sysvar: Rent (lamports_per_byte_year, exemption_threshold, burn_percent).
///
/// `SysvarRent111111111111111111111111111111111`
pub const SYSVAR_RENT: Address = Address::new_from_array([
    6, 167, 213, 23, 25, 44, 92, 81, 33, 140, 201, 76, 61, 74, 241, 127,
    88, 218, 238, 8, 155, 161, 253, 68, 227, 219, 217, 138, 0, 0, 0, 0,
]);

/// Sysvar: Instructions (introspect other instructions in the same tx).
///
/// `Sysvar1nstructions1111111111111111111111111`
pub const SYSVAR_INSTRUCTIONS: Address = Address::new_from_array([
    6, 167, 213, 23, 24, 123, 209, 102, 53, 218, 212, 4, 85, 253, 194, 192,
    193, 36, 198, 143, 33, 86, 117, 165, 219, 186, 203, 95, 8, 0, 0, 0,
]);
