#![no_std]
//! **Jiminy** - safety abstractions for [pinocchio](https://docs.rs/pinocchio) programs, minus the footguns.
//!
//! Pinocchio is the engine. Jiminy keeps it honest.
//!
//! Zero-copy, `no_std`, `no_alloc`, BPF-safe. Account checks, token + mint
//! readers, Token-2022 extension screening, CPI reentrancy guards, DeFi math
//! with u128 intermediates, slippage protection, zero-alloc event emission,
//! merkle proof verification, Ed25519 precompile helpers, transaction
//! introspection, authority handoff, Pyth oracle readers, AMM math (isqrt,
//! constant-product), balance delta guards, close revival sentinels, staking
//! reward accumulators, vesting schedules, multi-signer thresholds,
//! time/deadline checks, state machine validation, zero-copy cursors,
//! and more. All `#[inline(always)]`.
//!
//! # Quick-start
//!
//! ```rust,ignore
//! use jiminy::prelude::*;
//! ```
//!
//! One import gives you everything: account checks, token/mint readers,
//! Token-2022 screening, CPI guards, DeFi math, slippage, time checks,
//! state machines, cursors, macros, `AccountList`, and the pinocchio
//! core types.
//!
//! # Account validation
//!
//! | Function | What it verifies |
//! |---|---|
//! | [`check_signer`] | account is a transaction signer |
//! | [`check_writable`] | account is marked writable |
//! | [`check_owner`] | account is owned by your program |
//! | [`check_pda`] | account address equals a derived PDA |
//! | [`check_system_program`] | account is the system program |
//! | [`check_executable`] | account is an executable program |
//! | [`check_uninitialized`] | account has no data yet (anti-reinit) |
//! | [`check_has_one`] | stored address field matches account key |
//! | [`check_keys_eq`] | two addresses are equal |
//! | [`check_lamports_gte`] | account holds at least N lamports |
//! | [`check_rent_exempt`] | account holds enough lamports for rent exemption |
//! | [`check_closed`] | account has zero lamports and empty data |
//! | [`check_size`] | raw data slice is at least N bytes |
//! | [`check_discriminator`] | first byte matches expected type tag |
//! | [`check_account`] | owner + size + discriminator in one call |
//! | [`check_accounts_unique_2`] | two accounts have different addresses |
//! | [`check_accounts_unique_3`] | three accounts all different (src ≠ dest ≠ fee) |
//! | [`check_accounts_unique_4`] | four accounts all different (two-hop swaps) |
//! | [`check_instruction_data_len`] | exact instruction data length |
//! | [`check_instruction_data_min`] | minimum instruction data length |
//! | [`check_version`] | header version byte ≥ minimum |
//! | [`rent_exempt_min`] | compute minimum lamports for rent exemption |
//!
//! # Assert functions
//!
//! | Function | What it does |
//! |---|---|
//! | [`assert_pda`] | derive PDA, verify match, return bump |
//! | [`assert_pda_with_bump`] | verify PDA with known bump (cheaper) |
//! | [`assert_pda_external`] | same as `assert_pda` for external programs |
//! | [`assert_token_program`] | account is SPL Token or Token-2022 |
//! | [`assert_address`] | account address matches expected key |
//! | [`assert_program`] | address matches + account is executable |
//! | [`assert_not_initialized`] | lamports == 0 (account doesn't exist yet) |
//!
//! # Token account readers + checks
//!
//! Zero-copy reads from the 165-byte SPL Token layout.
//!
//! | Function | What it reads / checks |
//! |---|---|
//! | [`token_account_owner`] | owner address (bytes 32..64) |
//! | [`token_account_amount`] | token balance as u64 (bytes 64..72) |
//! | [`token_account_mint`] | mint address (bytes 0..32) |
//! | [`token_account_delegate`] | optional delegate address |
//! | [`token_account_state`] | state byte (0=uninit, 1=init, 2=frozen) |
//! | [`token_account_close_authority`] | optional close authority |
//! | [`token_account_delegated_amount`] | delegated amount (u64) |
//! | [`check_token_account_mint`] | mint matches expected |
//! | [`check_token_account_owner`] | owner matches expected |
//! | [`check_token_account_initialized`] | state == 1 |
//! | [`check_no_delegate`] | no active delegate |
//! | [`check_no_close_authority`] | no close authority set |
//! | [`check_token_balance_gte`] | token balance ≥ minimum |
//! | [`check_token_program_match`] | account owned by the right token program |
//!
//! # Mint account readers + checks
//!
//! Zero-copy reads from the 82-byte SPL Mint layout.
//!
//! | Function | What it reads / checks |
//! |---|---|
//! | [`mint_authority`] | optional mint authority address |
//! | [`mint_supply`] | total supply (u64) |
//! | [`mint_decimals`] | decimals (u8) |
//! | [`mint_is_initialized`] | is initialized (bool) |
//! | [`mint_freeze_authority`] | optional freeze authority |
//! | [`check_mint_owner`] | mint owned by expected token program |
//! | [`check_mint_authority`] | mint authority matches expected |
//!
//! # Token-2022 extension screening
//!
//! Programs accepting Token-2022 tokens **must** screen for dangerous extensions.
//! See the [`token_2022`] module for full TLV extension reading and one-line
//! safety guards like [`token_2022::check_safe_token_2022_mint`].
//!
//! # CPI reentrancy protection
//!
//! See the [`cpi_guard`] module. Reads the Sysvar Instructions account to
//! detect CPI callers: [`cpi_guard::check_no_cpi_caller`],
//! [`cpi_guard::check_cpi_caller`].
//!
//! # Transaction introspection
//!
//! See the [`introspect`] module. Read any instruction in the current
//! transaction: [`introspect::read_program_id_at`],
//! [`introspect::read_instruction_data_range`],
//! [`introspect::read_instruction_account_key`],
//! [`introspect::check_has_compute_budget`].
//!
//! # Ed25519 precompile verification
//!
//! See the [`ed25519`] module. Verify Ed25519 precompile signatures
//! from the Sysvar Instructions data: [`ed25519::check_ed25519_signature`],
//! [`ed25519::check_ed25519_signer`].
//!
//! # Authority handoff (two-step rotation)
//!
//! See the [`authority`] module. Safe propose + accept authority transfer:
//! [`authority::check_pending_authority`], [`authority::write_pending_authority`],
//! [`authority::accept_authority`].
//!
//! # Merkle proof verification
//!
//! See the [`merkle`] module. Zero-alloc tree verification via `sol_sha256`:
//! [`merkle::verify_merkle_proof`], [`merkle::sha256_leaf`].
//!
//! # Zero-alloc event emission
//!
//! See the [`event`] module. Emit structured events via `sol_log_data`:
//! [`event::emit_slices`] and the [`emit!`] macro.
//!
//! # Pyth oracle readers
//!
//! See the [`oracle`] module. Zero-copy Pyth V2 price feed reading at fixed
//! byte offsets, no `pyth-sdk-solana` dependency:
//! [`oracle::read_pyth_price`], [`oracle::read_pyth_ema`],
//! [`oracle::check_pyth_price_fresh`], [`oracle::check_pyth_confidence`].
//!
//! # AMM math
//!
//! See the [`amm`] module. Integer square root, constant-product swap math,
//! k-invariant verification, LP token minting:
//! [`amm::isqrt`], [`amm::constant_product_out`], [`amm::constant_product_in`],
//! [`amm::check_k_invariant`], [`amm::price_impact_bps`],
//! [`amm::initial_lp_amount`], [`amm::proportional_lp_amount`].
//!
//! # Balance delta (safe swap guard)
//!
//! See the [`balance`] module. Pre/post CPI balance verification for safe
//! swap composition:
//! [`balance::snapshot_token_balance`], [`balance::check_balance_increased`],
//! [`balance::check_balance_decreased`], [`balance::check_balance_delta`].
//!
//! # Close revival sentinel
//!
//! The [`close`] module now includes [`safe_close_with_sentinel`] to defend
//! against Sealevel Attack #9 (account revival). Also:
//! [`check_not_revived`], [`check_alive`].
//!
//! # Staking rewards math
//!
//! See the [`staking`] module. MasterChef reward-per-token accumulator:
//! [`staking::update_reward_per_token`], [`staking::pending_rewards`],
//! [`staking::update_reward_debt`], [`staking::emission_rate`].
//!
//! # Vesting schedules
//!
//! See the [`vesting`] module. Linear + cliff, stepped, and periodic unlock:
//! [`vesting::vested_amount`], [`vesting::unlocked_at_step`],
//! [`vesting::claimable`], [`vesting::elapsed_steps`].
//!
//! # Multi-signer threshold
//!
//! See the [`multisig`] module. M-of-N signature checking with duplicate
//! prevention:
//! [`multisig::check_threshold`], [`multisig::count_signers`],
//! [`multisig::check_all_signers`], [`multisig::check_any_signer`].
//!
//! # DeFi math
//!
//! | Function | What it does |
//! |---|---|
//! | [`checked_add`] | overflow-safe u64 addition |
//! | [`checked_sub`] | underflow-safe u64 subtraction |
//! | [`checked_mul`] | overflow-safe u64 multiplication |
//! | [`checked_div`] | division with zero check |
//! | [`checked_div_ceil`] | ceiling division |
//! | [`checked_mul_div`] | `(a * b) / c` with u128 intermediate |
//! | [`checked_mul_div_ceil`] | same, ceiling |
//! | [`bps_of`] | basis point fee: `amount * bps / 10_000` |
//! | [`bps_of_ceil`] | same, ceiling |
//! | [`checked_pow`] | exponentiation via repeated squaring |
//! | [`to_u64`] | safe u128 → u64 narrowing |
//! | [`scale_amount`] | decimal-aware token amount conversion |
//! | [`scale_amount_ceil`] | same, ceiling (protocol-side math) |
//!
//! # Slippage + economic bounds
//!
//! See the [`slippage`] module: [`slippage::check_slippage`],
//! [`slippage::check_min_amount`], [`slippage::check_nonzero`],
//! [`slippage::check_within_bps`], [`slippage::check_price_bounds`], and more.
//!
//! # Time + deadline checks
//!
//! | Function | What it does |
//! |---|---|
//! | [`check_not_expired`] | current time ≤ deadline |
//! | [`check_expired`] | current time > deadline |
//! | [`check_within_window`] | time is within [start, end] |
//! | [`check_cooldown`] | rate limiting (oracle updates, admin changes) |
//! | [`check_deadline`] | read Clock sysvar + check not expired |
//! | [`check_after`] | read Clock sysvar + check expired |
//! | [`check_slot_staleness`] | slot-based oracle/data feed staleness |
//!
//! # Sysvar readers
//!
//! | Function | What it does |
//! |---|---|
//! | [`read_clock`] | read (slot, timestamp) from Clock sysvar |
//! | [`read_clock_epoch`] | read epoch from Clock sysvar |
//! | [`read_rent`] | read (lamports_per_byte_year, exemption_threshold) from Rent |
//!
//! # State machine validation
//!
//! See the [`state`] module: [`state::check_state`],
//! [`state::check_state_transition`], [`state::write_state`],
//! [`state::check_state_not`], [`state::check_state_in`].
//!
//! # PDA utilities
//!
//! | Macro / Function | What it does |
//! |---|---|
//! | [`find_pda!`] | find canonical PDA + bump via syscall |
//! | [`derive_pda!`] | derive PDA with known bump (cheap, ~100 CU) |
//! | [`derive_pda_const!`] | derive PDA at compile time |
//! | [`derive_ata`] | derive ATA address for wallet + mint |
//! | [`derive_ata_with_program`] | derive ATA with explicit token program |
//! | [`derive_ata_with_bump`] | derive ATA with known bump (cheap) |
//! | [`derive_ata_const!`] | derive ATA at compile time |
//!
//! # Zero-copy cursors
//!
//! [`SliceCursor`] reads typed fields sequentially from account data.
//! [`DataWriter`] writes them when initializing a new account.
//! [`zero_init`] zero-fills account data before the first write.
//!
//! # Account iteration
//!
//! [`AccountList`] provides iterator-style account consumption with
//! inline constraint checks, replacing manual index arithmetic.
//!
//! # Well-known program IDs
//!
//! [`programs`] module: `SYSTEM`, `TOKEN`, `TOKEN_2022`, `ASSOCIATED_TOKEN`,
//! `METADATA`, `BPF_LOADER`, `COMPUTE_BUDGET`, `SYSVAR_CLOCK`, `SYSVAR_RENT`,
//! `SYSVAR_INSTRUCTIONS`. Also [`ed25519::ED25519_PROGRAM`].

#[cfg(feature = "programs")]
pub mod programs;

mod accounts;
pub mod amm;
mod asserts;
pub mod authority;
pub mod balance;
mod bits;
mod checks;
mod close;
pub mod cpi;
pub mod cpi_guard;
mod cursor;
pub mod ed25519;
pub mod event;
mod header;
pub mod introspect;
#[cfg(feature = "log")]
pub mod log;
mod math;
pub mod merkle;
mod mint;
pub mod multisig;
pub mod oracle;
mod pda;
pub mod prelude;
pub mod realloc;
pub mod slippage;
pub mod staking;
pub mod state;
mod sysvar;
mod time;
mod token;
pub mod token_2022;
pub mod vesting;

pub use accounts::AccountList;
pub use asserts::*;
pub use bits::*;
pub use checks::*;
pub use close::*;
pub use cursor::{write_discriminator, zero_init, DataWriter, SliceCursor};
pub use header::*;
pub use math::*;
pub use mint::*;
pub use pda::*;
pub use sysvar::*;
pub use time::*;
pub use token::*;

// Re-export the full pinocchio crate so existing pinocchio users can switch
// their imports to `jiminy::pinocchio::*` and drop the direct dependency.
pub use pinocchio;

// Re-export pinocchio companion crates for system and token CPI.
pub use pinocchio_system;
pub use pinocchio_token;

// Also re-export the most common types at crate root for convenience.
pub use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

// ── Macros ───────────────────────────────────────────────────────────────────

/// Require a boolean condition: return `$err` (converted via `Into`) if false.
///
/// Equivalent to Anchor's `require!`.
///
/// ```rust,ignore
/// require!(amount > 0, MyError::ZeroAmount);
/// ```
#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be equal.
///
/// ```rust,ignore
/// require_keys_eq!(vault.owner(), &expected_owner, MyError::WrongOwner);
/// ```
#[macro_export]
macro_rules! require_keys_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a != *$b {
            return Err($err.into());
        }
    };
}

/// Require two [`Address`] values to be **different**.
///
/// The counterpart to `require_keys_eq!`. Useful for authority rotation
/// checks, multi-hop validations, or any time two program-owned addresses
/// must not collide.
///
/// ```rust,ignore
/// require_keys_neq!(old_authority, new_authority, MyError::SameKey);
/// ```
#[macro_export]
macro_rules! require_keys_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if *$a == *$b {
            return Err($err.into());
        }
    };
}

/// Require two accounts to have **different** addresses.
///
/// Prevents source == destination attacks common in token and escrow
/// programs. Anchor has no built-in constraint for this case.
///
/// ```rust,ignore
/// require_accounts_ne!(source_vault, dest_vault, MyError::SameAccount);
/// ```
#[macro_export]
macro_rules! require_accounts_ne {
    ($a:expr, $b:expr, $err:expr) => {
        if $a.address() == $b.address() {
            return Err($err.into());
        }
    };
}

/// Require `a >= b`.
///
/// ```rust,ignore
/// require_gte!(balance, amount, MyError::InsufficientFunds);
/// ```
#[macro_export]
macro_rules! require_gte {
    ($a:expr, $b:expr, $err:expr) => {
        if $a < $b {
            return Err($err.into());
        }
    };
}

/// Require `a > b`.
#[macro_export]
macro_rules! require_gt {
    ($a:expr, $b:expr, $err:expr) => {
        if $a <= $b {
            return Err($err.into());
        }
    };
}

/// Require `a < b`.
#[macro_export]
macro_rules! require_lt {
    ($a:expr, $b:expr, $err:expr) => {
        if $a >= $b {
            return Err($err.into());
        }
    };
}

/// Require `a <= b`.
#[macro_export]
macro_rules! require_lte {
    ($a:expr, $b:expr, $err:expr) => {
        if $a > $b {
            return Err($err.into());
        }
    };
}

/// Require `a == b` for scalar types.
#[macro_export]
macro_rules! require_eq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a != $b {
            return Err($err.into());
        }
    };
}

/// Require `a != b` for scalar types.
///
/// The counterpart to `require_eq!`. Use `require_keys_neq!` for addresses.
///
/// ```rust,ignore
/// require_neq!(new_value, current_value, MyError::NoChange);
/// ```
#[macro_export]
macro_rules! require_neq {
    ($a:expr, $b:expr, $err:expr) => {
        if $a == $b {
            return Err($err.into());
        }
    };
}

/// Require bit `n` to be set in `$byte`, else return `$err`.
///
/// ```rust,ignore
/// require_flag!(state.flags, 0, MyError::AccountLocked);
/// ```
#[macro_export]
macro_rules! require_flag {
    ($byte:expr, $n:expr, $err:expr) => {
        if ($byte >> $n) & 1 == 0 {
            return Err($err.into());
        }
    };
}
