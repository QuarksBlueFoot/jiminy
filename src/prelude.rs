//! Convenience re-exports for the common Jiminy usage pattern.
//!
//! ```rust,ignore
//! use jiminy::prelude::*;
//! ```

// ── Check functions ──────────────────────────────────────────────────────────
pub use crate::checks::{
    check_account, check_accounts_unique_2, check_accounts_unique_3, check_closed,
    check_discriminator, check_executable, check_has_one, check_instruction_data_len,
    check_instruction_data_min, check_keys_eq, check_lamports_gte, check_owner, check_pda,
    check_rent_exempt, check_signer, check_size, check_system_program, check_uninitialized,
    check_version, check_writable, rent_exempt_min,
};

// ── Assert functions (PDA, address, program) ─────────────────────────────────
pub use crate::asserts::{
    assert_address, assert_not_initialized, assert_pda, assert_pda_external,
    assert_pda_with_bump, assert_program, assert_token_program,
};

// ── Token account readers & assertions ───────────────────────────────────────
pub use crate::token::{
    check_no_close_authority, check_no_delegate, check_token_account_frozen,
    check_token_account_initialized, check_token_account_mint, check_token_account_owner,
    check_token_balance_gte, check_token_program_match, token_account_amount,
    token_account_close_authority, token_account_delegate, token_account_delegated_amount,
    token_account_mint, token_account_owner, token_account_state, TOKEN_ACCOUNT_LEN,
};

// ── Mint account readers & checks ────────────────────────────────────────────
pub use crate::mint::{
    check_mint_authority, check_mint_owner, mint_authority, mint_decimals,
    mint_freeze_authority, mint_is_initialized, mint_supply, MINT_LEN,
};

// ── Token-2022 extension reader ──────────────────────────────────────────────
pub use crate::token_2022::{
    calculate_transfer_fee, check_no_cpi_guard as check_no_token_cpi_guard,
    check_no_default_account_state, check_no_permanent_delegate, check_no_transfer_fee,
    check_no_transfer_hook, check_not_non_transferable, check_safe_token_2022_mint,
    check_token_program_for_mint, find_extension, has_extension, check_no_extension,
    read_transfer_fee_config, ExtensionType, TransferFeeConfig, TransferFeeEpochConfig,
};

// ── Cursors ──────────────────────────────────────────────────────────────────
pub use crate::cursor::{write_discriminator, zero_init, DataWriter, SliceCursor};

// ── Account header (v1 convention) ───────────────────────────────────────────
pub use crate::header::{
    check_header, header_payload, header_payload_mut, read_data_len, read_header_flags,
    read_version, write_header, write_header_with_len, HEADER_LEN,
};

// ── Math ─────────────────────────────────────────────────────────────────────
pub use crate::math::{
    bps_of, bps_of_ceil, checked_add, checked_div, checked_div_ceil, checked_mul,
    checked_mul_div, checked_mul_div_ceil, checked_pow, checked_sub, to_u64,
};

// ── Bit helpers ──────────────────────────────────────────────────────────────
pub use crate::bits::{
    check_any_flag, check_flags, clear_bit, read_bit, read_flags_at, set_bit, toggle_bit,
    write_flags_at,
};

// ── Account lifecycle ────────────────────────────────────────────────────────
pub use crate::close::safe_close;

// ── PDA utilities ────────────────────────────────────────────────────────────
pub use crate::pda::{derive_ata, derive_ata_with_bump, derive_ata_with_program};
// Also: find_pda!, derive_pda!, derive_pda_const!, derive_ata_const! (macros, auto-exported)

// ── Account iteration ────────────────────────────────────────────────────────
pub use crate::accounts::AccountList;

// ── Sysvar readers ───────────────────────────────────────────────────────────
#[cfg(feature = "programs")]
pub use crate::sysvar::{
    check_clock_sysvar, check_rent_sysvar, read_clock, read_clock_epoch, read_clock_slot,
    read_clock_timestamp, read_rent_lamports_per_byte_year,
};

// ── CPI guard (reentrancy protection) ────────────────────────────────────────
#[cfg(feature = "programs")]
pub use crate::cpi_guard::{
    check_no_cpi_caller, check_cpi_caller, check_sysvar_instructions,
    get_instruction_index, get_num_instructions,
};

// ── Time / deadline checks ───────────────────────────────────────────────────
pub use crate::time::{
    check_cooldown, check_expired, check_not_expired, check_within_window,
};
#[cfg(feature = "programs")]
pub use crate::time::{check_after, check_deadline};

// ── State machine checks ─────────────────────────────────────────────────────
pub use crate::state::{
    check_state, check_state_in, check_state_not, check_state_transition, write_state,
};

// ── Slippage & economic bounds ───────────────────────────────────────────────
pub use crate::slippage::{
    check_max_amount, check_max_input, check_min_amount, check_nonzero,
    check_price_bounds, check_slippage, check_within_bps,
};

// ── Macros (re-exported from crate root via #[macro_export]) ─────────────────
pub use crate::{
    require, require_accounts_ne, require_eq, require_flag, require_gt, require_gte,
    require_keys_eq, require_keys_neq, require_lt, require_lte, require_neq,
};

// ── Pinocchio core types (so users only need `jiminy::prelude`) ──────────────
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

// ── Entrypoint macros (re-exported so programs don't need a direct pinocchio dep) ──
pub use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};

// ── CPI helpers (re-exported so programs don't need pinocchio feature flags) ──
pub use pinocchio::cpi;
pub use pinocchio::instruction::{InstructionAccount, InstructionView};
