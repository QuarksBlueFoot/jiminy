# Benchmarks

Jiminy's `#[inline(always)]` functions are designed to compile down to the same
BPF instructions as hand-written code. The benchmark suite validates this claim
by comparing three implementations of the same vault program:

| Variant | Description |
|---------|-------------|
| **Raw Pinocchio** | Hand-inlined checks, manual byte arithmetic, no abstractions |
| **Jiminy + Pinocchio** | Same logic using Jiminy's `check_*` functions, `AccountList`, `SliceCursor` |
| **Anchor** | Same logic using Anchor v0.30 with `#[account(...)]` constraints and borsh |

## Running the Benchmarks

### Prerequisites

- Solana CLI with `cargo build-sbf` support
- Rust toolchain

### Build the Programs

```sh
# Build Pinocchio and Jiminy vault programs
cargo build-sbf -p bench-pinocchio-vault
cargo build-sbf -p bench-jiminy-vault

# Build the Anchor variant (requires anchor-lang)
cargo build-sbf -p bench-anchor-vault
```

### Run the CU Comparison

```sh
cargo bench -p bench-runner
```

This uses [Mollusk](https://github.com/anza-xyz/mollusk) to execute each
program's instructions and measure compute units consumed.

## Expected Results

The Pinocchio and Jiminy variants should produce **identical or near-identical
CU counts** — the abstraction cost of Jiminy's functions should be zero after
inlining. If you see a difference greater than 1-2 CU, something is wrong
with the build or the benchmark setup.

The Anchor variant will show the cost of:
- 8-byte discriminator checks (vs Jiminy's 1-byte)
- Borsh deserialization/serialization
- `#[account(...)]` constraint expansion
- Allocator usage

## Binary Size Comparison

After building, compare `.so` file sizes:

```sh
ls -la target/deploy/*.so
```

| Program | Expected Size |
|---------|---------------|
| `bench_pinocchio_vault.so` | ~baseline |
| `bench_jiminy_vault.so` | ~same as baseline |
| `bench_anchor_vault.so` | significantly larger |

## Adding Your Own Benchmarks

The bench runner in `bench/runner/benches/vault_cu.rs` is a standard Rust
bench file using Mollusk. To add new instruction benchmarks:

1. Add a new instruction setup (accounts + data)
2. Call `mollusk.process_instruction(&ix, &accounts)`
3. Read `result.compute_units_consumed`

The `MolluskComputeUnitMatrixBencher` from `mollusk-svm-bencher` can
produce publication-ready markdown tables — see the Mollusk docs for
details.
