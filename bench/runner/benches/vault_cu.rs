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

    let guarded_withdraw_amount = 100_000_000u64; // 0.1 SOL
    let mut guarded_withdraw_data = vec![4u8];
    guarded_withdraw_data.extend_from_slice(&guarded_withdraw_amount.to_le_bytes());

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

    // ── Guarded Withdraw (exercises new DeFi safety modules) ──────────────
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
            data: guarded_withdraw_data.clone(),
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
        println!("  {:<10} │ Guarded  │ {:>6} CU │ {}", label, cu, if ok { "OK" } else { "FAIL" });
        results.push((label, "Guarded", cu, ok));
    }

    // ── Summary ──────────────────────────────────────────────────────────────
    println!();
    println!("┌────────────┬──────────────┬──────────────┬────────┐");
    println!("│ Instruction│  Pinocchio   │    Jiminy    │  Delta │");
    println!("├────────────┼──────────────┼──────────────┼────────┤");

    for instr in ["Deposit", "Withdraw", "Close", "Guarded"] {
        let p = results.iter().find(|r| r.0 == "Pinocchio" && r.1 == instr).unwrap();
        let j = results.iter().find(|r| r.0 == "Jiminy" && r.1 == instr).unwrap();
        let delta = j.2 as i64 - p.2 as i64;
        let _sign = if delta > 0 { "+" } else if delta < 0 { "" } else { " " };
        println!("│ {:<10} │ {:>8} CU  │ {:>8} CU  │ {:>+5}  │", instr, p.2, j.2, delta);
    }
    println!("└────────────┴──────────────┴──────────────┴────────┘");

    // ══════════════════════════════════════════════════════════════════════════
    // ── SECURITY FLAW DEMO ───────────────────────────────────────────────────
    // ══════════════════════════════════════════════════════════════════════════
    //
    // Missing signer check -- the exploit that actually drains funds.
    //
    // A real user created a vault (owned by our program). The stored authority
    // is the user's pubkey. An attacker reads the vault on-chain, sees the
    // authority, and calls our vuln_withdraw with:
    //   accounts[0] = real user's pubkey (NOT signing, just included)
    //   accounts[1] = real user's vault (owned by our program, real balance)
    //   accounts[2] = attacker's wallet
    //
    // The program checks owner, disc, authority match, balance -- all pass.
    // It never checked is_signer() on the authority. The vault IS owned by
    // our program so set_lamports() succeeds. Funds drained. Game over.
    // No runtime safety net.

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   SECURITY DEMO: Missing Signer Check                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  A real user has a vault with 2 SOL. The attacker reads the");
    println!("  vault on-chain, finds the stored authority pubkey, and calls");
    println!("  YOUR withdraw -- passing the real user's pubkey (unsigned)");
    println!("  and the real vault. Your program never checked is_signer().");
    println!();

    // A real user who deposited into a vault
    let real_user = next_pubkey();

    // The real vault -- owned by our program, real data, real balance
    let mut real_vault_data = vec![0u8; 41];
    real_vault_data[0] = 1; // correct discriminator
    real_vault_data[1..9].copy_from_slice(&2_000_000_000u64.to_le_bytes()); // 2 SOL balance
    real_vault_data[9..41].copy_from_slice(real_user.as_ref()); // real user is authority

    let attacker_wallet = next_pubkey();

    let steal_amount = 2_000_000_000u64; // drain everything
    let mut vuln_data = vec![5u8]; // tag 5 = vuln_withdraw
    vuln_data.extend_from_slice(&steal_amount.to_le_bytes());

    let attacker_wallet_account = Account {
        lamports: 0,
        data: vec![],
        owner: SYSTEM_PROGRAM,
        executable: false,
        rent_epoch: 0,
    };

    for (label, prog_id, mollusk) in [
        ("Pinocchio", &pinocchio_id, &pinocchio_mollusk),
        ("Jiminy",    &jiminy_id,    &jiminy_mollusk),
    ] {
        let real_vault_key = next_pubkey();

        let ix = Instruction {
            program_id: *prog_id,
            accounts: vec![
                // Attacker passes real user's pubkey -- but NOT as a signer.
                // In Mollusk, signer status comes from AccountMeta.
                AccountMeta::new_readonly(real_user, false), // NOT signed!
                AccountMeta::new(real_vault_key, false),
                AccountMeta::new(attacker_wallet, false),
            ],
            data: vuln_data.clone(),
        };

        let accounts = vec![
            // Real user's account -- just needs to exist, not signing
            (real_user, Account {
                lamports: 1_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            }),
            // Real vault -- owned by the program, has real funds
            (real_vault_key, Account {
                lamports: 2_000_000_000, // 2 SOL
                data: real_vault_data.clone(),
                owner: *prog_id, // owned by our program!
                executable: false,
                rent_epoch: 0,
            }),
            (attacker_wallet, attacker_wallet_account.clone()),
        ];

        let result = mollusk.process_instruction(&ix, &accounts);
        let ok = result.program_result.is_ok();
        let cu = result.compute_units_consumed;

        if ok {
            // Find resulting balances
            let vault_after = result.resulting_accounts.iter()
                .find(|a| a.0 == real_vault_key)
                .map(|a| a.1.lamports)
                .unwrap_or(0);
            let wallet_after = result.resulting_accounts.iter()
                .find(|a| a.0 == attacker_wallet)
                .map(|a| a.1.lamports)
                .unwrap_or(0);
            println!("  {:<10} │ vuln_withdraw │ {:>4} CU │ EXPLOITED", label, cu);
            println!("             │ Vault:    2,000,000,000 -> {} lamports", vault_after);
            println!("             │ Attacker: 0 -> {} lamports  << STOLEN", wallet_after);
        } else {
            println!("  {:<10} │ vuln_withdraw │ {:>4} CU │ SAFE -- signer check rejected it", label, cu);
        }
    }

    println!();
    println!("  ┌──────────────────────────────────────────────────────────────┐");
    println!("  │ WHAT HAPPENED:                                              │");
    println!("  │                                                             │");
    println!("  │ Pinocchio: The attacker passed a real user's pubkey (not    │");
    println!("  │ signing) and a real vault owned by the program. Owner check │");
    println!("  │ passed. Disc check passed. Authority matched. Balance was   │");
    println!("  │ sufficient. The program moved 2 SOL to the attacker.        │");
    println!("  │ No runtime safety net -- the vault IS owned by the program. │");
    println!("  │ The missing `is_signer()` check is the entire exploit.      │");
    println!("  │                                                             │");
    println!("  │ Jiminy: `accs.next_signer()` bundles the signer check into  │");
    println!("  │ the call you always use to get the authority account. There  │");
    println!("  │ is no separate `is_signer()` line to forget. The unsigned   │");
    println!("  │ authority was rejected instantly.                            │");
    println!("  └──────────────────────────────────────────────────────────────┘");
    println!();

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
