# Jiminy Repo Memory -- Ecosystem Intelligence

## Competitor Frameworks (researched 2026-03-27)

### Quasar (blueshift-gg/quasar) -- v0.0.0, Beta
- `no_std`, zero-alloc, Anchor-familiar API
- Inline dynamic fields: `String<P,N>`, `Vec<T,P,N>` with offset caching
- BUMP_OFFSET: reads bump from account data → `verify_program_address` (~200 CU vs ~544 CU)
- Batched u32 header validation (dup+signer+writable in one compare)
- Direct `sol_invoke_signed_c` CPI (bypasses invoke_signed_unchecked)
- `RawEncoded` for zero-copy CPI pass-through
- SBF profiler with flamegraph output
- NO layout_id, NO cross-program ABI, NO account versioning
- Deps: `solana-account-view 2.0`, `solana-address 2.2`

### Star Frame (staratlasmeta/star_frame) -- v0.30.0, Production
- Built by Star Atlas, production-proven
- `UnsizedType` system: List, Map, Set, UnsizedString, UnsizedMap -- with runtime resize
- Miri-validated pointer safety (Tree Borrows)
- 4-phase instruction lifecycle: decode → validate → process → cleanup
- Codama IDL integration with structural verifier (SFIDL001-011)
- ~60-93% CU reduction vs Anchor (their benchmarks)
- NOT `no_std` -- uses Box, Vec, BTreeMap
- Pinocchio 0.9.2 (not updated to 0.10+)
- Borsh for instruction data (serialization overhead)
- Deps: `pinocchio 0.9.2`, `bytemuck`, `borsh`, `ptr_meta`

### Other Frameworks
- **Typhoon 0.2.2**: Minimal pinocchio wrapper, size-adaptive discriminant checks (SF credits them)
- **Pina 0.6.0**: Pinocchio + Codama renderer for bytemuck models
- **Shank 0.4.8**: IDL extraction tool (Metaplex), not a framework
- **Codama 0.8.0**: Universal IDL → multi-lang client generator
- **Bolt 0.2.4**: ECS on Anchor (MagicBlock gaming), not zero-copy

## Jiminy Unique Advantages (confirmed)
1. Deterministic layout_id (SHA-256) -- NO competitor has this
2. Cross-program interfaces with 5-tier trust -- completely unmatched
3. Declarative-only macros -- best auditability
4. Segment capacity tracking -- unique explicit capacity
5. `no_std` + `no_alloc` purity

## Potential Jiminy Improvements from Research
- BUMP_OFFSET PDA optimization (~344 CU saving) -- from Quasar
- Inline dynamic layout for 1-2 fields -- from Quasar pattern
- Context caching (rent, clock) -- from Star Frame
- Generic ZeroCopySlice prefix (u8/u16/u32) -- from spl-list-view
- Binary search on sorted slices -- from Star Frame
- Codama IDL output option -- ecosystem alignment
