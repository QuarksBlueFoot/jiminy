# jiminy-anchor

Adapter for reading Anchor-framework accounts from Jiminy programs (and vice versa). No dependency on `anchor-lang`. Operates purely on raw byte layouts.

`#![no_std]` / `no_alloc` / BPF-safe

```toml
[dependencies]
jiminy-anchor = "0.16"
```

## What's in here

| | |
|---|---|
| `anchor_disc` | Compute the 8-byte Anchor discriminator at compile time |
| `check_anchor_disc` | Validate an Anchor discriminator on raw account data |
| `AnchorHeader` | Zero-copy overlay for the 8-byte Anchor discriminator |
| `anchor_body` / `anchor_body_mut` | Get the body slice (`[8..]`) from Anchor account data |
| `check_and_body` | Discriminator check + body slice in one call |
| `check_and_overlay` / `check_and_overlay_mut` | Disc check + `Pod` overlay on the body |
| `check_anchor_with_layout_id` | Verify both Anchor disc and Jiminy `layout_id` |

## Example

```rust,ignore
use jiminy_anchor::{anchor_disc, check_anchor_disc, anchor_body};
use jiminy_core::account::{pod_from_bytes, Pod, FixedLayout};

const VAULT_DISC: [u8; 8] = anchor_disc("Vault");

#[repr(C)]
#[derive(Clone, Copy)]
struct AnchorVaultBody {
    balance: [u8; 8],
    authority: [u8; 32],
}
unsafe impl Pod for AnchorVaultBody {}
impl FixedLayout for AnchorVaultBody { const SIZE: usize = 40; }

fn read_anchor_vault(data: &[u8]) -> Result<&AnchorVaultBody, jiminy_core::ProgramError> {
    check_anchor_disc(data, &VAULT_DISC)?;
    pod_from_bytes::<AnchorVaultBody>(&data[8..])
}
```

## About

Built by [MoonManQuark](https://x.com/moonmanquark) / [Bluefoot Labs](https://github.com/BluefootLabs).

Donations: `solanadevdao.sol` (`F42ZovBoRJZU4av5MiESVwJWnEx8ZQVFkc1RM29zMxNT`)

## License

Apache-2.0
