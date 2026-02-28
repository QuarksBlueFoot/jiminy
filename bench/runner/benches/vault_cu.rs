//! Vault CU benchmark — compares raw Pinocchio vs Jiminy vs Anchor.
//!
//! Requires compiled .so files for each program. Build them first:
//!
//! ```sh
//! cargo build-sbf -p bench-pinocchio-vault
//! cargo build-sbf -p bench-jiminy-vault
//! cargo build-sbf -p bench-anchor-vault
//! ```
//!
//! Then run:
//!
//! ```sh
//! cargo bench -p bench-runner
//! ```
//!
//! Output: a markdown table of CU consumed per instruction per variant.

use mollusk_svm::Mollusk;
use mollusk_svm_bencher::MolluskComputeUnitBencher;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

fn main() {
    // Program IDs — arbitrary, just need to be unique.
    let pinocchio_id = Pubkey::new_unique();
    let jiminy_id = Pubkey::new_unique();
    // Anchor program has a declared ID but we can override for testing.
    let anchor_id = Pubkey::new_unique();

    // ── Setup Mollusk instances ──────────────────────────────────────────────
    //
    // Each loads the compiled ELF from target/deploy/.
    // If .so files are not found, the bench will fail with a clear error.

    let pinocchio_mollusk = Mollusk::new(&pinocchio_id, "../../target/deploy/bench_pinocchio_vault");
    let jiminy_mollusk = Mollusk::new(&jiminy_id, "../../target/deploy/bench_jiminy_vault");
    // Note: Anchor program needs `anchor build` or `cargo build-sbf` to produce .so
    // let anchor_mollusk = Mollusk::new(&anchor_id, "../../target/deploy/bench_anchor_vault");

    // ── Shared test data ─────────────────────────────────────────────────────

    let authority = Pubkey::new_unique();
    let vault_key = Pubkey::new_unique();
    let recipient_key = Pubkey::new_unique();

    // Vault account data (41 bytes for pinocchio/jiminy variant).
    let mut vault_data_41 = vec![0u8; 41];
    vault_data_41[0] = 1; // discriminator
    vault_data_41[1..9].copy_from_slice(&1_000_000_000u64.to_le_bytes()); // balance = 1 SOL
    vault_data_41[9..41].copy_from_slice(authority.as_ref()); // authority

    let vault_account = Account {
        lamports: 2_000_000_000, // 2 SOL
        data: vault_data_41.clone(),
        owner: pinocchio_id, // will be overridden per variant
        executable: false,
        rent_epoch: 0,
    };

    // ── Deposit instruction (tag=1, amount=500_000_000) ──────────────────────

    let deposit_amount = 500_000_000u64;
    let mut deposit_data = vec![1u8]; // tag
    deposit_data.extend_from_slice(&deposit_amount.to_le_bytes());

    // ── Withdraw instruction (tag=2, amount=100_000_000) ─────────────────────

    let withdraw_amount = 100_000_000u64;
    let mut withdraw_data = vec![2u8]; // tag
    withdraw_data.extend_from_slice(&withdraw_amount.to_le_bytes());

    // ── Pinocchio benches ────────────────────────────────────────────────────

    println!("## Pinocchio (raw) — Deposit");
    {
        let ix = Instruction {
            program_id: pinocchio_id,
            accounts: vec![
                AccountMeta::new(authority, true),  // depositor
                AccountMeta::new(vault_key, false),  // vault
            ],
            data: deposit_data.clone(),
        };

        let mut vault_acc = vault_account.clone();
        vault_acc.owner = pinocchio_id;

        let accounts = vec![
            (authority, Account { lamports: 10_000_000_000, data: vec![], owner: system_program::id(), executable: false, rent_epoch: 0 }),
            (vault_key, vault_acc),
        ];

        let result = pinocchio_mollusk.process_instruction(&ix, &accounts);
        println!("  Result: {:?}", result.program_result);
        println!("  CU consumed: {}", result.compute_units_consumed);
    }

    println!("\n## Pinocchio (raw) — Withdraw");
    {
        let ix = Instruction {
            program_id: pinocchio_id,
            accounts: vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(vault_key, false),
                AccountMeta::new(recipient_key, false),
            ],
            data: withdraw_data.clone(),
        };

        let mut vault_acc = vault_account.clone();
        vault_acc.owner = pinocchio_id;

        let accounts = vec![
            (authority, Account { lamports: 10_000_000_000, data: vec![], owner: system_program::id(), executable: false, rent_epoch: 0 }),
            (vault_key, vault_acc),
            (recipient_key, Account { lamports: 1_000_000_000, data: vec![], owner: system_program::id(), executable: false, rent_epoch: 0 }),
        ];

        let result = pinocchio_mollusk.process_instruction(&ix, &accounts);
        println!("  Result: {:?}", result.program_result);
        println!("  CU consumed: {}", result.compute_units_consumed);
    }

    // ── Jiminy benches ───────────────────────────────────────────────────────

    println!("\n## Jiminy — Deposit");
    {
        let ix = Instruction {
            program_id: jiminy_id,
            accounts: vec![
                AccountMeta::new(authority, true),
                AccountMeta::new(vault_key, false),
            ],
            data: deposit_data.clone(),
        };

        let mut vault_acc = vault_account.clone();
        vault_acc.owner = jiminy_id;

        let accounts = vec![
            (authority, Account { lamports: 10_000_000_000, data: vec![], owner: system_program::id(), executable: false, rent_epoch: 0 }),
            (vault_key, vault_acc),
        ];

        let result = jiminy_mollusk.process_instruction(&ix, &accounts);
        println!("  Result: {:?}", result.program_result);
        println!("  CU consumed: {}", result.compute_units_consumed);
    }

    println!("\n## Jiminy — Withdraw");
    {
        let ix = Instruction {
            program_id: jiminy_id,
            accounts: vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(vault_key, false),
                AccountMeta::new(recipient_key, false),
            ],
            data: withdraw_data.clone(),
        };

        let mut vault_acc = vault_account.clone();
        vault_acc.owner = jiminy_id;

        let accounts = vec![
            (authority, Account { lamports: 10_000_000_000, data: vec![], owner: system_program::id(), executable: false, rent_epoch: 0 }),
            (vault_key, vault_acc),
            (recipient_key, Account { lamports: 1_000_000_000, data: vec![], owner: system_program::id(), executable: false, rent_epoch: 0 }),
        ];

        let result = jiminy_mollusk.process_instruction(&ix, &accounts);
        println!("  Result: {:?}", result.program_result);
        println!("  CU consumed: {}", result.compute_units_consumed);
    }

    println!("\n─── Done ───");
    println!("Build the Anchor variant with `anchor build` and uncomment the Anchor section to add it to the comparison.");
}
