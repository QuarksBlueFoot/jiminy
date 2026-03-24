# {{project-name}}

A SOL vault program built with [Jiminy](https://github.com/QuarksBlueFoot/jiminy).

## Account Layout

```
Byte   Field         Type       Notes
───────────────────────────────────────────
0      discriminator u8         = 1
1      version       u8         = 1
2-3    flags         u16        reserved
4-11   layout_id     [u8; 8]    deterministic
12-15  reserved      [u8; 4]    must be 0
16-23  balance       u64        lamports held
24-55  authority     Address    32-byte pubkey
───────────────────────────────────────────
Total: 56 bytes
```

## Instructions

| Tag | Name | Accounts | Data |
|-----|------|----------|------|
| `0` | Init | `[s,w] payer`, `[w] vault`, `[] system` | `authority: [u8;32]` |
| `1` | Deposit | `[s,w] depositor`, `[w] vault` | `amount: u64` |
| `2` | Withdraw | `[s] authority`, `[w] vault`, `[w] recipient` | `amount: u64` |
| `3` | Close | `[s] authority`, `[w] vault`, `[w] destination` | - |

## Build

```bash
cargo build-sbf
```

## Patterns Used

- `zero_copy_layout!` - account definition with compile-time ABI
- `init_account!` - CPI create + header write
- `load_checked` / `load_checked_mut` - validated access
- `check_has_one` - authority verification
- `safe_close` - atomic closure
- `checked_add` / `checked_sub` - overflow-safe math
- `AccountList` - iterator account consumption
- `SliceCursor` - zero-copy instruction parsing
