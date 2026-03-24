//! Token-2022 extension reader and safety checks.
//!
//! Token-2022 accounts use a TLV (Type-Length-Value) extension format
//! appended after the base account/mint data. Zero-copy accessors to
//! detect, read, and validate extensions without deserialization.
//!
//! ## Extension layout
//!
//! After the base mint (82 bytes) or token account (165 bytes), Token-2022
//! adds a 1-byte account type discriminator, followed by zero or more
//! TLV entries:
//!
//! ```text
//! [base data][padding to multisig len][account_type: u8][ext_type: u16][ext_len: u16][ext_data]...
//! ```
//!
//! The Multisig length is 355 bytes, so:
//! - For mints: extensions start at byte 166 (82 base + 83 padding + 1 account_type)
//! - For token accounts: extensions start at byte 166 (165 base + 0 padding + 1 account_type)
//!
//! ## Safety
//!
//! Programs accepting Token-2022 tokens MUST screen for dangerous extensions
//! that change transfer semantics (transfer fees, hooks, non-transferable,
//! permanent delegate). Ignoring these is a critical vulnerability.

use pinocchio::{error::ProgramError, AccountView, ProgramResult};

/// Base mint length before extensions.
pub const BASE_MINT_LEN: usize = 82;

/// Base token account length before extensions.
pub const BASE_ACCOUNT_LEN: usize = 165;

/// The MultisigAccount length - the padding boundary for TLV extensions.
/// Both mints and accounts are padded to this length before the account
/// type byte and TLV data begin.
const MULTISIG_LEN: usize = 355;

/// Offset where the account type byte lives (same for both mints and accounts
/// after padding).
const ACCOUNT_TYPE_OFFSET: usize = MULTISIG_LEN;

/// Offset where TLV extension data begins (after the account type byte).
const TLV_START: usize = ACCOUNT_TYPE_OFFSET + 1;

/// Account type discriminators used by Token-2022.
pub const ACCOUNT_TYPE_UNINITIALIZED: u8 = 0;
pub const ACCOUNT_TYPE_MINT: u8 = 1;
pub const ACCOUNT_TYPE_ACCOUNT: u8 = 2;

/// Declare an extension type enum with auto-generated `from_u16`.
macro_rules! extension_types {
    (
        $(#[$emeta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $disc:expr ),+ $(,)?
        }
    ) => {
        $(#[$emeta])*
        #[derive(Clone, Copy, PartialEq, Eq)]
        #[repr(u16)]
        $vis enum $name {
            $( $(#[$vmeta])* $variant = $disc ),+
        }

        impl $name {
            /// Convert a raw u16 to an ExtensionType, returning None for unknown types.
            #[inline(always)]
            pub fn from_u16(val: u16) -> Option<Self> {
                match val {
                    $( $disc => Some(Self::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

extension_types! {
    /// Known Token-2022 extension types.
    ///
    /// Each extension is identified by a `u16` type tag in the TLV header.
    /// This list covers the extensions most relevant to DeFi safety.
    pub enum ExtensionType {
        /// Uninitialized extension slot.
        Uninitialized = 0,
        /// Transfer fee configuration on the mint.
        TransferFeeConfig = 1,
        /// Transfer fee amount on the token account.
        TransferFeeAmount = 2,
        /// Mint close authority (allows closing the mint).
        MintCloseAuthority = 3,
        /// Confidential transfer mint configuration.
        ConfidentialTransferMint = 4,
        /// Confidential transfer account state.
        ConfidentialTransferAccount = 5,
        /// Default account state (e.g., Frozen by default).
        DefaultAccountState = 6,
        /// Immutable owner (token account owner cannot change).
        ImmutableOwner = 7,
        /// Memo required on incoming transfers.
        MemoTransfer = 8,
        /// Non-transferable (soulbound) token.
        NonTransferable = 9,
        /// Interest-bearing mint configuration.
        InterestBearingConfig = 10,
        /// CPI guard on the token account.
        CpiGuard = 11,
        /// Permanent delegate (can transfer/burn any time).
        PermanentDelegate = 12,
        /// Non-transferable account marker.
        NonTransferableAccount = 13,
        /// Transfer hook configuration on the mint.
        TransferHook = 14,
        /// Transfer hook account marker.
        TransferHookAccount = 15,
        /// Confidential transfer fee configuration.
        ConfidentialTransferFeeConfig = 16,
        /// Confidential transfer fee amount.
        ConfidentialTransferFeeAmount = 17,
        /// Metadata pointer (points to metadata stored elsewhere).
        MetadataPointer = 18,
        /// Inline token metadata stored on the mint.
        TokenMetadata = 19,
        /// Group pointer.
        GroupPointer = 20,
        /// Group member pointer.
        GroupMemberPointer = 21,
        /// Token group.
        TokenGroup = 22,
        /// Token group member.
        TokenGroupMember = 23,
    }
}

// ── TLV Walking ──────────────────────────────────────────────────────────────

/// Find a specific extension in TLV data and return its payload slice.
///
/// Walks the TLV entries after the base mint/account data. Returns `None`
/// if the extension is not present or if the data is too short for TLV.
///
/// `data` should be the full borrowed account data (`account.try_borrow()?`).
///
/// ```rust,ignore
/// let data = mint_account.try_borrow()?;
/// if let Some(fee_config) = find_extension(&data, ExtensionType::TransferFeeConfig) {
///     // fee_config is the raw extension payload bytes
/// }
/// ```
#[inline]
pub fn find_extension(data: &[u8], ext_type: ExtensionType) -> Option<&[u8]> {
    if data.len() <= TLV_START {
        return None;
    }
    let target = ext_type as u16;
    let mut offset = TLV_START;

    while offset + 4 <= data.len() {
        let ty = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;
        let payload_start = offset + 4;
        let payload_end = payload_start + len;

        if ty == 0 && len == 0 {
            // Uninitialized/padding entry - stop walking.
            break;
        }

        if ty == target {
            if payload_end > data.len() {
                return None; // Truncated extension data.
            }
            return Some(&data[payload_start..payload_end]);
        }

        if payload_end > data.len() {
            break; // Truncated - stop walking.
        }
        offset = payload_end;
    }

    None
}

/// Check if a specific extension exists in the account data.
///
/// Returns `true` if the extension type is found in the TLV entries.
///
/// ```rust,ignore
/// let data = mint_account.try_borrow()?;
/// if has_extension(&data, ExtensionType::TransferHook) {
///     // This mint has a transfer hook - extra accounts required.
/// }
/// ```
#[inline(always)]
pub fn has_extension(data: &[u8], ext_type: ExtensionType) -> bool {
    find_extension(data, ext_type).is_some()
}

/// Assert that a specific extension does NOT exist on the account.
///
/// Returns `InvalidAccountData` if the extension is found. Use this to
/// reject tokens with dangerous extensions your program doesn't support.
///
/// ```rust,ignore
/// let data = mint_account.try_borrow()?;
/// check_no_extension(&data, ExtensionType::TransferFeeConfig)?;
/// ```
#[inline(always)]
pub fn check_no_extension(data: &[u8], ext_type: ExtensionType) -> ProgramResult {
    if has_extension(data, ext_type) {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── Convenience Checks ───────────────────────────────────────────────────────
//
// One-line safety guards for the most commonly dangerous extensions.
// Generated by macro to avoid copy-paste for each extension type.

/// Generate a `check_no_$name` function that rejects a specific extension.
macro_rules! check_no_ext {
    ($(
        $(#[$meta:meta])*
        $fn_name:ident => $ext:ident;
    )*) => {
        $(
            $(#[$meta])*
            #[inline(always)]
            pub fn $fn_name(data: &[u8]) -> ProgramResult {
                check_no_extension(data, ExtensionType::$ext)
            }
        )*
    };
}

check_no_ext! {
    /// Reject mints with transfer fee extensions.
    ///
    /// If your AMM/vault doesn't account for transfer fees, accepting a
    /// fee-on-transfer mint means the pool receives fewer tokens than expected,
    /// draining LPs over time.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_no_transfer_fee(&data)?;
    /// ```
    check_no_transfer_fee => TransferFeeConfig;

    /// Reject mints with transfer hook extensions.
    ///
    /// Transfer hooks invoke an arbitrary program on every transfer. If your
    /// program doesn't pass the extra accounts the hook requires, transfers
    /// will fail. If you don't audit the hook program, it could be malicious.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_no_transfer_hook(&data)?;
    /// ```
    check_no_transfer_hook => TransferHook;

    /// Reject non-transferable (soulbound) tokens.
    ///
    /// Non-transferable tokens cannot be moved between accounts. Attempting
    /// to transfer them will fail inside the token program.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_not_non_transferable(&data)?;
    /// ```
    check_not_non_transferable => NonTransferable;

    /// Reject mints with a permanent delegate.
    ///
    /// A permanent delegate can transfer or burn tokens from ANY account for
    /// this mint at any time, without the owner's permission. This is
    /// extremely dangerous for DeFi pools and escrows.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_no_permanent_delegate(&data)?;
    /// ```
    check_no_permanent_delegate => PermanentDelegate;

    /// Reject token accounts with the CPI guard enabled.
    ///
    /// The CPI guard prevents transfers initiated via CPI. If your program
    /// needs to transfer tokens from this account via CPI, the transfer will
    /// fail silently.
    ///
    /// ```rust,ignore
    /// let data = token_account.try_borrow()?;
    /// check_no_cpi_guard(&data)?;
    /// ```
    check_no_cpi_guard => CpiGuard;

    /// Reject mints with a default frozen account state.
    ///
    /// Mints with `DefaultAccountState` set to `Frozen` will create all new
    /// token accounts in a frozen state, requiring explicit unfreezing.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_no_default_account_state(&data)?;
    /// ```
    check_no_default_account_state => DefaultAccountState;
}

// ── Transfer Fee Reader ──────────────────────────────────────────────────────

/// Transfer fee epoch configuration (one epoch's worth of fee settings).
#[derive(Clone, Copy)]
pub struct TransferFeeEpochConfig {
    /// Fee in basis points (100 = 1%).
    pub transfer_fee_basis_points: u16,
    /// Maximum fee in token amount - fees are capped at this value.
    pub maximum_fee: u64,
    /// The epoch at which this config activates.
    pub epoch: u64,
}

/// Transfer fee configuration containing both current and upcoming fee schedules.
#[derive(Clone, Copy)]
pub struct TransferFeeConfig {
    /// Authority that can modify transfer fees (32 bytes, may be zeroed).
    pub transfer_fee_config_authority: [u8; 32],
    /// Authority that can withdraw collected fees (32 bytes).
    pub withdraw_withheld_authority: [u8; 32],
    /// Fee withheld on the mint itself.
    pub withheld_amount: u64,
    /// Currently active fee configuration.
    pub older_transfer_fee: TransferFeeEpochConfig,
    /// Upcoming fee configuration.
    pub newer_transfer_fee: TransferFeeEpochConfig,
}

/// Read the TransferFeeConfig extension from mint account data.
///
/// Returns `None` if the extension is not present. Use this when your
/// program DOES want to handle transfer fees correctly instead of rejecting
/// them outright.
///
/// ```rust,ignore
/// let data = mint_account.try_borrow()?;
/// if let Some(config) = read_transfer_fee_config(&data)? {
///     let fee = calculate_transfer_fee(amount, config.older_transfer_fee);
/// }
/// ```
#[inline]
pub fn read_transfer_fee_config(data: &[u8]) -> Result<Option<TransferFeeConfig>, ProgramError> {
    let ext_data = match find_extension(data, ExtensionType::TransferFeeConfig) {
        Some(d) => d,
        None => return Ok(None),
    };

    // TransferFeeConfig is 108 bytes:
    // 32 (config authority) + 32 (withdraw authority) + 8 (withheld) +
    // 2+8+8 (older epoch config) + 2+8+8 (newer epoch config) = 108
    if ext_data.len() < 108 {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut auth = [0u8; 32];
    auth.copy_from_slice(&ext_data[0..32]);
    let mut withdraw_auth = [0u8; 32];
    withdraw_auth.copy_from_slice(&ext_data[32..64]);

    let withheld = u64::from_le_bytes(
        ext_data[64..72]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );

    let older = read_epoch_config(&ext_data[72..90])?;
    let newer = read_epoch_config(&ext_data[90..108])?;

    Ok(Some(TransferFeeConfig {
        transfer_fee_config_authority: auth,
        withdraw_withheld_authority: withdraw_auth,
        withheld_amount: withheld,
        older_transfer_fee: older,
        newer_transfer_fee: newer,
    }))
}

/// Parse a single epoch fee config from an 18-byte slice.
/// Layout: epoch (u64 LE) + maximum_fee (u64 LE) + basis_points (u16 LE)
#[inline(always)]
fn read_epoch_config(data: &[u8]) -> Result<TransferFeeEpochConfig, ProgramError> {
    let epoch = u64::from_le_bytes(
        data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    let maximum_fee = u64::from_le_bytes(
        data[8..16]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );
    let basis_points = u16::from_le_bytes(
        data[16..18]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    );

    Ok(TransferFeeEpochConfig {
        epoch,
        maximum_fee,
        transfer_fee_basis_points: basis_points,
    })
}

/// Calculate the transfer fee for a given amount.
///
/// Uses the same formula as the Token-2022 program:
/// `fee = min(amount * basis_points / 10_000, maximum_fee)`
///
/// Ceiling division ensures the protocol never rounds fees down to zero.
///
/// ```rust,ignore
/// let fee = calculate_transfer_fee(amount, &config.older_transfer_fee);
/// let net_amount = checked_sub(amount, fee)?;
/// ```
#[inline(always)]
pub fn calculate_transfer_fee(amount: u64, config: &TransferFeeEpochConfig) -> u64 {
    if config.transfer_fee_basis_points == 0 {
        return 0;
    }
    // Ceiling division, capped at max_fee
    let numerator = (amount as u128) * (config.transfer_fee_basis_points as u128);
    let fee = numerator.div_ceil(10_000) as u64;
    if fee > config.maximum_fee {
        config.maximum_fee
    } else {
        fee
    }
}

/// Verify that a mint's owning program matches the token program account.
///
/// This is the critical Token-2022 ↔ Token program mismatch check.
/// A Token-2022 mint must be used with the Token-2022 program, and a
/// classic SPL mint must be used with the classic Token program.
///
/// ```rust,ignore
/// check_token_program_for_mint(mint_account, token_program)?;
/// ```
#[inline(always)]
pub fn check_token_program_for_mint(
    mint_account: &AccountView,
    token_program: &AccountView,
) -> ProgramResult {
    if !mint_account.owned_by(token_program.address()) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Screen a Token-2022 mint for all commonly dangerous extensions.
///
/// Rejects mints that have ANY of: TransferFeeConfig, TransferHook,
/// NonTransferable, PermanentDelegate, DefaultAccountState.
///
/// Use this as a "safe default" for programs that only want to support
/// basic Token-2022 mints without extension-specific logic.
///
/// ```rust,ignore
/// let data = mint_account.try_borrow()?;
/// check_safe_token_2022_mint(&data)?;
/// ```
#[inline]
pub fn check_safe_token_2022_mint(data: &[u8]) -> ProgramResult {
    check_no_transfer_fee(data)?;
    check_no_transfer_hook(data)?;
    check_not_non_transferable(data)?;
    check_no_permanent_delegate(data)?;
    check_no_default_account_state(data)?;
    Ok(())
}
