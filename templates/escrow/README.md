# {{project-name}}

A time-locked escrow program built with [Jiminy](https://github.com/QuarksBlueFoot/jiminy).

## Account Layout

```
Byte   Field         Type       Notes
───────────────────────────────────────────
0      discriminator u8         = 2
1      version       u8         = 1
2-3    flags         u16        bit 0 = accepted
4-11   layout_id     [u8; 8]    deterministic
12-15  reserved      [u8; 4]    must be 0
16-23  amount        u64        escrowed lamports
24-55  creator       Address    who funded
56-87  recipient     Address    who can claim
88-95  deadline      i64        Unix timestamp
───────────────────────────────────────────
Total: 96 bytes
```

## Instructions

| Tag | Name | Accounts | Data |
|-----|------|----------|------|
| `0` | Create | `[s,w] creator`, `[w] escrow`, `[] system` | `recipient: [u8;32]`, `amount: u64`, `deadline: i64` |
| `1` | Accept | `[s] recipient`, `[w] escrow`, `[w] destination` | - |
| `2` | Cancel | `[s] creator`, `[w] escrow`, `[w] destination`, `[] clock` | - |

## Build

```bash
cargo build-sbf
```

## Patterns Used

- `zero_copy_layout!` - account definition
- Header **flags** for state tracking (`FLAG_ACCEPTED`)
- `read_header_flags` - read flag bits from header
- `check_after` - deadline enforcement via Clock sysvar
- `check_has_one` - multi-party authorization
- `safe_close` - atomic account closure
