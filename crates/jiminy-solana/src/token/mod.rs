//! SPL Token readers, mint readers, and Token-2022 extension screening.
//!
//! ```rust,ignore
//! use jiminy_solana::token::{token_account_owner, mint_decimals, check_safe_token_2022_mint};
//! ```

pub mod account;
pub mod ext;
pub mod mint;

// ── Re-exports: token account ────────────────────────────────────────────────
pub use account::{
    check_no_close_authority, check_no_delegate, check_not_frozen,
    check_token_account_frozen, check_token_account_initialized, check_token_account_mint,
    check_token_account_owner, check_token_balance_gte, check_token_program_match,
    token_account_amount, token_account_close_authority, token_account_delegate,
    token_account_delegated_amount, token_account_mint, token_account_owner,
    token_account_state, TOKEN_ACCOUNT_LEN,
};

// ── Re-exports: mint ─────────────────────────────────────────────────────────
pub use mint::{
    check_mint_authority, check_mint_owner, mint_authority, mint_decimals,
    mint_freeze_authority, mint_is_initialized, mint_supply, MINT_LEN,
};

// ── Re-exports: Token-2022 extensions ────────────────────────────────────────
pub use ext::{
    calculate_transfer_fee, check_no_cpi_guard, check_no_default_account_state,
    check_no_extension, check_no_extension_account, check_no_extension_mint,
    check_no_permanent_delegate, check_no_transfer_fee, check_no_transfer_hook,
    check_not_non_transferable, check_safe_token_2022_mint, check_token_program_for_mint,
    find_extension, find_extension_account, find_extension_mint, find_extension_typed,
    has_extension, has_extension_account, has_extension_mint,
    read_transfer_fee_config, ExtensionType, TransferFeeConfig, TransferFeeEpochConfig,
    BASE_ACCOUNT_LEN, BASE_MINT_LEN,
    ACCOUNT_TYPE_UNINITIALIZED, ACCOUNT_TYPE_MINT, ACCOUNT_TYPE_ACCOUNT,
    ACCOUNT_TYPE_OFFSET, TLV_START,
};
