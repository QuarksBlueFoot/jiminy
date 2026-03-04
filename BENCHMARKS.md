# Benchmarks

Jiminy's `#[inline(always)]` functions are designed to compile down to
nearly the same BPF instructions as hand-written Pinocchio. The benchmark
suite validates this by comparing two implementations of the same vault program:

| Variant | Description |
|---------|-------------|
| **Raw Pinocchio** | Hand-inlined checks, manual byte arithmetic, no abstractions |
| **Jiminy + Pinocchio** | Same logic using Jiminy's `check_*` functions, `AccountList`, `SliceCursor` |

## Results

Measured via [Mollusk SVM](https://github.com/anza-xyz/mollusk) on Agave 2.3.

### Compute Units

| Instruction | Pinocchio | Jiminy | Delta |
|-------------|-----------|--------|-------|
| Deposit     | 147 CU    | 154 CU | +7    |
| Withdraw    | 254 CU    | 266 CU | +12   |
| Close       | 215 CU    | 228 CU | +13   |
| Guarded Withdraw | 567 CU | 581 CU | +14 |

**Guarded Withdraw** exercises the new DeFi safety modules added in v0.3:
`check_nonzero`, `check_min_amount` (slippage), `check_accounts_unique_3` (checks),
`check_instruction_data_min` (checks), and `checked_mul_div` (math) to compute a
0.3% protocol fee. The Pinocchio version hand-rolls the same logic.

Jiminy adds **7-14 CU** of overhead per instruction. At these levels the cost
is negligible -- a single `sol_log` call costs ~100 CU.

### Binary Size (release SBF)

| Program | Size |
|---------|------|
| `bench_pinocchio_vault.so` | 27.4 KB |
| `bench_jiminy_vault.so`    | 26.5 KB |

Jiminy's binary is **0.9 KB smaller** because `AccountList` and the check
functions let the compiler deduplicate identical guard patterns that
hand-inlined code repeats.

### Security Demo: Missing Signer Check

The benchmark suite includes a `vuln_withdraw` instruction that is deliberately
vulnerable -- identical to a normal withdraw but "forgot" the `is_signer()`
check on the authority account.

The attacker reads a real user's vault on-chain, finds the stored authority
pubkey, and calls `vuln_withdraw` passing the real user's pubkey (unsigned)
and the real vault (owned by the program). All other checks pass: owner,
discriminator, authority match, balance. The program moves lamports.

| Program | CU | Result |
|---------|----|--------|
| Pinocchio | 211 CU | **EXPLOITED** -- 2 SOL drained to attacker |
| Jiminy    |  78 CU | **SAFE** -- `next_signer()` rejected unsigned authority |

In raw Pinocchio, `is_signer()` is one more manual `if` among many -- easy to
forget. In Jiminy, `accs.next_signer()` bundles the signer check into the call
you always use to get the authority account. There is no separate line to omit.

## Running the Benchmarks Yourself

### Prerequisites

- Solana CLI 2.x with the `solana` Rust toolchain
- Rust nightly or stable

### Build the Programs

```sh
# From workspace root
rustup run solana -- cargo build --release --target sbf-solana-solana -p bench-pinocchio-vault
rustup run solana -- cargo build --release --target sbf-solana-solana -p bench-jiminy-vault
```

### Run the CU Comparison

```sh
cd bench/runner
cargo bench
```

This uses [Mollusk](https://github.com/anza-xyz/mollusk) to run each
program's instructions inside the SVM and measure compute units consumed.
