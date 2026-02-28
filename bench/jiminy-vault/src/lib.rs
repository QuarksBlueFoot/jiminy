#![cfg_attr(target_os = "solana", no_std)]
//! Jiminy vault — same logic as bench-pinocchio-vault, using Jiminy helpers.
//!
//! Uses the same 1-byte discriminator layout (41 bytes) for a fair CU comparison.

pub mod processor;

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
