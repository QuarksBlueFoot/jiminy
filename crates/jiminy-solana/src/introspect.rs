//! Transaction introspection via Sysvar Instructions.
//!
//! Delegates to [`jiminy_core::instruction`] for all parsing logic.
//! This module re-exports with the `read_` prefixed names for backward
//! compatibility.

pub use jiminy_core::instruction::program_id_at as read_program_id_at;
pub use jiminy_core::instruction::instruction_data_range as read_instruction_data_range;
pub use jiminy_core::instruction::instruction_account_key as read_instruction_account_key;

#[cfg(feature = "programs")]
pub use jiminy_core::instruction::check_has_compute_budget;
