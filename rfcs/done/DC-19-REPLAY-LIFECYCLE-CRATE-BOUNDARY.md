# RFC (done) - DC-19 Replay/Lifecycle Crate Boundary and Extraction Plan

**Status.** Implemented in 0.12.0.
**Released.** 2026-07-05.
**Tracks.** Behavior-neutral crate-boundary design after DC-18, before broad M2+ production
merge/conflict surfaces.
**Touches.** Workspace crate graph, `prikk-store` ownership boundaries, replay/lifecycle semantics,
object evidence boundaries, text-span ownership, and future patch-algebra placement.
**Companion handoff.** `../handoffs/DC-19-replay-lifecycle-crate-boundary/design-handoff.md`.

## Context

Through DC-18, `prikk-store` has acted as Prikk's repository integration crate. That was useful while
the project established durable layout, objects, refs/WAL, active sessions, signing/trust, replay,
worktree authoring, rollback, verification/doctor, text-span support, and internal patch algebra.

After DC-18, the crate now contains multiple bounded contexts:

- durable repository IO and local session state;
- replay-derived lifecycle state and state-transition semantics;
- object/blob evidence resolution;
- text-span identity/localization/splice primitives;
- worktree authoring/materialization behavior;
- rollback/inverse planning behavior;
- internal patch-algebra classification, commutation, and flat confluence.

The accepted crate-boundary direction review concluded:

1. Do not block DC-18 / v0.11.0 on a crate split.
2. Do not leave `prikk-store` broad indefinitely.
3. Design the first split before adding a broader public M2+ merge/conflict surface.
4. Prefer a replay/lifecycle domain crate before either a standalone patch-algebra crate or a worktree
   crate.
5. Keep `patch_algebra` internal until a production caller clarifies its stable API.

DC-19 defines the design and migration plan for that first split. It is intentionally design-first and
behavior-neutral.

## Design Goals

1. Define the first crate boundary below `prikk-store` for replay/lifecycle semantics.
2. Choose a crate name and ownership model that does not imply durable repository IO, ref traversal,
   active-WAL, or filesystem ownership.
3. Define dependency direction so the new semantic crate does not depend on `prikk-store`.
4. Decide which modules are first-wave extraction candidates and which must stay in `prikk-store`.
5. Define object evidence and blob validation boundaries before moving code.
6. Preserve current behavior, object identity, replay semantics, diagnostics, and release claims.
7. Keep patch algebra internal until a production caller proves its public shape.
8. Provide a staged migration plan that is reviewable and reversible.

## Non-goals

DC-19 does not add:

- merge UX, merge execution, or branch merge commands;
- public confluence APIs or public conflict-witness formats;
- persisted proof, conflict-witness, or merge-evidence objects;
- object schema changes;
- rollback refs or rollback authorization;
- branch switching, branch copy/fork, tags, remotes, key lifecycle, audit/plugin, or sync behavior;
- worktree crate extraction;
- a standalone `prikk-patch-algebra` crate;
- behavior changes hidden inside file movement.

## Proposed Design

### Crate Name

The first new crate should be named **`prikk-replay`**.

Rationale:

- The intended boundary is broader than node lifecycle storage, but narrower than repository storage.
- The crate owns semantic replay state and operation application semantics.
- `prikk-lifecycle` would over-focus the name on one data structure and understate replay/text
  responsibilities.
- `prikk-replay` can later own lifecycle state, text-span primitives, replay-local evidence types, and
  possibly patch-algebra semantics if a production caller proves that shape.

`prikk-replay` is a semantic replay domain crate. It must not imply ownership of repository lineage
resolution, ref traversal, block graph lookup, active WAL state, repository layout, or filesystem
storage. Those responsibilities remain in `prikk-store`.

### Target Dependency Graph

The target graph after the first extraction is:

```text
prikk-hash
prikk-error
prikk-object
prikk-crypto

prikk-replay
  depends on: prikk-error, prikk-hash, prikk-object
  owns: lifecycle state, replay-derived semantic state, replay-local state equality,
        path/preimage facts, text-span primitives, and replay evidence traits

prikk-store
  depends on: prikk-replay, prikk-object, prikk-crypto, prikk-error, prikk-hash, getrandom
  owns: repository layout, object store, refs, WAL, active session, locks, publication,
        trust persistence, verify/doctor IO, and store-backed evidence/resolver construction

prikk
  depends on: prikk-store
```

`prikk-replay` must not depend on `prikk-store`. If it does, the split is mostly nominal and should be
rejected.

Each implementation phase must include dependency evidence, such as:

```text
cargo tree -p prikk-replay
```

or equivalent output proving that `prikk-replay` does not depend on `prikk-store`.

### Workspace-Internal API Policy

`prikk-replay` is workspace-internal during DC-19. New `pub` APIs needed for `prikk-store` integration
do not imply external API stability. No external stability promise is made until a later API-design RFC
explicitly creates one.

If the workspace publishes crates independently, the initial `prikk-replay` crate should either be
marked `publish = false` while the boundary stabilizes, or documented as an internal/experimental
`0.x` support crate in crate docs and release notes.

### Responsibilities Moved First

The smallest safe first extraction should target lifecycle semantics that can be moved without
changing repository behavior:

- `node_lifecycle`;
- lifecycle structural equality and internal consistency checks;
- lifecycle state mutation primitives that are direct helpers of supported lifecycle application.

The first implementation slice should add the `prikk-replay` crate skeleton, move `node_lifecycle` and
direct tests, move only direct lifecycle equality/consistency helpers, and update `prikk-store` callers
through the new crate.

Path occupancy facts, preimage/postimage facts, and small replay/evidence traits are later extraction
candidates after the lifecycle-only boundary is stable.

`text_span` is a later candidate, not a first-slice requirement. It should move only after the
lifecycle-only boundary proves the dependency graph, unless review finds that the lifecycle-only move
is too small to be useful.

The key is that moved code must not need `.prikk/` layout, refs, active WAL state, trust policy, cache
persistence, lineage traversal, or filesystem repair behavior.

### Responsibilities Staying in `prikk-store`

The following remain in `prikk-store` for the first split:

- repository layout and filesystem paths;
- object store and file-backed persistence;
- refs, ref logs, ref publication, ref recovery;
- WAL, active-session lock, active-WAL metadata;
- seal publication and crash/retry behavior;
- trust policy persistence and signer/trust-store IO;
- verify and doctor filesystem/repository diagnostics;
- migration and repair behavior;
- store-backed resolver construction that binds a sealed baseline to durable object evidence;
- ref, block lineage, and replay horizon resolution;
- replay-derived cache persistence, cache invalidation, and cache trust rules;
- CLI-facing integration surfaces through `prikk-store`.

Some of these may be split later. They are not part of DC-19's first-wave boundary.

### `patch_algebra` Placement

`patch_algebra` should **not** become a standalone crate in DC-19.

For the first boundary, keep patch algebra private/internal where it is, or move only the parts that are
plain replay/lifecycle facts if they naturally follow extracted modules. Do not create a public-ish
patch-algebra API until at least one production caller exists, such as:

- a production read-only analysis caller;
- public conflict/merge evidence;
- a merge-planning API with stable failure semantics;
- a public confluence/check command.

If later extraction is justified, likely options are:

1. keep patch algebra inside `prikk-replay` as a replay-semantic domain;
2. create `prikk-patch-algebra` after API shape is proven;
3. keep it inside `prikk-store` until merge/conflict evidence is designed.

DC-19 should record that choice as deferred.

### Worktree Extraction

Do not extract worktree behavior in DC-19.

Worktree authoring/status/materialization currently depends on repository layout, active-session
policy, path validation, replay results, and CLI expectations. A first worktree split would likely
create a crate that still depends heavily on `prikk-store`, which does not solve the dependency
direction problem.

Revisit a possible `prikk-worktree` crate only after `prikk-replay` is stable.

## Boundary Contracts

### Object Evidence Reader

The semantic crate may need object evidence, but it must not know about file-backed storage or ref
layout. Define a small trait before extraction. A candidate shape is:

```text
trait ObjectEvidenceReader {
    fn read_object(id: ObjectId) -> Result<Option<ObjectEnvelope>, EvidenceReadError>;
}
```

The exact Rust type names are not fixed by this design. The required properties are:

- `prikk-store` implements or adapts the trait for durable object storage and owns object lookup;
- `prikk-replay` consumes object evidence abstractly;
- object type, blob kind, malformed payload, unreadable, and missing distinctions remain explicit;
- store-backed resolver construction stays in `prikk-store`;
- evidence errors stay distinct from ordinary unsupported algebra or replay conflicts.

### Blob Validation Responsibility

Blob validation must be assigned explicitly:

- durable object read errors originate in `prikk-store`;
- canonical object decode and canonical shape helpers remain owned by `prikk-object`;
- replay-local validation decides whether a blob kind/content is acceptable for a replay semantic fact;
- missing/wrong-type/malformed/unreadable evidence must not silently become default-empty content or a
  generic unknown result.

This boundary must preserve DC-17/DC-18 evidence semantics.

### Replay Input Contract

A replay/lifecycle extraction must define:

- baseline identity and horizon metadata supplied by `prikk-store`;
- ordered patch sequence input;
- object evidence input;
- failure categories for missing objects, malformed payloads, inconsistent lifecycle transitions, and
  unsupported operations;
- whether replay returns owned state, borrowed views, or certified compared state.

`prikk-store` resolves refs, block lineage, and replay horizons. `prikk-replay` consumes a validated
ordered patch stream plus baseline identity/horizon metadata. `prikk-replay` may carry `ObjectId`
labels for diagnostics and evidence binding, but it must not read refs, blocks, WALs, repository layout,
or filesystem paths.

The extraction must not change replay order, object identity, patch identity, or existing fail-closed
behavior.

### Lifecycle State Ownership

`prikk-replay` should own lifecycle state data structures and validation, including:

- live node and tombstone state;
- path occupancy indexes;
- seen-id validation;
- structural equality used by replay and patch algebra;
- mutation primitives for supported operation application.

`prikk-store` may persist or cache replay-derived state, but it should not own semantic lifecycle rules
after extraction.

Replay-derived cache encoding remains in `prikk-store` initially. `prikk-store` owns persistence format,
cache invalidation, cache trust rules, and store-backed resolver construction. A later DC may revisit
cache placement after the semantic state type stabilizes.

### Text-Span Ownership

`text_span` should be a candidate for `prikk-replay` because it defines replay/authoring semantics for
text operation identity, localization, and splice behavior. Before moving it, DC-19 must confirm:

- object identity helpers still use `prikk-object` / `prikk-hash` appropriately;
- worktree authoring callers can depend through `prikk-store` or a later boundary without cycles;
- existing DC-12/DC-14 vectors remain unchanged.

## Migration Plan

### Phase 1 - Crate Skeleton and Dependency Proof

- Add the `prikk-replay` crate manifest and module skeleton.
- Move no behavior yet, or move only a tiny leaf type if needed to prove dependency direction.
- Run `cargo tree -p prikk-replay`, or equivalent, to prove `prikk-replay` does not depend on
  `prikk-store`.
- Preserve CLI/output behavior.

### Phase 2 - Lifecycle Extraction

- Move `node_lifecycle` and its direct tests into `prikk-replay`.
- Move lifecycle state, validation, mutation primitives, direct tests, and direct structural equality
  helpers that do not require store IO.
- Update `prikk-store` callers to use `prikk_replay`.
- Keep replay lineage walking, object store reads, lifecycle cache persistence, verified/compared cache
  construction, doctor diagnostics, and verify diagnostics in `prikk-store`.
- Preserve all public CLI behavior and all existing test outcomes.

### Phase 3 - Replay-Local Facts and Path Effects

- Move pure path occupancy and preimage/postimage fact structures if they no longer depend on
  `prikk-store`.
- Move small evidence traits only when the object evidence boundary remains explicit.
- Keep store-backed resolver construction in `prikk-store`.

### Phase 4 - Text-Span Evaluation

- Move `text_span` only after lifecycle extraction is stable and acyclic.
- Require DC-12/DC-14 text-span vectors to remain byte-identical.
- Keep authoring/materialization integration in `prikk-store` until a later boundary justifies moving
  it.

### Phase 5 - Evaluate Patch Algebra Placement

- Reassess `patch_algebra` after lifecycle/text extraction.
- Do not move it unless dependency direction is clear and no public API is implied.
- If moved, keep visibility workspace/crate-internal where possible.

Each phase must be reviewable independently and behavior-neutral unless a later RFC explicitly widens
scope.

## Release and Compatibility Rules

DC-19 extraction work must not:

- change object ids or canonical payload bytes;
- change patch replay results;
- change CLI output except for unavoidable internal error wording explicitly reviewed;
- change repository layout;
- change ref/WAL/trust behavior;
- add public merge or confluence behavior;
- weaken fail-closed evidence semantics.

Every implementation phase must run the usual workspace gates and targeted replay/lifecycle/vector
tests.

For every extraction phase, review evidence must show no change to:

- object ids and canonical vectors;
- replay results;
- lifecycle state results;
- text-span vectors, if text-span moved;
- CLI output except explicitly reviewed internal-error wording;
- repository layout, refs, WAL, and trust behavior.

Semantic errors moved into `prikk-replay` must remain structured enough for `prikk-store` to map them
into CLI, doctor, and verify errors. Evidence errors must not be flattened into generic integrity
errors or algebraic `Unknown` results before the store layer can preserve DC-17/DC-18 distinctions.

## Test and Review Requirements

The implementation plan must include:

- full workspace `cargo test`;
- focused lifecycle/replay tests after each move;
- object/vector tests proving identity stability;
- text-span vectors after any text-span move;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- line-count and test-module placement audit following project guidelines;
- `cargo tree -p prikk-replay`, or equivalent dependency evidence, showing `prikk-replay` does not
  depend on `prikk-store`;
- explicit evidence/replay error-conversion review.

## Open Questions

1. What is the smallest evidence-reader trait that avoids leaking store layout into the semantic crate?
2. Should `patch_algebra` stay in `prikk-store` for the whole first extraction, or may its pure fact
   helpers move with replay/lifecycle types?
3. Which lifecycle cache comparison helpers, if any, are pure enough to move without moving cache
   persistence?

## Acceptance Criteria

DC-19 design is accepted when review agrees on:

- crate name and target dependency graph;
- first-wave modules eligible for extraction;
- modules that must remain in `prikk-store`;
- evidence-reader and blob-validation ownership;
- staged behavior-neutral migration plan;
- explicit deferral of standalone patch algebra and worktree extraction.

Implemented in 0.12.0 as the first behavior-neutral crate-boundary extraction.
