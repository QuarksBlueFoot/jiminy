# jiminy-escrow

Two-party escrow with close checks and ordering guarantees, built on Jiminy.

## Instructions

| Index | Name | Accounts | Data |
|-------|------|----------|------|
| `0` | CreateEscrow | `[signer+writable] creator`, `[writable] escrow`, `[] system_program` | `amount: u64`, `recipient: Address`, `timeout_ts: i64` |
| `1` | AcceptEscrow | `[signer] recipient`, `[writable] escrow`, `[writable] destination` | - |
| `2` | CancelEscrow | `[signer] creator`, `[writable] escrow`, `[writable] destination`, `[optional] linked_account` | - |

## Account Layout (Jiminy Header v1)

```
Byte   Field         Type      Notes
──────────────────────────────────────────
0      discriminator u8        = 2
1      version       u8        = 1
2-3    flags         u16       bit 0 = accepted
4-11   layout_id     [u8; 8]   SHA-256 of canonical layout string
12-15  reserved      [u8; 4]   must be 0
16-23  amount        u64       lamports locked
24-55  creator       Address   who created
56-87  recipient     Address   who can accept
88-95  timeout_ts    i64       unix ts (0 = none)
──────────────────────────────────────────
Total: 96 bytes (16 header + 8 + 32 + 32 + 8)
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
