//! Convenience re-exports for the common Jiminy usage pattern.
//!
//! ```rust,ignore
//! use jiminy::prelude::*;
//! ```
//!
//! One import gives you every check, reader, guard, macro, and CPI helper
//! across all Jiminy subcrates, plus the pinocchio core types and entrypoint
//! macros so downstream programs need no extra direct dependencies.

// ── Ring 1: systems layer (jiminy-core) ──────────────────────────────────────
//
// Account header, zero-copy IO, Pod, cursors, lifecycle, bits, checks/asserts,
// PDA, math, sysvar, time, state, event, instruction, macros, pinocchio types.
pub use jiminy_core::prelude::*;

// ── Ring 2: platform helpers (jiminy-solana) ─────────────────────────────────
//
// Token/mint readers, Token-2022, CPI wrappers & guards, introspection,
// Ed25519, Merkle, authority, balance, compute, oracle, TWAP, upgrade.
//
// NOTE: jiminy_solana::prelude also exports `check_no_other_invocation`,
// `check_no_subsequent_invocation`, `detect_flash_loan_bracket`,
// `count_program_invocations`, and `check_has_compute_budget`.
// These overlap with jiminy_core::instruction (the consolidated module).
// The explicit re-exports below ensure the core versions win.
pub use jiminy_solana::prelude::*;

// ── Ring 3: protocol math (jiminy-finance) ───────────────────────────────────
//
// AMM math, slippage & economic bounds.
pub use jiminy_finance::prelude::*;

// ── Resolve glob conflicts ───────────────────────────────────────────────────
//
// Both jiminy_core::instruction and jiminy_solana::compose define these
// functions independently. We canonicalise on jiminy_core's consolidated
// instruction module; explicit imports win over globs.

pub use jiminy_core::instruction::{
    check_no_other_invocation, check_no_subsequent_invocation,
    count_program_invocations, detect_flash_loan_bracket,
};

#[cfg(feature = "programs")]
pub use jiminy_core::instruction::check_has_compute_budget;

// ── Domain crates ────────────────────────────────────────────────────────────

pub use jiminy_lending::{
    collateralization_ratio_bps, check_healthy, check_liquidatable,
    max_liquidation_amount, liquidation_seize_amount, simple_interest,
    utilization_rate_bps,
};

pub use jiminy_staking::{
    update_reward_per_token, pending_rewards, update_reward_debt,
    emission_rate, rewards_earned, REWARD_PRECISION,
};

pub use jiminy_vesting::{
    vested_amount, check_cliff_reached, unlocked_at_step, claimable, elapsed_steps,
};

pub use jiminy_multisig::{
    count_signers, check_threshold, check_all_signers, check_any_signer,
};

pub use jiminy_distribute::{proportional_split, extract_fee};

// ── Root macros (override core's identical #[macro_export] copies) ────────────

pub use crate::{
    close_account, init_account, require, require_accounts_ne, require_eq, require_flag,
    require_gt, require_gte, require_keys_eq, require_keys_neq, require_lt, require_lte,
    require_neq,
};
