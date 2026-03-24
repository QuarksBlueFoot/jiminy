# Staking Template

Minimal staking pool using Jiminy's **segmented account** layout.

Demonstrates dynamic-length accounts: a fixed prefix plus a variable-length
array of stake entries, all zero-copy.

## Account Layout

```text
Offset  Bytes  Field
──────────────────────────────────────────
 0       1     discriminator  (3)
 1       1     version        (1)
 2       2     flags
 4       8     layout_id
12       4     reserved
────── AccountHeader (16 bytes) ──────
16      32     authority
48       8     total_staked
────── Fixed prefix (56 bytes) ───────
56       8     stakes descriptor  [offset(4) | count(2) | elem_size(2)]
────── Segment table (8 bytes) ──────
64       N     StakeEntry[] data  (48 bytes each)
```

### StakeEntry (48 bytes, `#[repr(C)]`)

| Offset | Size | Field       |
|--------|------|-------------|
| 0      | 32   | staker      |
| 32     | 8    | amount      |
| 40     | 8    | start_epoch |

## Instructions

| Tag | Name     | Accounts                                     | Data                        |
|-----|----------|----------------------------------------------|-----------------------------|
| 0   | InitPool | payer(ws), pool(w), system_program            | authority(32), max_stakers(2) |
| 1   | Stake    | staker(s), pool(w), clock                    | amount(8)                   |
| 2   | Unstake  | staker(s), pool(w)                           | index(2)                    |

## Patterns Demonstrated

- **`segmented_layout!`** - declares fixed prefix + dynamic segment table
- **`compute_account_size`** - rent-exact allocation at init time
- **`init_segments`** - writes initial segment descriptors
- **`SegmentDescriptor`** - read / update element count
- **Swap-remove** - O(1) element deletion in unordered segments
- **`safe_create_account`** - CPI with automatic rent calculation
- **`checked_add` / `checked_sub`** - safe arithmetic on totals
