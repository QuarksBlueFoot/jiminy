# jiminy-escrow

A two-party escrow program demonstrating close checks and ordering guarantees with Jiminy.

## Instructions

| Index | Name | Accounts | Data |
|-------|------|----------|------|
| `0` | CreateEscrow | `[signer+writable] creator`, `[writable] escrow`, `[] system_program` | `amount: u64`, `recipient: Address`, `timeout_ts: i64` |
| `1` | AcceptEscrow | `[signer] recipient`, `[writable] escrow`, `[writable] destination` | — |
| `2` | CancelEscrow | `[signer] creator`, `[writable] escrow`, `[writable] destination`, `[optional] linked_account` | — |

## Account Layout (Jiminy Header v1)

```
Byte   Field         Type     Notes
──────────────────────────────────────────
0      discriminator u8       = 2
1      version       u8       = 1
2      flags         u8       bit 0 = accepted
3      reserved      u8       must be 0
4-7    data_len      u32      0 (fixed-size)
8-15   amount        u64      lamports locked
16-47  creator       Address  who created
48-79  recipient     Address  who can accept
80-87  timeout_ts    i64      unix ts (0 = none)
──────────────────────────────────────────
Total: 88 bytes
```

## Patterns Demonstrated

- **`check_header`** with version validation
- **`read_header_flags`** / **`read_bit`** / **`set_bit`** for flag-based state
- **`require_flag!`** pattern awareness (checking flag NOT set for cancellation)
- **`check_closed`** to verify a linked account was already closed
- **`check_has_one`** for stored address == account key validation
- **`require_accounts_ne!`** to prevent escrow == destination
- **`safe_close`** for atomic escrow closure
- Optional linked-account pattern for ordering guarantees
