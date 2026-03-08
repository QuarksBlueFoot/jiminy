//! Convenience re-exports for the common jiminy-solana usage pattern.
//!
//! ```rust,ignore
//! use jiminy_solana::prelude::*;
//! ```

// ── Token account readers & assertions ───────────────────────────────────────
pub use crate::token::{
    check_no_close_authority, check_no_delegate, check_not_frozen,
    check_token_account_frozen, check_token_account_initialized, check_token_account_mint,
    check_token_account_owner, check_token_balance_gte, check_token_program_match,
    token_account_amount, token_account_close_authority, token_account_delegate,
    token_account_delegated_amount, token_account_mint, token_account_owner,
    token_account_state, TOKEN_ACCOUNT_LEN,
};

// ── Mint account readers & checks ────────────────────────────────────────────
pub use crate::token::{
    check_mint_authority, check_mint_owner, mint_authority, mint_decimals,
    mint_freeze_authority, mint_is_initialized, mint_supply, MINT_LEN,
};

// ── Token-2022 extension reader ──────────────────────────────────────────────
pub use crate::token::{
    calculate_transfer_fee, check_no_cpi_guard as check_no_token_cpi_guard,
    check_no_default_account_state, check_no_permanent_delegate, check_no_transfer_fee,
    check_no_transfer_hook, check_not_non_transferable, check_safe_token_2022_mint,
    check_token_program_for_mint, find_extension, has_extension, check_no_extension,
    read_transfer_fee_config, ExtensionType, TransferFeeConfig, TransferFeeEpochConfig,
};

// ── CPI guard (reentrancy protection) ────────────────────────────────────────
#[cfg(feature = "programs")]
pub use crate::cpi::{
    check_no_cpi_caller, check_cpi_caller, check_sysvar_instructions,
};
pub use crate::cpi::{get_instruction_index, get_num_instructions};

// ── Transaction introspection ────────────────────────────────────────────────
pub use crate::introspect::{
    read_program_id_at, read_instruction_data_range, read_instruction_account_key,
};
#[cfg(feature = "programs")]
pub use crate::introspect::check_has_compute_budget;

// ── Compute budget guards ────────────────────────────────────────────────────
pub use crate::compute::{
    remaining_compute_units, check_compute_remaining, require_compute_remaining,
};

// ── Transaction composition guards ───────────────────────────────────────────
pub use crate::compose::{
    check_no_other_invocation, check_no_subsequent_invocation,
    detect_flash_loan_bracket, count_program_invocations,
};

// ── Cryptographic verification ───────────────────────────────────────────────
pub use crate::crypto::{check_ed25519_signature, check_ed25519_signer, ED25519_PROGRAM};
pub use crate::crypto::{sha256_leaf, verify_merkle_proof};

// ── Authority handoff (two-step rotation) ────────────────────────────────────
pub use crate::authority::{accept_authority, check_pending_authority, write_pending_authority};

// ── Pyth oracle readers ─────────────────────────────────────────────────────
pub use crate::oracle::{
    read_pyth_price, read_pyth_ema, pyth_agg_pub_slot,
    check_pyth_price_fresh, check_pyth_confidence,
    PythPrice, PythEma,
    PYTH_MAGIC, PYTH_VERSION, PYTH_PRICE_TYPE, STATUS_TRADING, PYTH_HEADER_LEN,
};

// ── Balance delta (safe swap guard) ──────────────────────────────────────────
pub use crate::balance::{
    snapshot_token_balance, snapshot_lamport_balance,
    check_balance_increased, check_balance_decreased, check_balance_delta,
    check_lamport_balance_increased,
};

// ── Safe CPI wrappers ───────────────────────────────────────────────────────
pub use crate::cpi::{
    safe_burn, safe_checked_transfer, safe_close_token_account, safe_create_account,
    safe_create_account_signed, safe_mint_to, safe_mint_to_signed, safe_transfer_sol,
    safe_transfer_tokens, safe_transfer_tokens_signed, transfer_lamports,
};

// ── CPI return data ─────────────────────────────────────────────────────────
pub use crate::cpi::{
    read_return_data, read_return_data_from, read_return_u64, MAX_RETURN_DATA,
};

// ── Program upgrade verification ─────────────────────────────────────────────
#[cfg(feature = "programs")]
pub use crate::upgrade::{
    read_upgrade_authority, check_program_immutable, check_upgrade_authority,
};

// ── TWAP accumulators ────────────────────────────────────────────────────────
pub use crate::twap::{update_twap_cumulative, compute_twap, check_twap_deviation};

// ── pinocchio CPI helpers ────────────────────────────────────────────────────
pub use pinocchio::cpi;
pub use pinocchio::instruction::{InstructionAccount, InstructionView};

// ── System program CPI ──────────────────────────────────────────────────────
pub use pinocchio_system::instructions::{
    CreateAccount, Transfer as SystemTransfer,
};

// ── Token program CPI ───────────────────────────────────────────────────────
pub use pinocchio_token::instructions::{
    Burn, CloseAccount, InitializeAccount, MintTo, Transfer as TokenTransfer,
};
