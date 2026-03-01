# Jiminy Account Layout Convention (v1)

A strict, copy-pasteable convention for Solana account data layouts in
Pinocchio programs using Jiminy.

## The Header

Every Jiminy-managed account starts with an **8-byte fixed header**:

```
Byte   Field          Type    Description
──────────────────────────────────────────────────────
0      discriminator  u8      Account type tag (unique per program)
1      version        u8      Schema version - bump when layout changes
2      flags          u8      Application-defined bitfield (LSB-first)
3      reserved       u8      Must be zero (future use)
4–7    data_len       u32     Payload size (LE); 0 for fixed-size accounts
──────────────────────────────────────────────────────
```

Payload fields follow immediately after byte 7.

## Rules

1. **Discriminator is immutable.** Once an account type is deployed, its
   discriminator value never changes.

2. **Version must increment** whenever the payload layout changes. This
   includes adding fields, removing fields, or changing field sizes.

3. **Fields never move.** Once a field is assigned an offset within a
   version, that offset is permanent for that version. Append new fields
   at the end.

4. **Account size never decreases.** If you need fewer fields, leave the
   old bytes as reserved padding. Programs that encounter an older, larger
   account should still work.

5. **`data_len` is optional.** For fixed-size accounts, leave it at 0.
   For variable-length accounts (e.g., a registry entry with a name
   string), set it to the actual payload byte count.

## Using the Header in Code

### Writing (init)

```rust
use jiminy::prelude::*;

let mut raw = account.try_borrow_mut()?;
zero_init(&mut raw);
write_header(&mut raw, MY_DISC, MY_VERSION, 0)?;
let mut w = DataWriter::new(header_payload_mut(&mut raw));
w.write_u64(initial_balance)?;
w.write_address(&authority)?;
```

### Reading (validate + access)

```rust
use jiminy::prelude::*;

let data = account.try_borrow()?;
check_header(&data, MY_DISC, MIN_VERSION)?;

let payload = header_payload(&data);
let mut cur = SliceCursor::new(payload);
let balance   = cur.read_u64()?;
let authority = cur.read_address()?;
```

### Version migration

```rust
let data = account.try_borrow()?;
let version = read_version(&data)?;
match version {
    1 => { /* read v1 fields */ }
    2 => { /* read v1 fields + new v2 fields */ }
    _ => return Err(ProgramError::InvalidAccountData),
}
```

## Layout Lint (Test-Time)

Add this pattern to your program's test suite to catch accidental layout
drift:

```rust
#[cfg(test)]
mod layout_tests {
    use super::*;

    // If you change VAULT_LEN, you MUST bump VAULT_VERSION.
    const EXPECTED_LEN: usize = 48;
    const EXPECTED_VERSION: u8 = 1;

    #[test]
    fn vault_layout_stable() {
        assert_eq!(VAULT_LEN, EXPECTED_LEN, "VAULT_LEN changed - bump VAULT_VERSION");
        assert_eq!(VAULT_VERSION, EXPECTED_VERSION);
    }

    #[test]
    fn vault_offsets_valid() {
        // Header is 8 bytes.
        assert_eq!(jiminy::HEADER_LEN, 8);
        // balance at offset 0 in payload (byte 8 absolute).
        assert_eq!(BALANCE_OFFSET, 0);
        // authority at offset 8 in payload (byte 16 absolute).
        assert_eq!(AUTHORITY_OFFSET, 8);
        // Total: header(8) + balance(8) + authority(32) = 48.
        assert_eq!(jiminy::HEADER_LEN + 8 + 32, VAULT_LEN);
    }
}
```

This test will fail CI if someone changes the account size without
updating the version constant - catching layout drift before it reaches
mainnet.

## FAQ

**Q: Why not use Anchor's 8-byte discriminator?**
A: Anchor uses the first 8 bytes of `sha256("account:TypeName")` as a
discriminator. This is safe but costs 7 extra bytes per account. Since
Jiminy programs don't use Anchor's IDL or client generation, a single
`u8` type tag is sufficient (256 account types per program) and leaves
more room for payload.

**Q: What about backward compatibility?**
A: Programs should always check `version >= MIN_VERSION` using
`check_header`. If a program encounters a version it doesn't understand
(too new), it should return `InvalidAccountData`. If it encounters an
older version, it can either migrate in-place or read the old layout.

**Q: Do I have to use this header?**
A: No. Jiminy's check functions work with any layout. The header is a
*recommended convention* that gives you versioning, flags, and payload
length for free. If you're porting an existing program that uses a
different discriminator scheme, keep using it.
