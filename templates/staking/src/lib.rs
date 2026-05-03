#![cfg_attr(target_os = "solana", no_std)]
//! **{{project-name}}** - A token staking program with segmented accounts.
//!
//! Demonstrates Jiminy's `segmented_layout!` for variable-length data.
//!
//! Instructions:
//! - `0` InitPool - create a staking pool
//! - `1` Stake - add a stake entry (appends to segment)
//! - `2` Unstake - remove a stake entry by index

pub mod processor;
pub mod state;

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint {
    use jiminy::prelude::{
        program_entrypoint, no_allocator, nostd_panic_handler,
        AccountView, Address, ProgramResult,
    };

    program_entrypoint!(process_instruction);
    no_allocator!();
    nostd_panic_handler!();

    #[allow(dead_code)]
    pub fn process_instruction(
        program_id: &Address,
        accounts: &[AccountView],
        instruction_data: &[u8],
    ) -> ProgramResult {
        crate::processor::process(program_id, accounts, instruction_data)
    }
}
