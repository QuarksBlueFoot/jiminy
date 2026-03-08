# jiminy-multisig

M-of-N signer checks. Pass a slice of accounts, get back whether enough of
them signed. Four functions, zero allocation, covers every multisig pattern
you'll hit in practice.

`#![no_std]` · `no_alloc` · BPF-safe · Built on [pinocchio](https://github.com/anza-xyz/pinocchio)

Part of the [jiminy](https://crates.io/crates/jiminy) toolkit.

## Install

```toml
[dependencies]
jiminy-multisig = "0.11"
```

## What's inside

| Function | What it does |
|---|---|
| `count_signers` | Count how many accounts in a slice have signed |
| `check_threshold` | Assert at least M of N accounts are signers |
| `check_all_signers` | Assert every account in the slice is a signer |
| `check_any_signer` | Assert at least one account is a signer |

## Quick start

```rust,ignore
use jiminy_multisig::*;

// Require 3-of-5 multisig
check_threshold(&signer_accounts, 3)?;
```

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

If jiminy saved you some CU, donations welcome at `solanadevdao.sol`
(`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`).

## License

Apache-2.0

Apache-2.0
