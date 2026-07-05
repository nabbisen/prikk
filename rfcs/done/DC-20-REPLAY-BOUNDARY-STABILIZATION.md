# RFC (done) - DC-20 Replay Boundary Stabilization

**Status.** Implemented in 0.13.0.
**Released.** 2026-07-05.
**Target release.** v0.13.0.
**Tracks.** Post-DC-19 stabilization of the workspace-internal `prikk-replay` boundary before broader
M2+ patch-algebra, merge, conflict-witness, or production confluence surfaces.
**Touches.** `prikk-replay` API shape, `prikk-store` compatibility wrappers, crate dependency
direction, ownership documentation, replay/lifecycle tests, and future extraction criteria.
**Companion handoff.** `../handoffs/DC-20-replay-boundary-stabilization/implementation-checklist.md`.

## Context

DC-19 introduced `prikk-replay` as a workspace-internal semantic crate and moved the node lifecycle
substrate plus the lexical `RepoPath` leaf out of `prikk-store`. That was the first real crate
boundary below the repository integration layer.

The move deliberately avoided larger semantic or integration shifts. `prikk-store` still owns durable
repository layout, object storage, refs, WAL, active sessions, lifecycle-cache persistence,
verification, doctor, trust persistence, worktree integration, store-backed resolver construction, and
CLI-facing behavior. `prikk-replay` owns only the moved replay/lifecycle semantic substrate.

The next risk is boundary drift. If new M2+ work immediately moves patch algebra, text-span logic,
resolver construction, or worktree behavior without first stabilizing the new boundary, the crate graph
can become confusing: developers may not know whether `prikk-replay` is a semantic replay crate, a
repository integration crate, or a future public API.

DC-20 is a design-first stabilization pass. It does not broaden behavior. It records what the current
boundary means after DC-19, what must stay out of `prikk-replay`, and what evidence future extraction
RFCs must provide.

## Design Goals

1. Stabilize the post-DC-19 `prikk-replay` boundary before adding larger M2+ surfaces.
2. Make ownership rules explicit for semantic replay/lifecycle code versus repository integration code.
3. Keep `prikk-replay` workspace-internal and avoid implying external API stability.
4. Preserve the dependency direction: `prikk-store` may depend on `prikk-replay`; `prikk-replay` must
   not depend on `prikk-store`.
5. Audit public reexports and compatibility wrappers so existing call sites have a clear migration
   path without duplicating semantic ownership.
6. Add focused tests and review evidence that prove the boundary remains behavior-neutral.
7. Define readiness criteria for later extraction of text-span, path/preimage facts, evidence traits,
   or patch-algebra helpers.

## Non-goals

DC-20 does not add:

- CLI merge, branch merge, branch switching, or public confluence commands;
- persisted merge evidence, conflict-witness, proof, or rollback-ref objects;
- Patch, Block, RefState, RefUpdate, trust, WAL, or repository-layout schema changes;
- text-span extraction;
- patch-algebra extraction;
- worktree crate extraction;
- store-backed resolver movement into `prikk-replay`;
- lifecycle-cache persistence movement into `prikk-replay`;
- public API stability for `prikk-replay`;
- behavior changes hidden inside boundary cleanup.

## Proposed Design

### Boundary Definition

`prikk-replay` is the workspace-internal semantic replay/lifecycle crate.

It may own:

- lifecycle state data structures;
- lifecycle mutation and query primitives;
- lifecycle validation helpers;
- lexical repository-relative path identity, normalization, equality, and validation needed by
  lifecycle state;
- path-occupancy facts over lexical repository paths;
- pure replay-local facts that do not require durable repository IO;
- future semantic helpers that operate on already-supplied objects, facts, or decoded operations.

It must not own:

- `.prikk/` repository layout;
- file-backed object storage;
- ref pointers, ref logs, ref publication, or ref recovery;
- WAL records, active-session locks, or active-WAL metadata;
- trust-store persistence or signer lookup;
- verification and doctor repository IO;
- lifecycle-cache persistence, invalidation, or trust policy;
- block lineage walking and replay-horizon resolution;
- filesystem-root joining, host-path materialization, or platform filesystem policy;
- worktree scanning, authoring, status, checkout, or materialization policy;
- store-backed resolver construction;
- CLI-facing command behavior.

`prikk-store` remains the repository integration crate. It may expose compatibility modules for moved
types, but semantic source ownership should be documented as `prikk-replay`.

### `RepoPath` Boundary

`RepoPath` is acceptable in `prikk-replay` only as a lexical semantic type used by lifecycle and replay
facts.

Allowed `prikk-replay` responsibilities:

- parse and validate repository-relative path strings;
- reject absolute paths, parent traversal, repository-private paths, platform-ambiguous path spellings,
  and duplicate/colliding lexical paths;
- provide stable ordering, equality, hashing, and string access for lifecycle/path-occupancy state.

Disallowed `prikk-replay` responsibilities:

- join a `RepoPath` to a repository root or worktree root;
- decide where or how bytes are written to the host filesystem;
- scan the worktree;
- perform checkout/materialization;
- encode platform-specific filesystem recovery or repair policy.

If any current or future `RepoPath` helper starts to smell like filesystem materialization rather than
lexical identity, DC-20 implementation must either leave it as a `prikk-store` compatibility concern
with a carry-forward note, or move the behavior back toward an integration/worktree boundary.

### Internal API Policy

`prikk-replay` remains workspace-internal for DC-20. Public Rust items are permitted when needed for
workspace integration, but they do not create an external stability promise.

The crate docs and README should say:

- the crate is internal and experimental during the current boundary-stabilization phase;
- external users should not treat its `pub` surface as stable;
- repository IO and CLI behavior remain mediated through `prikk-store`.

Any later decision to publish or stabilize a `prikk-replay` API must be a separate RFC.

### Reexport and Compatibility Policy

Compatibility wrappers in `prikk-store` are acceptable during stabilization when they reduce churn for
existing callers. They must not become a second implementation location.

Rules:

- reexports should point to `prikk-replay` types rather than redefining equivalent structs;
- docs should avoid implying that `prikk-store` owns lifecycle semantics after DC-19;
- new semantic lifecycle behavior should be added to `prikk-replay`, not to compatibility wrappers;
- callers may continue through `prikk-store` where that is the stable integration surface.

This keeps the current release behavior intact while leaving room for gradual migration.

Initial compatibility-wrapper inventory:

| Surface | Current path | Semantic owner | Keep / migrate / remove later | Reason |
|---|---|---|---|---|
| Lifecycle compatibility imports | `crates/prikk-store/src/node_lifecycle.rs` | `prikk-replay` | Keep during DC-20 | Existing store modules and tests import lifecycle types through `crate::node_lifecycle`; the file is import-only and avoids churn while the boundary stabilizes. |
| Repository path compatibility reexports | `crates/prikk-store/src/path.rs` and `prikk_store::RepoPath` | `prikk-replay` for lexical path identity; `prikk-store` for integration use | Keep during DC-20 | `RepoPath` remains part of the current `prikk-store` integration surface, while lexical validation is owned by `prikk-replay`. |

Implementation review must update this inventory if any wrapper is added, removed, or changed. A
wrapper that starts accumulating semantic logic is a design failure unless the RFC is amended.

### Dependency Direction Gate

Every implementation of DC-20 must prove that `prikk-replay` does not depend on `prikk-store`.

Required evidence includes:

```text
cargo tree -p prikk-replay
```

and either:

```text
cargo metadata --format-version 1
```

or a short dependency summary proving all of the following:

- normal dependencies of `prikk-replay` do not include `prikk-store`;
- dev-dependencies of `prikk-replay` do not include `prikk-store`, unless a test-only exception is
  explicitly justified in review;
- no feature flag can enable a `prikk-replay -> prikk-store` dependency;
- `crates/prikk-replay/Cargo.toml` keeps `publish = false`.

The gate is not merely cosmetic. If `prikk-replay` depends on `prikk-store`, the boundary has inverted
and the design should be rejected or revised.

### File Layout and Test Placement

DC-20 should keep the DC-19 lifecycle split maintainable:

- implementation modules should stay below the project file-size guidance;
- direct lifecycle tests should remain outside implementation files;
- new tests should be placed under the existing `node_lifecycle/tests/` layout or equivalent sibling
  test modules;
- no new `#[allow(dead_code)]` or `#[allow(unused_imports)]` should be introduced for convenience.

If a file grows past the guideline, split it by behavior boundary, not by arbitrary line count alone.

### Boundary Tests

The implementation should add or confirm focused tests for:

- lifecycle behavior through `prikk-replay` direct APIs;
- compatibility imports through `prikk-store`, where those imports remain part of current integration;
- path validation behavior after the `RepoPath` move;
- store-level replay/cache callers still producing the same lifecycle state;
- no observable CLI behavior change caused by import movement or wrapper cleanup.

The intent is to catch accidental semantic duplication or ownership drift, not to add new product
behavior.

### Future Extraction Readiness

DC-20 should define what a later RFC must prove before moving more code into `prikk-replay`.

For `text_span`, a later RFC must show:

- DC-12/DC-14 text identity, localization, splice, and inverse vectors remain byte-identical;
- authoring and replay continue to share one implementation;
- worktree integration does not pull filesystem policy into `prikk-replay`;
- object/hash dependencies remain acyclic and minimal.

For replay-local path/preimage facts, a later RFC must show:

- the facts are pure semantic inputs or outputs;
- store-backed evidence lookup remains in `prikk-store`;
- evidence error categories remain distinguishable.

For patch-algebra helpers, a later RFC must show:

- whether they are replay-semantic helpers, store-backed analysis helpers, or public merge planning
  APIs;
- DC-17/DC-18 evidence-error precedence is preserved;
- no public conflict/merge API is implied accidentally;
- production caller needs are clear enough to justify the move.

For resolver construction, a later RFC must show a compelling reason. The default is that resolver
construction stays in `prikk-store` because it binds durable object storage, sealed baselines, and
repository lineage to semantic facts.

## Migration Plan

### Phase 1 - Boundary Audit

- Audit `prikk-replay` public exports and crate docs.
- Audit `prikk-store` compatibility wrappers for lifecycle/path types.
- Confirm no semantic duplicate implementation remains in `prikk-store`.
- Confirm dependency direction with `cargo tree -p prikk-replay`.

### Phase 2 - Documentation and Ownership Cleanup

- Update crate README/docs where they still describe DC-19 as the active state rather than the
  post-DC-19 stabilization state.
- Document which surfaces are workspace-internal and which surfaces are compatibility imports.
- Keep release claims behavior-neutral.

### Phase 3 - Focused Test Hardening

- Add focused tests only where the audit finds missing coverage.
- Keep tests outside implementation files.
- Prefer behavior tests over tests that assert module paths or implementation details.

### Phase 4 - Review Evidence

- Provide dependency evidence.
- Provide file-size and test-placement evidence.
- Provide workspace gate evidence.
- Provide a list of moved/edited files.
- State which modules changed logic, and which edits were import/doc/test-only.
- Provide identity/vector, focused lifecycle, and store-level replay/cache test evidence.
- State that CLI output did not change, or list exact wording-only changes if review accepted them.
- List any compatibility wrapper retained intentionally.
- List every future extraction candidate explicitly deferred.

## Release and Compatibility Rules

DC-20 must not change:

- object ids or canonical payload bytes;
- patch identity or replay order;
- lifecycle semantics;
- text-span identity or inverse behavior;
- repository layout;
- ref/WAL/trust behavior;
- verification or doctor semantics;
- CLI output except for explicitly reviewed internal wording.

If cleanup exposes a bug, the bug fix should be split into a separate reviewed change unless review
agrees it is inseparable from the boundary stabilization.

Release notes and implementation status for DC-20 must explicitly record that these remain deferred:

- `text_span` extraction;
- patch-algebra extraction;
- store-backed resolver movement;
- lifecycle-cache persistence movement;
- worktree extraction;
- public `prikk-replay` API stabilization;
- public merge, confluence, and conflict surfaces.

## Test and Review Requirements

Implementation review should include:

- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo fmt --check`;
- `git diff --check`;
- focused `prikk-replay` tests;
- focused `prikk-store` compatibility/replay tests if wrappers are adjusted;
- `cargo tree -p prikk-replay` dependency evidence;
- `cargo metadata --format-version 1` evidence or an equivalent dependency summary covering normal
  dependencies, dev-dependencies, feature-enabled dependencies, and `publish = false`;
- file-size and test-module placement audit;
- moved/edited-file inventory;
- logic-change versus import/doc/test-only summary;
- identity/vector, focused lifecycle, and store-level replay/cache test evidence;
- explicit statement that no CLI/schema/repository-layout behavior changed.

## Open Questions

1. Which `prikk-store` compatibility wrappers should remain as stable integration surfaces, and which
   should be migrated away from internally? Initial answer: keep the current import-only
   `node_lifecycle` and `path` compatibility surfaces during DC-20, inventory them, and prevent them
   from accumulating semantic logic.
2. Should the crate docs say "during DC-20" or use a version-neutral phrase such as "during boundary
   stabilization" to avoid stale wording after 0.13.0? Answer: use version-neutral wording such as
   "during replay-boundary stabilization" or "while `prikk-replay` is workspace-internal."
3. Is `RepoPath` now stable enough as a `prikk-replay` leaf type, or should future path-policy work
   revisit whether lexical path validation belongs in a lower utility crate?
4. What is the smallest future evidence-reader trait that would not leak store layout into
   `prikk-replay`?

## Acceptance Criteria

DC-20 design is accepted when review agrees on:

- the post-DC-19 ownership rules for `prikk-replay` and `prikk-store`;
- the internal API and compatibility-wrapper policy;
- the dependency-direction gate;
- the concrete compatibility-wrapper inventory;
- the lexical-only `RepoPath` boundary;
- file-size and test-placement expectations;
- focused stabilization test requirements;
- behavior-neutral review evidence requirements;
- version-neutral crate documentation wording;
- explicit deferral of text-span, patch-algebra, worktree, resolver, cache-persistence, CLI, schema,
  and public merge/conflict surfaces.
