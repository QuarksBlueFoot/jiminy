# jiminy-vault

A minimal vault program demonstrating Jiminy's safety primitives on top of Pinocchio.

## Instructions

| Index | Name | Accounts | Data |
|-------|------|----------|------|
| `0` | InitVault | `[signer+writable] payer`, `[writable] vault`, `[] system_program` | `authority: [u8; 32]` |
| `1` | Deposit | `[signer+writable] depositor`, `[writable] vault` | `amount: u64` |
| `2` | Withdraw | `[signer] authority`, `[writable] vault`, `[writable] recipient` | `amount: u64` |
| `3` | CloseVault | `[signer] authority`, `[writable] vault`, `[writable] destination` | - |

## Account Layout (Jiminy Header v1)

```
Byte  Field         Type     Notes
─────────────────────────────────────────
0     discriminator u8       = 1
1     version       u8       = 1
2     flags         u8       reserved (0)
3     reserved      u8       must be 0
4-7   data_len      u32      0 (fixed-size)
8-15  balance       u64      lamports held
16-47 authority     Address  32-byte pubkey
─────────────────────────────────────────
Total: 48 bytes
```

## Patterns Demonstrated

- **`AccountList`** for iterator-style account consumption
- **`check_header`** / **`write_header`** for the Jiminy Layout v1 convention
- **`SliceCursor`** / **`DataWriter`** for zero-copy field access
- **`require_accounts_ne!`** to prevent source == destination attacks
- **`safe_close`** for atomic account closure
- **`checked_add`** / **`checked_sub`** for overflow-safe math
- CPI to the system program for account creation
