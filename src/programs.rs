use pinocchio::address::address;
use pinocchio::Address;

/// The system program - where lamports come from and where rent goes.
pub const SYSTEM: Address = Address::new_from_array([0u8; 32]);

/// SPL Token (original) program.
///
/// Handles mint/burn/transfer for standard tokens. If you're not sure
/// which token program a given mint uses, check the mint account's owner.
pub const TOKEN: Address =
    address!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// SPL Token-2022 (Token Extensions) program.
///
/// The newer token program with optional extensions: transfer fees,
/// confidential transfers, metadata, interest-bearing, and more.
pub const TOKEN_2022: Address =
    address!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Associated Token Account (ATA) program.
///
/// Derives and creates the canonical token account for a wallet + mint pair.
pub const ASSOCIATED_TOKEN: Address =
    address!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bTu");

/// Metaplex Token Metadata program.
///
/// Manages on-chain NFT and fungible token metadata (name, symbol, URI,
/// creators, royalties). Owner of the metadata account PDA.
pub const METADATA: Address =
    address!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

/// BPF Loader Upgradeable.
///
/// All deployed programs are owned by this. Useful for verifying that
/// an account passed as a `program` parameter actually is a program.
pub const BPF_LOADER: Address =
    address!("BPFLoaderUpgradeab1e11111111111111111111111");

/// Compute Budget program.
///
/// Used to set `ComputeUnitLimit` and `ComputeUnitPrice` via instructions
/// prepended to a transaction. Not something you typically CPI into from
/// an on-chain program, but useful for address checks.
pub const COMPUTE_BUDGET: Address =
    address!("ComputeBudget111111111111111111111111111111");

/// Sysvar: Clock (slot, epoch, unix_timestamp, leader_schedule_epoch).
pub const SYSVAR_CLOCK: Address =
    address!("SysvarC1ock11111111111111111111111111111111");

/// Sysvar: Rent (lamports_per_byte_year, exemption_threshold, burn_percent).
pub const SYSVAR_RENT: Address =
    address!("SysvarRent111111111111111111111111111111111");

/// Sysvar: Instructions (introspect other instructions in the same tx).
pub const SYSVAR_INSTRUCTIONS: Address =
    address!("Sysvar1nstructions1111111111111111111111111");
