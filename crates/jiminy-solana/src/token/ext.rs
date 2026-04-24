//! Token-2022 extension reader and safety checks.
//!
//! Token-2022 accounts use a TLV (Type-Length-Value) extension format
//! appended after the base account/mint data. Zero-copy accessors to
//! detect, read, and validate extensions without deserialization.
//!
//! ## Extension layout (must match spl-token-2022 on mainnet)
//!
//! Token-2022 extended accounts always carry the account-type byte at
//! offset `BASE_ACCOUNT_LEN` (165). Mints are **padded** from their 82-byte
//! base up to `BASE_ACCOUNT_LEN` so that the type byte lives at the same
//! absolute position for both shapes. TLV entries begin immediately after
//! the type byte.
//!
//! ```text
//!   byte 0      ..82    : Mint body (or Account body 0..165)
//!   byte 82     ..165   : zero padding for extended mints (not present on Accounts)
//!   byte 165            : account-type discriminator (0x01=Mint, 0x02=Account)
//!   byte 166    ..      : TLV stream: [type:u16 LE][length:u16 LE][value: length bytes]...
//! ```
//!
//! A **non-extended** Token-2022 mint or account is indistinguishable from
//! its SPL Token counterpart: 82 bytes for a mint, 165 bytes for an account,
//! with no type byte and no TLV. `find_extension_*` returns `None` in that
//! case.
//!
//! ## Safety
//!
//! Programs accepting Token-2022 tokens MUST screen for dangerous extensions
//! that change transfer semantics (transfer fees, hooks, non-transferable,
//! permanent delegate). Ignoring these is a critical vulnerability.

use hopper_runtime::{ProgramError, AccountView, ProgramResult};

/// Base mint length before extensions.
pub const BASE_MINT_LEN: usize = 82;

/// Base token account length before extensions.
pub const BASE_ACCOUNT_LEN: usize = 165;

/// Offset of the account-type discriminator byte in an **extended**
/// Token-2022 account (mint padded, account natural).
///
/// Equal to `spl-token-2022`'s `BASE_ACCOUNT_LENGTH` constant.
pub const ACCOUNT_TYPE_OFFSET: usize = BASE_ACCOUNT_LEN;

/// Offset where TLV extension data begins (account-type byte + 1).
pub const TLV_START: usize = ACCOUNT_TYPE_OFFSET + 1;

/// Account-type byte value for an uninitialized extended account.
pub const ACCOUNT_TYPE_UNINITIALIZED: u8 = 0;

/// Account-type byte value for an extended Mint.
pub const ACCOUNT_TYPE_MINT: u8 = 1;

/// Account-type byte value for an extended Token Account.
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

/// Walk TLV entries without any account-type check.
///
/// **This is the raw primitive.** It will happily walk TLV on any buffer whose
/// length is `> TLV_START`, regardless of whether the account-type byte at
/// offset 165 matches the caller's expectation. Use this only when you have
/// already verified the account kind (e.g. via
/// [`find_extension_mint`]/[`find_extension_account`], or by screening data
/// length for a non-extended account).
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

/// Find an extension, requiring the buffer to match a specific account kind.
///
/// This is the **safe** walker. It rejects cross-kind confusion: if `data`
/// represents an extended Token-2022 account but the account-type byte at
/// offset 165 doesn't match `expected_kind` (`ACCOUNT_TYPE_MINT = 0x01` or
/// `ACCOUNT_TYPE_ACCOUNT = 0x02`), the function returns
/// `Err(InvalidAccountData)` rather than silently missing the extension.
///
/// Handling of the classic (non-extended) shape:
///
/// * A buffer of exactly `BASE_MINT_LEN` (82) bytes is a classic SPL Mint —
///   no TLV is possible, returns `Ok(None)` when `expected_kind` is Mint, and
///   `Err` otherwise.
/// * A buffer of exactly `BASE_ACCOUNT_LEN` (165) bytes is a classic SPL
///   Token Account — no TLV is possible, returns `Ok(None)` when
///   `expected_kind` is Account, and `Err` otherwise.
/// * Any buffer of length `>= TLV_START` (166) must carry the correct
///   account-type byte or is rejected.
/// * Anything else is malformed and rejected.
///
/// Pinocchio users can call this directly with their own `&[u8]` borrow; it
/// has no Hopper-native dependencies beyond the `ProgramError` enum.
#[inline]
pub fn find_extension_typed(
    data: &[u8],
    expected_kind: u8,
    ext_type: ExtensionType,
) -> Result<Option<&[u8]>, ProgramError> {
    // Classic (non-extended) shapes: no type byte, no TLV possible.
    // We still enforce that the classic *length* matches the kind the caller
    // asked about, so a 165-byte classic Account can't be screened as a Mint.
    if data.len() == BASE_MINT_LEN {
        return if expected_kind == ACCOUNT_TYPE_MINT {
            Ok(None)
        } else {
            Err(ProgramError::InvalidAccountData)
        };
    }
    if data.len() == BASE_ACCOUNT_LEN {
        return if expected_kind == ACCOUNT_TYPE_ACCOUNT {
            Ok(None)
        } else {
            Err(ProgramError::InvalidAccountData)
        };
    }

    // Extended Token-2022 shape: must contain the type byte and it must match.
    if data.len() < TLV_START {
        return Err(ProgramError::InvalidAccountData);
    }
    if data[ACCOUNT_TYPE_OFFSET] != expected_kind {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(find_extension(data, ext_type))
}

/// Find a mint-level extension on a Token-2022 buffer.
///
/// Same contract as [`find_extension_typed`] with `expected_kind =
/// ACCOUNT_TYPE_MINT`. Classic 82-byte SPL mints return `Ok(None)`.
#[inline(always)]
pub fn find_extension_mint(
    data: &[u8],
    ext_type: ExtensionType,
) -> Result<Option<&[u8]>, ProgramError> {
    find_extension_typed(data, ACCOUNT_TYPE_MINT, ext_type)
}

/// Find an account-level extension on a Token-2022 buffer.
///
/// Same contract as [`find_extension_typed`] with `expected_kind =
/// ACCOUNT_TYPE_ACCOUNT`. Classic 165-byte SPL accounts return `Ok(None)`.
#[inline(always)]
pub fn find_extension_account(
    data: &[u8],
    ext_type: ExtensionType,
) -> Result<Option<&[u8]>, ProgramError> {
    find_extension_typed(data, ACCOUNT_TYPE_ACCOUNT, ext_type)
}

/// Check if a specific extension exists in the account data.
///
/// Returns `true` if the extension type is found in the TLV entries.
/// **Does not validate the account-type byte** — prefer
/// [`has_extension_mint`]/[`has_extension_account`] when you know which kind
/// you are looking at.
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

/// Typed variant of [`has_extension`] for mint-level extensions.
///
/// Returns `Ok(false)` for classic 82-byte SPL mints. Returns
/// `Err(InvalidAccountData)` if the buffer does not look like a valid
/// mint (e.g. it is actually an account, or is truncated/malformed).
#[inline(always)]
pub fn has_extension_mint(
    data: &[u8],
    ext_type: ExtensionType,
) -> Result<bool, ProgramError> {
    Ok(find_extension_mint(data, ext_type)?.is_some())
}

/// Typed variant of [`has_extension`] for account-level extensions.
///
/// Returns `Ok(false)` for classic 165-byte SPL accounts. Returns
/// `Err(InvalidAccountData)` if the buffer does not look like a valid
/// account (e.g. it is actually a mint, or is truncated/malformed).
#[inline(always)]
pub fn has_extension_account(
    data: &[u8],
    ext_type: ExtensionType,
) -> Result<bool, ProgramError> {
    Ok(find_extension_account(data, ext_type)?.is_some())
}

/// Assert that a specific extension does NOT exist on the account.
///
/// Returns `InvalidAccountData` if the extension is found. This is the
/// **untyped** primitive; prefer the `check_no_*` helpers below (which are
/// routed through the typed walker) for safety-critical checks.
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

/// Assert a mint-level extension is absent, failing closed on wrong kind.
///
/// Unlike [`check_no_extension`], this returns `InvalidAccountData` both
/// when the extension is present **and** when the caller passed data that
/// does not look like a mint (e.g. a token account buffer). Use this for
/// any screen that is only meaningful on a mint.
#[inline(always)]
pub fn check_no_extension_mint(data: &[u8], ext_type: ExtensionType) -> ProgramResult {
    if find_extension_mint(data, ext_type)?.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Assert an account-level extension is absent, failing closed on wrong kind.
#[inline(always)]
pub fn check_no_extension_account(data: &[u8], ext_type: ExtensionType) -> ProgramResult {
    if find_extension_account(data, ext_type)?.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

// ── Convenience Checks ───────────────────────────────────────────────────────
//
// One-line safety guards for the most commonly dangerous extensions.
// Each helper routes through the *typed* walker for its kind, so passing the
// wrong kind of buffer (mint data where an account was expected, or vice
// versa) is rejected as `InvalidAccountData` rather than silently returning
// `Ok(())` — the exact failure mode that made the previous offset bug
// dangerous.

/// Generate `check_no_$name` helpers that reject a mint-level extension.
macro_rules! check_no_ext_mint {
    ($(
        $(#[$meta:meta])*
        $fn_name:ident => $ext:ident;
    )*) => {
        $(
            $(#[$meta])*
            #[inline(always)]
            pub fn $fn_name(data: &[u8]) -> ProgramResult {
                check_no_extension_mint(data, ExtensionType::$ext)
            }
        )*
    };
}

/// Generate `check_no_$name` helpers that reject an account-level extension.
macro_rules! check_no_ext_account {
    ($(
        $(#[$meta:meta])*
        $fn_name:ident => $ext:ident;
    )*) => {
        $(
            $(#[$meta])*
            #[inline(always)]
            pub fn $fn_name(data: &[u8]) -> ProgramResult {
                check_no_extension_account(data, ExtensionType::$ext)
            }
        )*
    };
}

check_no_ext_mint! {
    /// Reject mints with transfer fee extensions.
    ///
    /// If your AMM/vault doesn't account for transfer fees, accepting a
    /// fee-on-transfer mint means the pool receives fewer tokens than expected,
    /// draining LPs over time.
    ///
    /// Also returns `InvalidAccountData` if `data` is not a valid mint buffer
    /// (e.g. a token-account buffer was passed by mistake).
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
    /// Also returns `InvalidAccountData` on wrong-kind buffers.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_no_transfer_hook(&data)?;
    /// ```
    check_no_transfer_hook => TransferHook;

    /// Reject non-transferable (soulbound) mints.
    ///
    /// Non-transferable tokens cannot be moved between accounts. Attempting
    /// to transfer them will fail inside the token program.
    ///
    /// Also returns `InvalidAccountData` on wrong-kind buffers.
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
    /// Also returns `InvalidAccountData` on wrong-kind buffers.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_no_permanent_delegate(&data)?;
    /// ```
    check_no_permanent_delegate => PermanentDelegate;

    /// Reject mints with a default frozen account state.
    ///
    /// Mints with `DefaultAccountState` set to `Frozen` will create all new
    /// token accounts in a frozen state, requiring explicit unfreezing.
    ///
    /// Also returns `InvalidAccountData` on wrong-kind buffers.
    ///
    /// ```rust,ignore
    /// let data = mint_account.try_borrow()?;
    /// check_no_default_account_state(&data)?;
    /// ```
    check_no_default_account_state => DefaultAccountState;
}

check_no_ext_account! {
    /// Reject token accounts with the CPI guard enabled.
    ///
    /// The CPI guard prevents transfers initiated via CPI. If your program
    /// needs to transfer tokens from this account via CPI, the transfer will
    /// fail silently.
    ///
    /// Also returns `InvalidAccountData` if `data` is not a valid token
    /// account buffer (e.g. a mint buffer was passed by mistake).
    ///
    /// ```rust,ignore
    /// let data = token_account.try_borrow()?;
    /// check_no_cpi_guard(&data)?;
    /// ```
    check_no_cpi_guard => CpiGuard;
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
/// Returns `Ok(None)` if the extension is not present. Use this when your
/// program DOES want to handle transfer fees correctly instead of rejecting
/// them outright.
///
/// Fails with `InvalidAccountData` if the buffer is not a valid mint
/// (classic 82-byte mint or extended Token-2022 mint with the correct
/// account-type byte). Pass token-account data by mistake? This surfaces
/// the error instead of silently returning `None`.
///
/// ```rust,ignore
/// let data = mint_account.try_borrow()?;
/// if let Some(config) = read_transfer_fee_config(&data)? {
///     let fee = calculate_transfer_fee(amount, config.older_transfer_fee);
/// }
/// ```
#[inline]
pub fn read_transfer_fee_config(data: &[u8]) -> Result<Option<TransferFeeConfig>, ProgramError> {
    let ext_data = match find_extension_mint(data, ExtensionType::TransferFeeConfig)? {
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
