#![cfg_attr(target_os = "solana", no_std)]
//! **Program A** - Defines and owns a `Vault` account.

pub mod processor;
pub mod state;

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint {
    use jiminy::prelude::{
        program_entrypoint, no_allocator, nostd_panic_handler, AccountView, Address, ProgramResult,
    };

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
