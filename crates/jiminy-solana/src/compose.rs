//! Transaction composition guards.
//!
//! Delegates to [`jiminy_core::instruction`] for all logic. This module
//! re-exports the composition guard functions for backward compatibility.

pub use jiminy_core::instruction::check_no_other_invocation;
pub use jiminy_core::instruction::check_no_subsequent_invocation;
pub use jiminy_core::instruction::detect_flash_loan_bracket;
pub use jiminy_core::instruction::count_program_invocations;
