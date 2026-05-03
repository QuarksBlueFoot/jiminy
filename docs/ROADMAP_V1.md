# Jiminy v1.0 Roadmap

## Current status: standard-capable, not yet standard-declared

Jiminy is the most complete zero-copy ABI-oriented Solana library in its
lane. It is not yet ready to declare itself the frozen universal standard.

**Phase 3 status (2026-05-03):** adoption hardening is active. The core ABI is
solid enough for serious downstream use, but the pre-1.0 gate now prioritizes
small correctness and migration seams over new abstractions. This pass fixed
guard-macro ergonomics (`require*!` trailing commas), corrected
`require_keys_eq!` / `require_keys_neq!` to accept owned or borrowed `Address`
operands, restored compatibility with Hopper's `legacy-pinocchio-compat`
backend feature, and added `assert_legacy_layout!` for live programs that need
Jiminy layout safety without changing account header conventions. It also added
a template smoke gate that expands placeholder crates and checks vault, escrow,
and staking templates before they ship.

**What exists today (v0.15):**

- Fixed 16-byte account header with deterministic `layout_id`
- Safe-by-default tiered loading (5 tiers)
- Alignment-safe `Le*` wire types with field-level borrow splitting
- Cross-program interface generation (`jiminy_interface!`)
- Segmented ABI for variable-length accounts
- Schema + TypeScript runtime/tooling (`jiminy-schema`, `@jiminy/ts`)
- CI with Miri and SBF build coverage
- Explicit safety model with 10 documented invariants
- `solana-zero-copy` bidirectional bridge

**What Jiminy already beats:**

- Anchor zero-copy: framework-owned layouts, not a portable ABI contract
- `solana-zero-copy`: unaligned primitives, not a full ABI standard
- SPL `spl-pod` / `spl-list-view`: useful but not a cohesive standard
- Pina: framework-like, not ABI substrate

**The biggest differentiator:** runtime-verifiable ABI identity (`layout_id`).
This is what turns "zero-copy helpers" into "ABI layer."

---

## 1.0 go / no-go checklist

### Go when ALL of these are true

| # | Gate | Status |
|---|------|--------|
| G1 | Published crate version matches repo reality | done (v0.15.1) |
| G2 | Manifest/schema format frozen for tooling | in progress |
| G3 | Segmented ABI semantics frozen for auditors | in progress |
| G4 | Fuzz targets for dangerous parsing/overlay surfaces | partial (best-effort + ZeroCopySlice) |
| G5 | Trust model fully locked and boring | done |
| G6 | TS/codegen path real enough for client dependents | done (`@jiminy/ts`) |
| G7 | Anchor/interop story documented and benchmarked | partial (jiminy-anchor exists, needs benchmarks) |
| G8 | 1-2 serious downstream examples prove cross-program ABI | not started |

### No-go if ANY of these are still moving

- Segmented layout contract (capacity, growth, compaction rules)
- Manifest format
- Foreign interface semantics
- Load-tier naming / trust semantics
- Primitive wire type strategy
- Published package surface vs repo surface mismatch

---

## Pre-1.0 work (priority order)

### P1: Freeze segmented ABI contract

Lock these permanently:

- Capacity semantics (fixed vs growable)
- Realloc / growth rules
- Compaction / swap-remove behavior
- Manifest / codegen shape for segments
- Segment migration / version rules

This is the biggest technical frontier. Must feel boring and inevitable,
not clever and still moving.

### P2: Fuzz the dangerous surfaces

Add fuzz targets for:

- Header validation (malformed headers, truncated data)
- Segment table parsing (overlapping, out-of-bounds, zero-size)
- Malformed manifests
- Foreign interface loads (`jiminy_interface!` against bad data)
- Best-effort / compatibility loading paths

Miri in CI is table stakes. Fuzzing is what makes the overlay safety
story credible to auditors.

### P3: layout_id collision hardening (optional-harder)

Eight-byte IDs are fine in practice. Standards need a credible answer
before auditors ask. Document and optionally implement:

- Optional 16-byte layout IDs
- Manifest-level full SHA-256 hash verification
- Build-time registry with `const` collision assertions

### P4: Ecosystem bridge polish

Adoption path must be smoother than the rewrite path for:

- Anchor hot paths (drop-in overlay alongside Anchor accounts)
- Pinocchio-first codebases (already native)
- SPL/TLV-heavy programs (jiminy-layouts bridge)
- Client/indexer tooling (schema manifests, TS decoders)

### P5: Downstream proof

Ship 1-2 real cross-program examples where:

- Program A creates accounts with `zero_copy_layout!`
- Program B reads them with `jiminy_interface!`
- TypeScript client decodes both via `@jiminy/ts`

This is the proof that the ABI contract works end-to-end,
not just in unit tests.

---

## Post-1.0 phases

### Phase A: Ecosystem adoption

- Indexer integration examples (Helius, Triton)
- Explorer decoding examples
- Push schema manifests into tooling pipelines

Done when third-party tools decode Jiminy accounts natively.

### Phase B: Framework layer (separate repo)

- CLI scaffolding (`create-jiminy-app`)
- Protocol templates (staking, lending, vault)
- Optional proc macros (allowed here, not in core)

Constraint: framework depends on Jiminy. Jiminy never depends on framework.

### Phase C: Network effects

- Encourage other frameworks to adopt Jiminy ABI
- Promote `layout_id` as ecosystem-wide standard
- Push toward "layout_id is how Solana accounts identify themselves"

---

## Non-goals

- Full framework in core
- Proc macros in core
- Hiding the Solana execution model
- Abstractions that don't pay for themselves

## Risk

The danger is not "this won't work." The danger is:

- Shipping 1.0 too early (before segmented ABI and fuzzing are locked)
- Letting SPL/Anchor-adjacent tooling fill the same gap first
- Leaving adoption friction high enough that a less-complete but more
  "official" solution wins by gravity
