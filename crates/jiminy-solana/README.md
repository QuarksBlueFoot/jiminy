# jiminy-solana

Token/mint readers, Token-2022 screening, CPI guards, Ed25519, Merkle proofs, Pyth oracles, authority rotation, TWAP, compute guards. The Solana platform layer on top of `jiminy-core`.

`#![no_std]` / `no_alloc` / BPF-safe

```toml
[dependencies]
jiminy-solana = "0.16"
```

Builds on `jiminy-core` and Hopper Runtime's Solana-facing account surface.
In this workspace, that runtime is wired to the pinocchio backend.

## What's in here

| | |
|---|---|
| `token` | SPL Token account readers, mint readers, Token-2022 extension screening |
| `cpi` | Safe CPI wrappers, reentrancy guards, return-data readers |
| `crypto` | Ed25519 precompile verification, Merkle proof verification |
| `authority` | Two-step authority rotation (propose + accept) |
| `balance` | Pre/post CPI balance-delta guards |
| `compute` | Compute-budget guards |
| `compose` | Transaction-composition guards (flash-loan detection) |
| `introspect` | Raw transaction introspection |
| `oracle` | Pyth V2 price-feed readers |
| `twap` | TWAP accumulators and deviation checks |
| `upgrade` | Program upgrade-authority verification *(feature-gated)* |

```rust,ignore
use jiminy_solana::prelude::*;

let balance = token_account_amount(token_account)?;
let valid = verify_merkle_proof(&proof, &root, &leaf);
```

## Token-2022 extension screening

The `token::ext` module ships kind-aware TLV walkers. `find_extension_mint` /
`find_extension_account` (and the matching `check_no_extension_mint` /
`check_no_extension_account`) verify the account-type discriminator at byte
165 against the caller's expectation, so a Token-2022 *account* buffer passed
where a *mint* was expected (or vice versa) fails closed as
`InvalidAccountData` instead of silently returning `Ok(())`. The convenience
guards (`check_no_transfer_fee`, `check_no_transfer_hook`,
`check_no_permanent_delegate`, `check_no_default_account_state`,
`check_not_non_transferable`, plus the composite `check_safe_token_2022_mint`)
all route through the mint walker; `check_no_cpi_guard` routes through the
account walker. The untyped `find_extension` / `has_extension` /
`check_no_extension` primitives are kept for advanced callers (Pinocchio
users, custom kinds) that have already verified the buffer kind.

```rust,ignore
let data = mint_account.try_borrow()?;
check_safe_token_2022_mint(&data)?;        // rejects all dangerous mint exts
check_no_transfer_fee(&data)?;             // or pick individually
if let Some(cfg) = read_transfer_fee_config(&data)? {
    let fee = calculate_transfer_fee(amount, &cfg.older_transfer_fee);
}
```

## Token / mint readers return owned `Address`

`token_account_owner`, `token_account_mint`, `token_account_delegate`,
`token_account_close_authority`, `mint_authority`, and `mint_freeze_authority`
return `Address` (or `Option<Address>`) by value — not `&Address`. A 32-byte
copy on BPF is a handful of loads, and the owned return removes the
unsoundness window where a returned reference outlived the `try_borrow` guard
and could alias a concurrent `&mut [u8]`.

---

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs) / Apache-2.0
