# jiminy-multisig

M-of-N signer checks. Four functions, zero allocation.

```toml
jiminy-multisig = "0.11"
```

`count_signers` counts how many accounts in a slice signed.
`check_threshold` asserts at least M of them did.
`check_all_signers` and `check_any_signer` do what you'd expect.

```rust,ignore
use jiminy_multisig::*;

check_threshold(&signer_accounts, 3)?; // 3-of-N
```

`#![no_std]` · `no_alloc` · BPF-safe · Apache-2.0

[MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs)
