//! Convenience re-exports for the common Jiminy usage pattern.
//!
//! ```rust,ignore
//! use jiminy::prelude::*;
//! ```

// ── Check functions ──────────────────────────────────────────────────────────
pub use crate::checks::{
    check_account, check_closed, check_discriminator, check_executable, check_has_one,
    check_keys_eq, check_lamports_gte, check_owner, check_pda, check_rent_exempt,
    check_signer, check_size, check_system_program, check_uninitialized, check_writable,
    rent_exempt_min,
};

// ── Assert functions (PDA, address, program) ─────────────────────────────────
pub use crate::asserts::{
    assert_address, assert_not_initialized, assert_pda, assert_pda_external,
    assert_pda_with_bump, assert_program, assert_token_program,
};

// ── Token account readers ────────────────────────────────────────────────────
pub use crate::token::{
    token_account_amount, token_account_delegate, token_account_mint, token_account_owner,
    TOKEN_ACCOUNT_LEN,
};

// ── Cursors ──────────────────────────────────────────────────────────────────
pub use crate::cursor::{write_discriminator, zero_init, DataWriter, SliceCursor};

// ── Account header (v1 convention) ───────────────────────────────────────────
pub use crate::header::{
    check_header, header_payload, header_payload_mut, read_data_len, read_header_flags,
    read_version, write_header, write_header_with_len, HEADER_LEN,
};

// ── Math ─────────────────────────────────────────────────────────────────────
pub use crate::math::{checked_add, checked_mul, checked_sub};

// ── Bit helpers ──────────────────────────────────────────────────────────────
pub use crate::bits::{
    check_any_flag, check_flags, clear_bit, read_bit, read_flags_at, set_bit, toggle_bit,
    write_flags_at,
};

// ── Account lifecycle ────────────────────────────────────────────────────────
pub use crate::close::safe_close;

// ── Account iteration ────────────────────────────────────────────────────────
pub use crate::accounts::AccountList;

// ── Macros (re-exported from crate root via #[macro_export]) ─────────────────
pub use crate::{
    require, require_accounts_ne, require_eq, require_flag, require_gt, require_gte,
    require_keys_eq, require_keys_neq, require_lt, require_lte, require_neq,
};

// ── Pinocchio core types (so users only need `jiminy::prelude`) ──────────────
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

// ── CPI helpers (re-exported so programs don't need pinocchio feature flags) ──
pub use pinocchio::cpi;
pub use pinocchio::instruction::{InstructionAccount, InstructionView};
