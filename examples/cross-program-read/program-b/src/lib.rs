#![cfg_attr(target_os = "solana", no_std)]
//! **Program B** - Reads Program A's `Vault` account via `load_foreign`.
//!
//! This program does **not** depend on Program A's crate. It declares
//! its own `VaultView` struct with the same fields, sizes, and version.
//! Because `zero_copy_layout!` produces a deterministic `LAYOUT_ID` from
//! the field descriptors, `load_foreign` can verify ABI compatibility at
//! runtime without any compile-time coupling.

pub mod processor;
pub mod state;

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint {
    use jiminy::prelude::{
        hopper_entrypoint, no_allocator, nostd_panic_handler, AccountView, Address, ProgramResult,
    };

    hopper_entrypoint!(process_instruction);
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
