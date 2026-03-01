#![cfg_attr(target_os = "solana", no_std)]
//! **jiminy-escrow** - A two-party escrow demonstrating close checks and ordering guarantees.
//!
//! Instructions:
//! - `0`: CreateEscrow: lock lamports until the counterparty accepts or timeout expires
//! - `1`: AcceptEscrow: counterparty claims the escrowed lamports
//! - `2`: CancelEscrow: creator reclaims after timeout (or if linked account is closed)

pub mod processor;
pub mod state;

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint {
    use pinocchio::{program_entrypoint, no_allocator, nostd_panic_handler, AccountView, Address, ProgramResult};

    program_entrypoint!(process_instruction);
    no_allocator!();
    nostd_panic_handler!();

    pub fn process_instruction(
        program_id: &Address,
        accounts: &[AccountView],
        instruction_data: &[u8],
    ) -> ProgramResult {
        crate::processor::process(program_id, accounts, instruction_data)
    }
}
