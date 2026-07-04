# RFC (archive) — DC-09 Phase 4 Node Model and Operation Application

**Status.** Superseded / partially implemented historical umbrella — shipped slices are captured by
DC-10 through DC-15 and follow-up DCs; any remaining active scope requires a new DC.
**Tracks.** Completion of the FDD-03 §9.3 operation surface: node-addressed
operation *application* (replay, inverse, rollback, checkout) and worktree authoring.
**Touches.** `prikk-store` patch replay/inverse/rollback/checkout, worktree patch
authoring, a new node-lifecycle model; `prikk-object` node id minting.

**Archive note.** DC-09 is preserved as historical context, not as a live implementation backlog.
Carry-forward items from this umbrella are tracked in `rfcs/IMPLEMENTATION-STATUS.md`.

## Context

The DC-09 Phase 4.2 work (increments 4.2a–4.2e, ratified) reconciled the **identity
and read-validation surface** of all seven FDD-03 §9.3 operation records:

- field layouts for `CreateFile`, `DeleteNode`, `EditText`, `ReplaceBinary`,
  `RenamePath`, `ChangePerm`, `CreateSymlink` (node-addressed where the FDD requires);
- all-zero `node_id` rejected on encode and decode;
- the §9.2 operation-kind oneof enforced on read;
- the §9.2.1 `op_seq` canonical invariant enforced on read.

What is **not** done — and what this RFC proposes — is *application* of the node-
addressed operations. Because the replay manifest is currently path-keyed, any
operation that is addressed by `node_id` rather than path (`EditText`,
`ReplaceBinary`, and the lifecycle of every node through rename/delete/recreate)
cannot be located or applied yet. Application is deferred behind a `node_id`→entry
model. Worktree authoring is fail-closed for the same reason: authoring any §9.3
mutation requires the node's `node_id`, which needs tracking and minting.

This RFC is a plan, not an implementation. It exists so the design rationale and the
deferred-work inventory are recorded rather than living only in review threads.

## Proposed increments

### 4.3 — Store decode model
Promote the per-operation decoders into a coherent decode model that yields a typed,
node-addressed operation stream (rather than the current path-keyed
`SupportedPatchOperation` subset), without changing identity bytes.

### 4.4 — Node lifecycle state and threading
Introduce `NodeLifecycleState` carried through replay/inverse/rollback/checkout:

- `live_by_id` — node_id → current live entry (path, blob, mode, kind);
- `path_to_id` — path → node_id for the current tree (authoring + lookup);
- `latest_tombstone_by_id` — node_id → most recent `Tombstone` for restoration;
- `seen_ids` — every node_id observed, for reuse/restoration-equivalence checks.

Threading rules (from the architect's 4.3/4.4 guardrails):

1. do not regress to path-only semantics for node-bearing operations;
2. enforce **restoration-equivalence** on reintroduction (a reintroduced node_id must
   match its prior identity), not merely "non-live reuse";
3. preserve `node_id` across rename through replay **and** rollback;
4. every new identity-bearing state-root or replay vector is byte-level pinned, never
   round-trip-only.

With the lifecycle model in place:

- `EditText` span-anchored apply/inverse (FDD-01 §7.2.1) can resolve span identity
  against the post-forward state;
- `ReplaceBinary` apply resolves both `old_blob_id` and `new_blob_id` and calls the
  existing `blob_access::ensure_blob_kind_is_binary` on **both** (binary-only,
  kind-preserving; text/snapshot/mixed rejected);
- worktree authoring is re-enabled for create/delete/modify.

### 4.4a — Production node id minting
A `NodeIdGenerator` backed by the OS CSPRNG, fail-closed, test-injectable. The node
model plan forbids derived/deterministic production minting. Re-enables worktree
create authoring.

### 4.5 — State root
Implement the §10.2 state-merkle-root construction over the node model; byte-level
pin its vector.

### 4.6 — Golden vectors and deep-verify
Golden replay/inverse/rollback vectors plus a mandatory deep-verify negative suite.

## Tracked carry-forward items (not yet owned by an increment above)

- **Symlink static target validator (FDD-04 §5.4a / §13.1).** Object validation must
  reject statically-decidable symlink-target problems (absolute/drive/UNC target, NUL
  or control characters, Windows-reserved component, target climb above the declared
  symlink parent depth). Applies to both `CreateSymlink.target` and the `DeleteNode`
  symlink preimage; implement once for both sites. Blocker before symlink application,
  sync ingest, or any symlink release claim. (Record identity is already reconciled;
  this is the semantic validator.)
- **Duplicate scalar-field rejection** (later validator-suite increment). Repeated scalar tags
  that currently overwrite an `Option` in decoders should be rejected. Distinct from
  the closed §9.2.1 `op_seq` duplicate check.
- **`Operation.preconditions` / `PatchPayload.preconditions`** migration to
  `record_list_item` and the FDD-03 §9.2.2 discriminator model — Phase 6.

## Release-gate note

As of v0.1.0 (Phase 4.2 closure), worktree authoring (`commit --from-worktree`) is
fail-closed and there are four DEV-ONLY ignored authoring tests. No release may claim
worktree authoring support until 4.4 / 4.4a restore it (or release notes explicitly
mark those commands unsupported).

## Open questions

- Exact `Tombstone` record shape and whether it is identity-bearing or a derived
  replay-time structure.
- Whether `RenamePath`/`ChangePerm`/`CreateSymlink` application lands in 4.4 or a
  follow-up once the lifecycle model is proven on create/delete/edit/replace.
