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
| Deposit     | 146 CU    | 149 CU | +3    |
| Withdraw    | 253 CU    | 266 CU | +13   |
| Close       | 214 CU    | 230 CU | +16   |

Jiminy adds **3–16 CU** of overhead per instruction. At these levels the cost
is negligible — a single `sol_log` call costs ~100 CU.

### Binary Size (release SBF)

| Program | Size |
|---------|------|
| `bench_pinocchio_vault.so` | 18.7 KB |
| `bench_jiminy_vault.so`    | 17.4 KB |

Jiminy's binary is **1.3 KB smaller** (7% reduction) because `AccountList`
and the check functions let the compiler deduplicate identical guard patterns
that hand-inlined code repeats.

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
