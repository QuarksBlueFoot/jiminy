#![cfg_attr(target_os = "solana", no_std)]
//! **jiminy-vault** - A minimal vault program demonstrating Jiminy's safety primitives.
//!
//! Instructions:
//! - `0`: InitVault: create a new vault account
//! - `1`: Deposit: add lamports to a vault
//! - `2`: Withdraw: remove lamports from a vault
//! - `3`: CloseVault: close a vault and reclaim rent

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
