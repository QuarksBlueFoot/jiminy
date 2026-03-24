# Layout ID Collision Analysis

> **Status:** Locked - v0.15.0

## Summary

Jiminy uses an 8-byte (64-bit) `layout_id` truncated from a SHA-256
hash of the canonical schema string. This document provides the formal
collision probability analysis and explains why 8 bytes is sufficient
for all practical Solana use cases.

## How layout_id Is Computed

```
SHA-256("jiminy:v1:<Name>:<version>:<field_name>:<canonical_type>:<size>,...")
```

The first 8 bytes of the resulting 32-byte hash become the `layout_id`.
This is stored in every account header and verified on every `load()`
and `load_foreign()` call.

## Birthday Paradox Analysis

The birthday paradox gives the probability that any two distinct layouts
collide. For an $n$-bit hash truncated to $k$ bits, the probability of
at least one collision among $m$ distinct inputs is approximately:

$$P(\text{collision}) \approx 1 - e^{-m^2 / 2^{k+1}}$$

For $k = 64$ (8-byte layout_id):

| Distinct layouts ($m$) | Collision probability |
|------------------------|----------------------|
| 1,000 | $2.7 \times 10^{-14}$ |
| 10,000 | $2.7 \times 10^{-12}$ |
| 100,000 | $2.7 \times 10^{-10}$ |
| 1,000,000 | $2.7 \times 10^{-8}$ |
| 1,000,000,000 | $0.027$ (2.7%) |
| $2^{32}$ ($\approx$ 4.3 billion) | $\approx 0.39$ (39%) |

The 50% collision threshold (birthday bound) is at
$\approx 2^{32} = 4.3$ billion distinct layouts.

## Why 8 Bytes Is Sufficient

**The Solana ecosystem will never produce 4.3 billion distinct account
layouts.** For context:

- The entire Solana mainnet program count is in the low tens of
  thousands.
- Each program typically defines 2–20 account types.
- Even with 100,000 programs × 20 types = 2,000,000 layouts, the
  collision probability is $2.7 \times 10^{-8}$ - roughly one in
  37 million.

The collision probability at realistic scale is negligible - orders of
magnitude less likely than hardware bit-flip errors.

## What a Collision Would Mean

If two layouts did collide:

1. **Same program:** Impossible in practice. The struct name and
   version are part of the hash input. Two types within one program
   collide only if both their names, versions, AND field lists happen
   to produce the same 8-byte truncation.
2. **Cross-program (`load_foreign`):** A collision would mean
   Program B's struct overlays onto Program A's account without
   detecting the mismatch. However, `load_foreign` also requires an
   **owner check**: the account must be owned by the expected program.
   A layout_id collision only matters if you're loading accounts from
   a program whose layouts you don't control and happen to collide
   with. Even then, the field sizes and types must structurally match
   for the overlay to produce meaningful values.

## Higher-Assurance Escape Hatches

For programs that require defense-in-depth beyond the 64-bit
`layout_id`, Jiminy provides two mechanisms:

### 1. Off-Chain Full Hash Verification (LayoutManifest)

`LayoutManifest::hash_input()` reconstructs the full canonical string.
Tooling, indexers, and CI pipelines can compute the full 32-byte
SHA-256 and compare it against a known-good registry. This provides
256-bit collision resistance for the deployment/verification pipeline
while keeping on-chain validation at 8 bytes (minimal compute cost).

`LayoutManifest::verify_hash()` performs this check: it recomputes the
SHA-256 of the canonical hash input and verifies that the first 8 bytes
match the stored `layout_id`. If the full hash is needed, the caller
retains all 32 bytes from `hash_input()`.

### 2. Owner Check as Second Factor

Every `load()` and `load_foreign()` call verifies that the account is
owned by the expected program. This is an independent check orthogonal
to `layout_id`. For a false-positive load, an attacker would need:

1. A layout_id collision ($< 10^{-8}$ for realistic layout counts), **AND**
2. The account to be owned by the expected program (requires program
   compromise or same-program collision which is negligible).

The combined probability is vanishingly small.

## Header Format Constraint

The Jiminy account header is 16 bytes:

```
Byte   Field          Size
──────────────────────────────
0      discriminator   1
1      version         1
2-3    flags           2
4-11   layout_id       8
12-15  reserved        4
──────────────────────────────
```

The reserved field (4 bytes) cannot accommodate a 16-byte extended
layout_id without breaking the header format. Extending to 16 bytes
would require a header format bump (a breaking change). This is
unnecessary given the analysis above. 8 bytes provides ample
collision resistance for any practical deployment, and the off-chain
full-hash verification covers high-assurance scenarios.

## Recommendation

- **On-chain:** 8-byte `layout_id` is sufficient. No format change needed.
- **Off-chain/CI:** Use `LayoutManifest::hash_input()` + full SHA-256
  for pre-deployment verification in security-critical pipelines.
- **Monitoring:** Programs with hundreds of layouts can maintain a
  build-time registry of `(layout_id, hash_input)` pairs and assert
  no duplicates.

## References

- Birthday paradox: Menezes, Oorschot, Vanstone, *Handbook of Applied
  Cryptography*, §9.7.1
- SHA-256 truncation analysis: NIST SP 800-107 Rev. 1, §5
