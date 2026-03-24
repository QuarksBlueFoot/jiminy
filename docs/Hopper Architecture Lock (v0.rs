Hopper Architecture Lock (v0.1)
Positioning
Hopper
Typed zero-copy state framework for Solana

Built on Jiminy, the zero-copy standard library.

Product split
jiminy → low-level zero-copy standard library
hopper → framework/runtime layer
hopper-macros → optional ergonomics
hopper-schema → schema, layout fingerprints, export/codegen
hopper-cli → state-aware tooling

That split is not negotiable. It’s one of your biggest strategic advantages.

The Core Thesis
Hopper’s real category:

A framework for modeling Solana account memory systems

Not:

“another smart contract framework”
“Anchor alternative”
“macro crate”

This is how you take the crown:

own memory layout
own state access
own state evolution
own state tooling

That’s bigger than “nice instruction syntax.”

What Hopper should steal and improve
From Star Frame
Keep:
trait-based architecture
typed instruction/account set ergonomics
compile-time leaning APIs
performance-first mentality
Improve:
less framework lock-in
better state tooling
stronger account evolution model
clearer memory architecture
more introspection/debuggability
From Steel
Keep:
explicit parsing / validation style
chainable checks
clean workspace split between API and program
pragmatic “less magic” ergonomics
Improve:
much deeper state modeling
much stronger schema system
much better zero-copy collections
much more serious layout evolution story
From Jiminy
Keep:
no_std / no_alloc / BPF-safe philosophy
account readers / writers / field refs / cursor model
strong guard / check vocabulary
transaction / CPI / balance / upgrade / sysvar safety
macro_rules! over proc-macro dependency whenever possible
Improve:
orchestration
framework ergonomics
state model abstractions
runtime phases
tooling + schema surface
The 10 Must-Have Hopper Features

These are required before Hopper is “real.”

1) Segmented Accounts
This is Hopper’s flagship feature.
Core concept

One account can contain multiple typed regions:

[Header][SegmentTable][CoreState][RiskState][Extensions][Journal]
Required APIs
let core = account.segment::<CoreState>()?;
let risk = account.segment::<RiskState>()?;
let journal = account.segment::<EventJournal>()?;
Required metadata per segment
kind
offset
len
version
flags
layout hash
Why it matters

This is how Hopper stops being “zero-copy structs” and becomes:

a framework for real protocol state

This is your crown feature.

2) Layout Fingerprints

Every layout should produce a deterministic fingerprint.

Required
pub trait LayoutFingerprint {
    const LAYOUT_HASH: [u8; 32];
}
Required APIs
frame.assert_layout::<Vault>()?;
segment.assert_layout::<RiskState>()?;
Use cases
migration checks
CI break detection
runtime compatibility checks
client compatibility
schema export
CLI diffing
Why it matters

This kills the nastiest zero-copy failure mode:

“the bytes still line up, but the meaning changed.”

This should be native, not bolted on.

3) Borrowed-State Execution Context
Core runtime type
pub struct Frame<'info> { ... }
Responsibilities
resolve accounts
enforce borrow discipline
manage mutable / immutable views
track phase state
optionally track dirty writes in debug/test mode
Required APIs
let mut frame = Frame::new(accounts, ix_data)?;
let vault = frame.account_mut::<Vault>(VAULT)?;
let user = frame.account::<User>(USER)?;
Why it matters

This is how Hopper becomes:

a state runtime
instead of just a pile of helpers.

4) Phased Execution
Required phases
Resolve
Validate
Borrow
Mutate
Emit
Commit
Required API
frame
    .resolve::<SwapAccounts>()?
    .validate()?
    .borrow_mut()?
    .run(|ctx| {
        // typed state access here
    })?;
Why it matters

This is one of the cleanest “fusion improvements” from the current field.

It gives:

clearer sequencing
easier audits
better tests
better fuzzing
more explainable behavior

This should become part of Hopper’s identity.

5) Validation Graph

Not just checks.
Not just account constraints.

Required support
account-local validation
cross-account validation
state transition validation
tx/CPI composition validation
Required shape
#[validate]
fn validate(ctx: &SwapCtx) -> Result<()> {
    ctx.pool.owner(program_id)?;
    ctx.user.signer()?;
    ctx.pool.field_eq(|p| p.mint, ctx.mint.key())?;
    ctx.state.transition(State::Open, State::Filled)?;
}
Why it matters

Jiminy already has a great low-level check vocabulary. Hopper should turn it into a real validation engine.

This is how you become better than “just account parsing.”

6) Zero-Copy Collections
Hopper must ship with:
FixedVec<T, N>
RingBuffer<T, N>
Slab<T, N>
SparseSet<T, N>
PackedMap<K, V, N>
SegmentedJournal<T>
BitSet<N>
Requirements
no alloc
deterministic
BPF-safe
borrow-safe
index-safe
Why it matters

This is one of the most practical things serious builders actually need.

Everyone eventually writes their own cursed slab or append-only registry. Hopper should stop that nonsense.

7) Versioned State + Compatibility
Required APIs
frame.migrate::<VaultV1, VaultV2>()?;
frame.compat::<VaultV1, VaultV2>()?;
Required metadata
kind
version
flags
layout hash
optional segment table
Why it matters

This is where most zero-copy systems eventually eat drywall.

Hopper should be unusually good at evolution:

append-safe
segment-safe
explicit migration-safe

This is a serious differentiator.

8) Foreign Account Interfaces
Required API
#[foreign_account(owner = spl_token::ID)]
pub struct MintView { ... }
Must support
owner checks
optional version checks
optional layout fingerprint checks
safe read-only typed access
Why it matters

This makes:

CPI integrations
multi-program composition
external state reads

much cleaner and safer.

Jiminy already hints in this direction. Hopper should own it fully.

9) Instruction Capability System
Required API
#[instruction(capabilities = [
    MutatesVault,
    RequiresSigner,
    TouchesTokenVault,
    EmitsReceipt,
])]
Used for
docs
audits
simulation
fuzzing
blast-radius analysis
Why it matters

This turns instructions into:

machine-readable protocol behavior

That is very strong and still underexplored in the ecosystem.

10) State-Aware CLI Tooling
Hopper CLI v1 should include:
hopper inspect
hopper diff
hopper decode
hopper schema
hopper migration-check
Killer command
hopper inspect <account_pubkey>
Output
layout
version
layout hash
segments
collections
decoded fields
Why it matters

Advanced zero-copy without state-aware tooling becomes a debugging war crime.

This is a major adoption moat.

The Out-of-the-Box Innovations

Now the part that makes Hopper not just “good,” but hard to replace.

Innovation 1 — Account Virtualization

This is one of the strongest things you can do.

Concept

A logical typed view can span:

multiple segments
or multiple accounts
Example
let market = frame.virtual_view::<MarketState>()?;

Internally it may stitch together:

header from one account
order slab from another
journal from another
Why it matters

This gives you a clean way to model:

sharded registries
large order books
hybrid ledgers
queue / archive systems
protocol state that outgrows one account

This is a real crown feature.

Innovation 2 — Deterministic State Diff Engine
API
let diff = frame.diff(before, after);
Output
changed fields
changed segments
changed collections
changed balances
invariant impact
Why it matters

Perfect for:

tests
audits
fuzzing
debugging
governance visibility

This is one of those features that makes the whole framework feel more “serious.”

Innovation 3 — State Snapshots / Replay
API
let snap = frame.snapshot();
Uses
debugging
fuzzing
replay
deterministic testing
local simulation
Why it matters

This makes Hopper feel like a systems framework, not just a crate stack.

Very strong if combined with inspect/diff.

Innovation 4 — Invariant Engine
API
#[invariant]
fn vault_consistent(vault: &Vault) -> bool {
    vault.total >= vault.withdrawn
}
Run modes
post-execution
post-CPI
tests
fuzzing
simulation
Why it matters

This pushes Hopper into:

protocol-grade correctness engineering

That’s a serious lane.

Innovation 5 — Self-Describing Accounts
Concept

Hopper accounts can optionally embed:

kind
version
layout hash
segment table
schema fingerprint
Why it matters

This makes tooling dramatically better:

automatic decode
explorer support
debugging
migration tooling
compatibility checks

This is very practical and very strong.

Innovation 6 — Execution Sandbox / Dry Run
API
frame.simulate(|ctx| {
    // no commit
});
Uses
dry-run logic
risk previews
composability safety checks
complex transition previewing
Why it matters

This gives Hopper a smarter runtime feel than most frameworks.

Innovation 7 — Memory Safety Debug Mode
In debug/test mode, detect:
double mutable borrows
invalid overlay access
invalid segment reads
unvalidated access
mutation before proper phase
Why it matters

This is one of those features that will quietly make builders love Hopper.

It adds confidence without bloating release runtime.

Innovation 8 — Capability-Based Account Permissions
API
#[account(capabilities = [CanWithdraw, CanSetFees])]
Why it matters

This enables richer authority models:

role-based state access
protocol permissions
delegated capabilities

Especially strong for:

DAOs
treasury systems
operator roles
governance systems
Innovation 9 — Segmented Realloc Protocol
Concept

Instead of raw “realloc and pray,” Hopper should support:

append-safe growth
segment-safe growth
version-safe expansion
Why it matters

This is one of the ugliest pain points in advanced Solana state.

If Hopper makes this civilized, that is a big win.

Innovation 10 — Proof / Compression Hooks

Not full ZK or compression in v0.1 — don’t be insane — but Hopper should be designed to support:

compressed account surfaces
Merkle-backed substate
proof-verified overlays
external witness-backed reads
Why it matters

This future-proofs Hopper for:

privacy
compression
hybrid execution
provable off-chain substate

That’s a long-term moat.

Exact Crate / Module Layout
jiminy

Leave it alone structurally except for low-level primitives Hopper truly needs.

hopper

Recommended module map:

hopper/
  src/
    lib.rs

    frame/
      mod.rs
      context.rs
      phase.rs
      borrow.rs
      snapshot.rs
      diff.rs
      simulate.rs

    account/
      mod.rs
      header.rs
      typed.rs
      overlay.rs
      segment.rs
      arena.rs
      compat.rs
      virtual.rs

    validate/
      mod.rs
      account.rs
      cross.rs
      transition.rs
      compose.rs
      graph.rs

    state/
      mod.rs
      version.rs
      layout.rs
      fingerprint.rs
      metadata.rs
      migrate.rs

    collections/
      mod.rs
      fixed_vec.rs
      ring_buffer.rs
      slab.rs
      sparse_set.rs
      packed_map.rs
      journal.rs
      bitset.rs

    instruction/
      mod.rs
      dispatch.rs
      capability.rs
      args.rs
      account_set.rs

    foreign/
      mod.rs
      interface.rs
      token.rs
      system.rs
      metadata.rs

    event/
      mod.rs
      emit.rs
      schema.rs

    error.rs
    prelude.rs

This is clean, scalable, and still zip-first sane.

hopper-macros
hopper-macros/
  src/
    lib.rs
    account_set.rs
    foreign_account.rs
    instruction.rs
    invariant.rs
    schema.rs

Only for optional derives / sugar.

hopper-schema
hopper-schema/
  src/
    lib.rs
    account.rs
    instruction.rs
    event.rs
    layout.rs
    fingerprint.rs
    export.rs

This should stay usable without forcing Hopper runtime usage.

hopper-cli
hopper-cli/
  src/
    main.rs
    commands/
      inspect.rs
      diff.rs
      decode.rs
      schema.rs
      migration_check.rs

This is where adoption magic happens.

Core Traits (the ones that matter)

These are the first real trait surfaces I would lock.

Layout / State Traits
pub trait LayoutFingerprint {
    const LAYOUT_HASH: [u8; 32];
}

pub trait VersionedState {
    const KIND: u16;
    const VERSION: u16;
}

pub trait SegmentKind {
    const SEGMENT_KIND: u16;
}

pub trait ForeignAccount {
    const OWNER: Pubkey;
}
Runtime Traits
pub trait ResolveAccounts<'info>: Sized {
    fn resolve(frame: &mut Frame<'info>) -> Result<Self>;
}

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

pub trait BorrowState<'info> {
    type Borrowed;
    fn borrow(self, frame: &mut Frame<'info>) -> Result<Self::Borrowed>;
}
Schema Traits
pub trait SchemaExport {
    fn export_schema() -> Schema;
}

pub trait CapabilitySet {
    fn capabilities() -> &'static [Capability];
}
The First 5 Modules to Build

This is the real build order.
Do not freestyle this.

1) frame/context.rs

Why first:
Because everything else depends on the execution model.

Build:

Frame<'info>
account resolution
mutable / immutable borrow tracking
phase tracking
2) account/header.rs + account/segment.rs

Why second:
Because this is Hopper’s crown feature.

Build:

versioned header
segment table
typed segment access
layout fingerprint hooks
3) validate/account.rs + validate/cross.rs

Why third:
Because Hopper’s real advantage is not just state access — it’s safe state access.

Build:

account-local validation
cross-account validation
Jiminy-backed checks integration
4) collections/

Why fourth:
Because this is immediate real-world value.

Build first:

FixedVec
RingBuffer
Slab
BitSet

Ship those before getting cute.

5) hopper-cli inspect

Why fifth:
Because if you can’t inspect your own cursed bytes, you’re not building a serious zero-copy framework.

This is the tooling proof that Hopper is real.

The Killer Example App (build this first)

Do not prove Hopper with a counter.

That would be cowardly.

Build:
Segmented Treasury + Journal
One Hopper account family with:
TreasuryCore
PermissionSegment
BudgetRulesSegment
ActionJournalSegment
VersionedHeader
Demonstrates:
segmented accounts
validation graph
layout fingerprints
zero-copy collections
schema export
CLI inspection
migration path

That is the exact kind of example that makes people understand why Hopper exists.

Competitor Parity Matrix (the honest version)
Capability	Anchor	Steel	Star Frame	Jiminy	Hopper Target
Fixed zero-copy structs	Yes	Yes	Yes	Yes	Yes
Typed account groups	Yes	Partial	Yes	No	Yes
Validation chains	Partial	Yes	Partial	Primitive-level	Yes
Segmented accounts	No	No	Partial-ish direction	No	Yes
Layout fingerprints	No	No	No public core feature	No	Yes
Borrowed-state phases	No	No	Partial-ish lifecycle	No	Yes
Zero-copy collections	Limited	Limited	Some direction	Primitive pieces	Yes
Version compatibility layer	Weak	Weak	Weak	No	Yes
Foreign account interfaces	Partial	Manual	Partial	Partial-ish	Yes
State-aware CLI tooling	Weak	Partial	Partial	No	Yes
Account virtualization	No	No	No public core feature	No	Yes

That’s how you know Hopper is not just “more of the same.”

Final blunt verdict
The right build philosophy is:
Jiminy stays sacred
Hopper owns orchestration + architecture
Hopper wins by owning memory layout, state access, state evolution, and state tooling
Everything else is secondary

If you stay disciplined, Hopper has a very real shot at becoming:

the framework serious Solana builders graduate into and never want to leave