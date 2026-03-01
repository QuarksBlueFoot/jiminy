//! Vault CU benchmark - compares raw Pinocchio vs Jiminy.
//!
//! Requires compiled .so files. Build them first:
//!
//! ```sh
//! rustup run solana -- cargo build --release --target sbf-solana-solana -p bench-pinocchio-vault
//! rustup run solana -- cargo build --release --target sbf-solana-solana -p bench-jiminy-vault
//! # Copy to target/deploy/
//! cp target/sbf-solana-solana/release/bench_pinocchio_vault.so target/deploy/
//! cp target/sbf-solana-solana/release/bench_jiminy_vault.so target/deploy/
//! ```
//!
//! Then from bench/runner/:
//!
//! ```sh
//! cargo bench
//! ```

use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_pubkey::Pubkey;
use solana_instruction::{AccountMeta, Instruction};

const SYSTEM_PROGRAM: Pubkey = Pubkey::new_from_array([0u8; 32]);

static mut COUNTER: u8 = 0;
fn next_pubkey() -> Pubkey {
    let mut bytes = [0u8; 32];
    for i in 0..4 {
        unsafe {
            COUNTER = COUNTER.wrapping_add(37);
            bytes[i] = COUNTER;
        }
    }
    Pubkey::from(bytes)
}

fn main() {
    let pinocchio_id = next_pubkey();
    let jiminy_id = next_pubkey();

    let pinocchio_mollusk =
        Mollusk::new(&pinocchio_id, "../../target/deploy/bench_pinocchio_vault");
    let jiminy_mollusk =
        Mollusk::new(&jiminy_id, "../../target/deploy/bench_jiminy_vault");

    let authority = next_pubkey();
    let vault_key = next_pubkey();
    let recipient_key = next_pubkey();

    // Vault account data (41 bytes: 1 disc + 8 balance + 32 authority).
    let mut vault_data_41 = vec![0u8; 41];
    vault_data_41[0] = 1; // discriminator
    vault_data_41[1..9].copy_from_slice(&1_000_000_000u64.to_le_bytes());
    vault_data_41[9..41].copy_from_slice(authority.as_ref());

    // For deposit: the depositor account must be owned by the program so the
    // runtime allows lamport deduction. We create per-program versions below.
    let authority_account_system = Account {
        lamports: 10_000_000_000,
        data: vec![],
        owner: SYSTEM_PROGRAM,
        executable: false,
        rent_epoch: 0,
    };

    let recipient_account = Account {
        lamports: 1_000_000_000,
        data: vec![],
        owner: SYSTEM_PROGRAM,
        executable: false,
        rent_epoch: 0,
    };

    // ── Instruction data ─────────────────────────────────────────────────────
    let deposit_amount = 500_000_000u64;
    let mut deposit_data = vec![1u8];
    deposit_data.extend_from_slice(&deposit_amount.to_le_bytes());

    let withdraw_amount = 100_000_000u64;
    let mut withdraw_data = vec![2u8];
    withdraw_data.extend_from_slice(&withdraw_amount.to_le_bytes());

    let close_data = vec![3u8];

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         Jiminy Vault CU Benchmark - Pinocchio vs Jiminy    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let mut results: Vec<(&str, &str, u64, bool)> = Vec::new();

    // ── Deposit ──────────────────────────────────────────────────────────────
    for (label, prog_id, mollusk) in [
        ("Pinocchio", &pinocchio_id, &pinocchio_mollusk),
        ("Jiminy",    &jiminy_id,    &jiminy_mollusk),
    ] {
        // Depositor owned by the program so the runtime allows lamport deduction.
        let depositor_account = Account {
            lamports: 10_000_000_000,
            data: vec![],
            owner: *prog_id,
            executable: false,
            rent_epoch: 0,
        };
        let ix = Instruction {
            program_id: *prog_id,
            accounts: vec![
                AccountMeta::new(authority, true),
                AccountMeta::new(vault_key, false),
            ],
            data: deposit_data.clone(),
        };
        let accounts = vec![
            (authority, depositor_account),
            (vault_key, Account {
                lamports: 2_000_000_000,
                data: vault_data_41.clone(),
                owner: *prog_id,
                executable: false,
                rent_epoch: 0,
            }),
        ];
        let result = mollusk.process_instruction(&ix, &accounts);
        let ok = result.program_result.is_ok();
        let cu = result.compute_units_consumed;
        println!("  {:<10} │ Deposit  │ {:>6} CU │ {}", label, cu, if ok { "OK" } else { "FAIL" });
        results.push((label, "Deposit", cu, ok));
    }

    // ── Withdraw ─────────────────────────────────────────────────────────────
    for (label, prog_id, mollusk) in [
        ("Pinocchio", &pinocchio_id, &pinocchio_mollusk),
        ("Jiminy",    &jiminy_id,    &jiminy_mollusk),
    ] {
        let ix = Instruction {
            program_id: *prog_id,
            accounts: vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(vault_key, false),
                AccountMeta::new(recipient_key, false),
            ],
            data: withdraw_data.clone(),
        };
        let accounts = vec![
            (authority, authority_account_system.clone()),
            (vault_key, Account {
                lamports: 2_000_000_000,
                data: vault_data_41.clone(),
                owner: *prog_id,
                executable: false,
                rent_epoch: 0,
            }),
            (recipient_key, recipient_account.clone()),
        ];
        let result = mollusk.process_instruction(&ix, &accounts);
        let ok = result.program_result.is_ok();
        let cu = result.compute_units_consumed;
        println!("  {:<10} │ Withdraw │ {:>6} CU │ {}", label, cu, if ok { "OK" } else { "FAIL" });
        results.push((label, "Withdraw", cu, ok));
    }

    // ── Close ────────────────────────────────────────────────────────────────
    for (label, prog_id, mollusk) in [
        ("Pinocchio", &pinocchio_id, &pinocchio_mollusk),
        ("Jiminy",    &jiminy_id,    &jiminy_mollusk),
    ] {
        let ix = Instruction {
            program_id: *prog_id,
            accounts: vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(vault_key, false),
                AccountMeta::new(recipient_key, false),
            ],
            data: close_data.clone(),
        };
        let accounts = vec![
            (authority, authority_account_system.clone()),
            (vault_key, Account {
                lamports: 2_000_000_000,
                data: vault_data_41.clone(),
                owner: *prog_id,
                executable: false,
                rent_epoch: 0,
            }),
            (recipient_key, recipient_account.clone()),
        ];
        let result = mollusk.process_instruction(&ix, &accounts);
        let ok = result.program_result.is_ok();
        let cu = result.compute_units_consumed;
        println!("  {:<10} │ Close    │ {:>6} CU │ {}", label, cu, if ok { "OK" } else { "FAIL" });
        results.push((label, "Close", cu, ok));
    }

    // ── Summary ──────────────────────────────────────────────────────────────
    println!();
    println!("┌────────────┬──────────────┬──────────────┬────────┐");
    println!("│ Instruction│  Pinocchio   │    Jiminy    │  Delta │");
    println!("├────────────┼──────────────┼──────────────┼────────┤");

    for instr in ["Deposit", "Withdraw", "Close"] {
        let p = results.iter().find(|r| r.0 == "Pinocchio" && r.1 == instr).unwrap();
        let j = results.iter().find(|r| r.0 == "Jiminy" && r.1 == instr).unwrap();
        let delta = j.2 as i64 - p.2 as i64;
        let _sign = if delta > 0 { "+" } else if delta < 0 { "" } else { " " };
        println!("│ {:<10} │ {:>8} CU  │ {:>8} CU  │ {:>+5}  │", instr, p.2, j.2, delta);
    }
    println!("└────────────┴──────────────┴──────────────┴────────┘");

    // Binary sizes
    println!();
    println!("Binary sizes (release SBF):");
    for name in ["bench_pinocchio_vault.so", "bench_jiminy_vault.so"] {
        let path = format!("../../target/deploy/{}", name);
        if let Ok(meta) = std::fs::metadata(&path) {
            println!("  {} - {:.1} KB", name, meta.len() as f64 / 1024.0);
        }
    }

    // Also check sbf-solana-solana path
    for name in ["bench_pinocchio_vault.so", "bench_jiminy_vault.so"] {
        let path = format!("../../target/sbf-solana-solana/release/{}", name);
        if let Ok(meta) = std::fs::metadata(&path) {
            println!("  {} - {:.1} KB (sbf-solana-solana)", name, meta.len() as f64 / 1024.0);
        }
    }
    println!();
}
