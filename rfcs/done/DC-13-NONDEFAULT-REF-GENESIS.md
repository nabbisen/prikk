# RFC (done) - DC-13 Non-Default Ref Genesis

**Status.** Implemented (v0.6.0)
**Target release.** v0.6.0.
**Tracks.** Allowing first-commit genesis on explicitly selected non-default branch refs while keeping
branch lifecycle, ref recovery, and publication trust fail-closed.
**Touches.** `prikk commit --ref`, `prikk seal --ref`, ref-name validation, active-WAL ownership
metadata, ref recovery diagnostics, CLI output/docs, and tests.
**Companion FDD updates.** `../handoffs/DC-13-nondefault-ref-genesis/fdd-02-update.md`,
`../handoffs/DC-13-nondefault-ref-genesis/fdd-03-update.md`.

## Context

v0.5.0 made worktree text edits local enough for future patch algebra. The next roadmap gap is the
default-ref-only genesis rule introduced during DC-09 4.4b: `init -> commit -> seal` works for
`heads/main`, but an unborn non-default ref still fails closed until branch-creation semantics are
designed.

The storage layer already has the important primitive:
`RefPublication.expected_previous_ref_state_id = None` creates a ref through the same signed
`RefState` and inline `RefUpdate` path used by default-ref genesis. DC-13 should therefore be a narrow
branch-lifecycle design pass, not a new ref object model.

## Design goals

1. Permit explicit first commit onto an unborn branch ref such as `heads/topic`.
2. Publish that unborn branch with `seal --ref heads/topic` as a signed Root block at update sequence 1.
3. Preserve existing corruption distinctions: missing pointer plus non-empty log is not genesis.
4. Prevent a queued active WAL from being sealed to a different ref than the ref it was authored for.
5. Keep non-default genesis explicit; never infer or auto-create refs from checkout, status, log, or
verification commands.
6. Keep v0.6.0 single-commit-per-active-WAL semantics; queued multi-commit sessions remain deferred.
7. Avoid claims about merge, branch switching, tags, rollback refs, remote refs, or multi-head policy.

## Proposed design

### Scope

DC-13 supports unborn **branch** refs only. A valid DC-13 target ref must:

- be explicitly supplied with `--ref`;
- start with `heads/`;
- pass the repository's storage-safe ref-name checks;
- have no current pointer and an empty, readable ref log.

The default `heads/main` remains the CLI default. Existing published refs keep their current behavior.

Out of scope:

- tag creation or tag movement;
- remote-tracking refs;
- symbolic `HEAD` or current-branch state;
- branch deletion, rename, switch, merge-base, or checkout branch creation;
- rollback-specific refs;
- publication policies beyond the DC-11 local maintainer trust check.

### CLI surface

`commit` already accepts `--ref`; DC-13 changes its unpublished non-default behavior:

```text
prikk commit --ref heads/topic -m "start topic"
```

If `heads/topic` is unborn and has an empty readable ref log, authoring uses the same empty baseline as
default-ref genesis. All worktree files become `CreateFile` operations, with existing DC-09/DC-12
authoring rules for node ids, modes, text spans, signatures, and active-WAL serialization.

This creates a new independent Root history for `heads/topic` from the current worktree. It does not
copy or fork the current `heads/main` tip, switch the checkout branch, or create a merge base with
`heads/main`.

`seal` gains a matching explicit ref selector:

```text
prikk seal --allow-no-audit --ref heads/topic
```

When the target ref is unborn, seal writes:

- a signed Root `Block` with no parents;
- a signed `RefState` with `ref_name = "heads/topic"`, `kind = Branch`, `update_seq = 1`, and
  `previous_ref_state_id = None`;
- a signed inline `RefUpdate` with `old_ref_state_id = None`;
- a ref pointer selected through the existing ref-specific lock and CAS path.

`seal` without `--ref` continues to publish `heads/main`.

### Active-WAL ref ownership

The active WAL is still a single queue in DC-13. Because `commit --ref heads/topic` and
`seal --ref heads/main` would otherwise be ambiguous, DC-13 introduces a small active-session metadata
record:

```text
.prikk/active/default/ref-name
```

The file contains the exact validated canonical ref string for the current active WAL, with no trailing
newline and no lossy conversion. It is written and fsynced under the active lock before the first WAL
record is appended. Seal must read it under the active lock and fail closed if it differs from the
requested seal ref.

Rules:

- Empty WAL with missing metadata is allowed.
- A crash after metadata fsync but before WAL append leaves empty WAL plus stale metadata, which is
  safe and non-authoritative.
- Metadata creation or replacement must include active-directory durability before the first WAL append:
  write a temporary file under `.prikk/active/default/`, fsync it, atomically rename it to `ref-name`,
  fsync the active-session directory, and only then append the first WAL record. A direct write path is
  acceptable only if it provides equivalent crash semantics and malformed empty-WAL cleanup remains
  deterministic.
- Non-empty WAL with missing or malformed metadata fails closed. DC-13 does not include a legacy
  `heads/main` compatibility exception because there is no reliable pre-DC-13 session discriminator.
- Empty WAL with stale, malformed, or ref-mismatched metadata is non-authoritative and is removed by
  commit/seal after taking the active lock and fsyncing the active-session directory.
- After successful seal drains the active WAL, seal removes `active/default/ref-name` under the active
  lock and fsyncs the active-session directory.
- `doctor` should diagnose mismatched or stale metadata but must not rewrite it in DC-13 unless a
  separate repair is designed.

This keeps DC-13 within the current single-active-session architecture while preventing accidental
cross-ref publication.

### Non-empty active WAL

DC-13 keeps the existing single-commit-per-active-WAL behavior. Any `commit --ref <r>` that sees a
non-empty active WAL fails closed with a "seal first" diagnostic, even if active-WAL ref metadata also
says `<r>`. This avoids introducing queued multi-commit baseline semantics before the patch replay,
precondition, and parentage rules for active-session stacking are designed.

The metadata still matters for publication: `seal --ref <r>` may seal the queued patch only when the
active-WAL metadata exactly matches `<r>`.

Seal must hold the active-session lock across the full active-session decision: metadata read and
validation, WAL replay and partial-tail validation, ref ownership comparison, publication, WAL drain,
metadata removal, and active-directory fsync. Ref-specific publication locking is still required for the
selected ref. To avoid deadlocks, DC-13 uses active-session lock first, then the ref-specific lock inside
publication.

### Genesis selection

`resolve_worktree_baseline(layout, ref_name)` should classify an unborn branch ref as genesis when all
of the following are true:

1. `ref_name` is a valid local branch ref (`heads/...`);
2. the ref pointer is absent;
3. the ref log is absent, or exists and is readable;
4. the ref log has zero valid records;
5. the ref log has zero trailing partial bytes;
6. the active WAL guard confirms there are no records, no trailing partial bytes, and no conflicting
   active-WAL ref metadata.

Absent pointer plus absent log is treated as a readable empty log for a valid unborn branch ref. Absent
pointer plus an existing readable empty log is also genesis-eligible. Any missing pointer with ref-log
history remains corruption/recovery, never a new genesis. Any unreadable, malformed, or partial ref log
remains corruption. A non-branch ref name is an invalid target, not a genesis case.

### Ref-name validation

DC-13 should make branch target validation explicit before enabling non-default genesis. The validator
must be single-sourced and return the canonical identity string used by command logic, active-WAL
metadata, `RefState`, and `RefUpdate`. Ref names are storage-hashed today, but human ref names are
still durable identity fields in `RefState` and `RefUpdate`. The implementation must reject:

- empty refs;
- names without the `heads/` prefix for DC-13 genesis;
- `heads/` with no branch component;
- path traversal segments (`.` or `..`);
- duplicate separators and leading/trailing separators after `heads/`;
- NUL or control characters;
- names that collide with reserved namespaces selected by future refs, such as `tags/`, `remotes/`, and
  `rollback/`.

Strict UTF-8 is required; lossy conversion is forbidden. NFC and case-collision posture follow the
project's existing policy: DC-13 performs no Unicode normalization and treats ref names as exact
byte-preserving UTF-8 strings after validation. This is a DC-13 local branch policy. Broader ref
namespace policy can expand later without changing the Root-block publication semantics.

New command targets must use this validator. Object decoding of historical `RefState` and `RefUpdate`
payloads should remain compatibility-aware unless a later schema design declares invalid historical ref
names unreadable.

Active-WAL metadata reads must use the same validator before comparison. Seal compares canonical
validated ref strings byte-for-byte. Invalid metadata with a non-empty WAL is an active-session
integrity failure; invalid metadata with an empty WAL follows the cleanup rule above.

### Ref publication ordering

DC-13 does not change `RefState` / `RefUpdate` identity, but the implementation gate must preserve the
FDD-02 crash-safety discipline: the signed ref-update event for the transition is durably
staged/journaled before pointer promotion, so recovery can complete or diagnose the original transition
rather than infer one from a promoted pointer. DC-13 must not codify or introduce pointer-before-log as
the normative storage transaction.

The safe outline is:

1. acquire the ref-specific lock;
2. construct and sign the `RefState` and `RefUpdate` for the same transition;
3. persist the `RefState` object;
4. durably stage or journal the signed `RefUpdate` transition according to FDD-02;
5. verify the current pointer still equals `expected_previous_ref_state_id`;
6. promote the pointer candidate atomically;
7. finalize the log entry and fsync files/directories according to the existing crash matrix.

### Verification and recovery

Repository verification already validates ref pointer/log consistency by ref name. DC-13 should extend
tests and diagnostics so a created non-default branch ref is covered by the same structural and
publication-trust checks as `heads/main`.

Doctor repair remains narrow. Existing `doctor --repair-main-ref` stays default-ref-only. General
`--repair-ref <ref>` is deliberately deferred; DC-13 may add diagnostics that say a non-default ref is
recoverable from its log, but it must not silently repair arbitrary refs.

### Diagnostics

CLI errors should distinguish these cases:

- invalid target ref, such as `tags/x`, `remotes/origin/x`, `rollback/x`, `heads//x`, or traversal;
- unborn branch with absent pointer, readable empty log, and clean active WAL, which is allowed;
- absent pointer plus non-empty, partial, or unreadable log, which is a doctor/recovery case;
- active WAL owned by another ref, which requires sealing that ref or clearing the active session
  through a future repair path;
- non-empty active WAL with missing or malformed ref metadata, which is an ambiguous active-session
  integrity failure.

### Compatibility

- Existing repositories with only `heads/main` are unchanged.
- Existing Patch, Block, RefState, and RefUpdate canonical encodings are unchanged.
- New non-default genesis histories produce ordinary Root blocks, not a new block kind.
- Object identity changes only because the selected ref name is part of `RefState` and `RefUpdate`
  payloads.
- The active-WAL ref metadata is local session state, not an object and not content-addressed history.

## Implementation plan

1. Review this RFC and companion FDD updates before implementation.
2. Add a shared branch-ref validator for DC-13 command targets.
3. Add active-WAL ref ownership metadata read/write helpers under the active lock.
4. Implement metadata writes as temp-file fsync, atomic rename, and active-directory fsync before the
   first WAL append.
5. Remove the default-ref-only guard from genesis baseline resolution and replace it with the branch-ref
   validation plus absent-or-empty-log checks.
6. Add `seal --ref <heads/name>` parsing and use the selected ref through publication and output.
7. Make seal fail closed when active-WAL ref metadata is missing, malformed, or different from the
   requested publication ref for any non-empty active WAL.
8. Preserve single-commit-per-active-WAL behavior: a second commit before seal fails closed even when
   metadata matches.
9. Hold the active-session lock across seal metadata validation, WAL replay, ref publication, WAL drain,
   metadata removal, and active-directory fsync.
10. Ensure ref publication uses the FDD-02 journal-before-pointer crash-safety ordering.
11. Add store and CLI tests for unborn non-default commit, seal, log, verify, ownership metadata, and
   negative mismatches.
12. Update README, docs, roadmap, implementation status, changelog, and upgrade notes for v0.6.0.

## Test gates

Required positive tests:

- `commit --ref heads/topic` on a fresh repository authors all files as `CreateFile`;
- active-WAL ref metadata is durable before the first WAL record;
- `seal --allow-no-audit --ref heads/topic` publishes a Root block and update sequence 1;
- successful seal removes active-WAL ref metadata;
- `log --ref heads/topic` and `verify` see the non-default branch history;
- a second commit on the now-published non-default ref authors against replay-derived state.

Required negative tests:

- non-branch ref names fail before WAL or object mutation;
- unborn ref with existing ref-log history is corruption, not genesis;
- partial/unreadable ref log blocks genesis;
- active WAL authored for `heads/topic` cannot be sealed to `heads/main`;
- non-empty WAL with conflicting active-WAL ref metadata blocks further commit;
- non-empty WAL with missing or malformed active-WAL ref metadata fails closed;
- empty WAL with stale, malformed, or ref-mismatched metadata is cleaned deterministically;
- concurrent first commits for different refs serialize so only one can claim the empty active WAL;
- absent pointer plus absent log is eligible unborn genesis for a valid branch ref;
- invalid branch refs fail before object, blob, WAL, or metadata mutation;
- RefState and RefUpdate golden vectors remain unchanged;
- Patch object identity is unchanged by the selected ref name;
- doctor does not repair non-default refs through `--repair-main-ref`.

## Rejected alternatives

### Add a full `branch create` command first

Rejected for DC-13. An unborn branch ref can be created by publishing its first Root block through the
existing signed ref-state path. A separate branch command becomes useful once branch copy/fork semantics
from an existing block are designed.

### Treat any missing pointer as unborn

Rejected. DC-09 already established the critical safety rule: pointer absence plus log history is
recoverable corruption, not a blank ref. DC-13 preserves that rule for every branch ref.

### Add one active WAL per ref

Rejected for this increment. Per-ref active queues are a larger transaction and UX change. Active-WAL
ref ownership metadata provides the needed safety without changing the active-session storage model.

### Allow queued multi-commit active sessions

Rejected for v0.6.0. Once the active WAL is non-empty, a matching-ref second commit would need to author
against the published ref plus active-WAL replay and define precondition/parentage semantics for
session stacking. DC-13 keeps the current "seal first" rule.

### Allow tags and rollback refs

Rejected. Tags and rollback refs carry different policy and lifecycle semantics. DC-13 is branch-only.
