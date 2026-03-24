# Jiminy v1.0 Roadmap

## Positioning

Jiminy is the safety + ABI layer for Solana programs built on pinocchio.

Not a framework. The layer frameworks should be built on.

---

## Phase 1: Lock API surface

- Freeze core API (check, account, math, abi)
- Freeze ABI contract rules (header, layout_id, versioning)
- Safety model documented as canonical reference
- docs.rs clean and authoritative

Done when developers default to "just use Jiminy."

---

## Phase 2: Ecosystem adoption

Targets: indexers, wallets, explorers, SDKs.

- Publish TypeScript decoder tooling (`@jiminy/ts`)
- Example indexer integration (Helius, Triton)
- Explorer decoding examples

Done when third-party tools decode Jiminy accounts natively.

---

## Phase 3: Framework layer (separate repo)

- CLI scaffolding (`create-jiminy-app`)
- Protocol templates (staking, lending, vault)
- Optional proc macros (allowed here, not in core)

Constraint: framework depends on Jiminy. Jiminy never depends on framework.

---

## Phase 4: Network effects

- Encourage other frameworks to adopt Jiminy ABI
- Promote `layout_id` as ecosystem-wide standard
- Push schema manifests into tooling pipelines

Done when `layout_id` is how Solana accounts identify themselves.

---

## Non-goals

- Full framework in core
- Proc macros in core
- Hiding the Solana execution model
- Abstractions that don't pay for themselves
