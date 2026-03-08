//! Convenience re-exports for the common jiminy-core usage pattern.
//!
//! ```rust,ignore
//! use jiminy_core::prelude::*;
//! ```

// ── Check functions ──────────────────────────────────────────────────────────
pub use crate::checks::{
    check_account, check_accounts_unique_2, check_accounts_unique_3, check_accounts_unique_4,
    check_closed, check_discriminator, check_executable, check_has_one,
    check_instruction_data_len, check_instruction_data_min, check_keys_eq,
    check_lamports_gte, check_owner, check_pda, check_rent_exempt, check_signer, check_size,
    check_system_program, check_uninitialized, check_version, check_writable, rent_exempt_min,
};

// ── Assert functions ─────────────────────────────────────────────────────────
pub use crate::asserts::{
    assert_address, assert_not_initialized, assert_pda, assert_pda_external,
    assert_pda_with_bump, assert_program,
};
#[cfg(feature = "programs")]
pub use crate::asserts::assert_token_program;

// ── Account header ───────────────────────────────────────────────────────────
pub use crate::header::{
    AccountHeader, body, body_mut, check_header, header_payload, header_payload_mut,
    read_data_len, read_header_flags, read_version, write_header, write_header_with_len,
    HEADER_LEN,
};

// ── Zero-copy IO ─────────────────────────────────────────────────────────────
pub use crate::account_io::{AccountReader, AccountWriter};
pub use crate::cursor::{write_discriminator, zero_init, DataWriter, SliceCursor};
pub use crate::pod::{pod_from_bytes, pod_from_bytes_mut, pod_write, FixedLayout, Pod};

// ── Math ─────────────────────────────────────────────────────────────────────
pub use crate::math::{
    bps_of, bps_of_ceil, checked_add, checked_div, checked_div_ceil, checked_mul,
    checked_mul_div, checked_mul_div_ceil, checked_pow, checked_sub, scale_amount,
    scale_amount_ceil, to_u64,
};

// ── Bit helpers ──────────────────────────────────────────────────────────────
pub use crate::bits::{
    check_any_flag, check_flags, clear_bit, read_bit, read_flags_at, set_bit, toggle_bit,
    write_flags_at,
};

// ── Account lifecycle ────────────────────────────────────────────────────────
pub use crate::lifecycle::{
    safe_close, safe_close_with_sentinel, safe_realloc, safe_realloc_shrink,
    check_not_revived, check_alive, CLOSE_SENTINEL,
};

// ── PDA utilities ────────────────────────────────────────────────────────────
#[cfg(feature = "programs")]
pub use crate::pda::{
    check_ata, check_ata_with_program, derive_ata, derive_ata_with_bump,
    derive_ata_with_program,
};

// ── Account iteration ────────────────────────────────────────────────────────
pub use crate::accounts::AccountList;

// ── Sysvar readers ───────────────────────────────────────────────────────────
#[cfg(feature = "programs")]
pub use crate::sysvar::{
    check_clock_sysvar, check_rent_sysvar, read_clock, read_clock_epoch, read_clock_slot,
    read_clock_timestamp, read_rent_lamports_per_byte_year,
};

// ── Instruction access ───────────────────────────────────────────────────────
pub use crate::instruction::{
    current_index, instruction_count, program_id_at, instruction_data_range,
    instruction_account_key, caller_program, require_top_level, require_cpi_from,
    count_program_invocations, detect_flash_loan_bracket,
    check_no_other_invocation, check_no_subsequent_invocation,
};
#[cfg(feature = "programs")]
pub use crate::instruction::check_has_compute_budget;

// ── Zero-alloc event emission ────────────────────────────────────────────────
pub use crate::event::emit_slices;

// ── Time / deadline checks ───────────────────────────────────────────────────
pub use crate::time::{
    check_cooldown, check_expired, check_not_expired, check_slot_staleness,
    check_within_window,
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

// ── Macros ───────────────────────────────────────────────────────────────────
pub use crate::{
    require, require_accounts_ne, require_eq, require_flag, require_gt, require_gte,
    require_keys_eq, require_keys_neq, require_lt, require_lte, require_neq,
};

// ── Pinocchio core types ─────────────────────────────────────────────────────
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
pub use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};
