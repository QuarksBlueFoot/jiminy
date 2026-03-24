# Reference Templates

Ready-to-use program templates demonstrating idiomatic Jiminy patterns.

## Templates

| Template | Description | Patterns |
|----------|-------------|----------|
| [`vault`](vault/) | SOL vault with deposit/withdraw/close | Init, checked load, safe close, CPI transfer |
| [`escrow`](escrow/) | Time-locked escrow with accept/cancel | Flags, time checks, multi-account close |
| [`staking`](staking/) | Token staking with rewards | Segmented accounts, epoch math, SPL Token CPI |

## Usage

### Quick Start

Copy a template directory into your project:

```bash
cp -r templates/vault my-program
cd my-program
```

Edit `Cargo.toml` to set your program name, then customize `state.rs` and `processor.rs`.

### With `cargo generate` (optional)

If you prefer `cargo generate`, each template is also compatible:

```bash
cargo generate --path templates/vault --name my-vault
```

### Build

```bash
cargo build-sbf
```

## What Each Template Demonstrates

### Vault

The simplest Jiminy program. Demonstrates:

- `zero_copy_layout!` for account definition
- `init_account!` for CPI account creation + header write
- `load_checked` / `load_checked_mut` for validated access
- `check_has_one` for authority verification
- `safe_close` for atomic account closure
- `checked_add` / `checked_sub` for overflow-safe math
- `AccountList` for iterator-style account consumption
- `SliceCursor` for zero-copy instruction parsing

### Escrow

Adds time-based logic and flag management:

- Header flags for state tracking (`is_accepted`)
- `check_time_not_expired` for deadline enforcement
- Multi-party account relationships (creator, recipient)
- Conditional close logic

### Staking

Shows advanced patterns:

- `segmented_layout!` for variable-length stake entries
- SPL Token CPI (transfer, mint-to)
- Epoch-based reward calculation
- Cross-program token account reads via `jiminy-layouts`
- Segment table initialization and element access
