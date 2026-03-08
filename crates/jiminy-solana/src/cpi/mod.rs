//! Safe CPI wrappers, reentrancy guards, and return data readers.
//!
//! ```rust,ignore
//! use jiminy_solana::cpi::{safe_transfer_tokens, check_no_cpi_caller, read_return_u64};
//! ```

pub mod guard;
pub mod return_data;
pub mod safe;

// ── Re-exports: safe wrappers ────────────────────────────────────────────────
pub use safe::{
    safe_burn, safe_checked_transfer, safe_close_token_account, safe_create_account,
    safe_create_account_signed, safe_mint_to, safe_mint_to_signed, safe_transfer_sol,
    safe_transfer_tokens, safe_transfer_tokens_signed, transfer_lamports,
};

// ── Re-exports: reentrancy guard ─────────────────────────────────────────────
pub use guard::get_instruction_index;
pub use guard::get_num_instructions;
#[cfg(feature = "programs")]
pub use guard::{check_cpi_caller, check_no_cpi_caller, check_sysvar_instructions};

// ── Re-exports: return data ──────────────────────────────────────────────────
pub use return_data::{read_return_data, read_return_data_from, read_return_u64, MAX_RETURN_DATA};
