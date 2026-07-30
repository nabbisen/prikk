# RFC (proposed) - DC-61 Branch Closure

**Status.** **Accepted by the project owner on 2026-07-30.** All three §3 verification obligations were
discharged at design review before acceptance — one surfaced an unnamed cost (schema-blind decoding, 10
non-test call sites) now in scope. **Design revised 2026-07-30** after design review v1
(`.git-exclude/reviewed/prikk-dc61-design-review-v1.md`) rejected the original tombstone design. Owner
approved the redirection to closure the same day.
**Renamed.** Was "Branch Deletion and Ref-Log Tombstones." The tombstone approach is abandoned; see §1.
**Split from.** DC-60, whose scope was amended 2026-07-30 to `list` and `create` only.
**Requirement.** `specs/prikk-app-requirements-v1.2.md` §6.5, the deletion half.
**Touches.** `RefStatePayload` (one new field, schema bump), **schema-aware decoding threaded through its 10
non-test call sites** (see §3 obligation 1), `branch` CLI (`close`, and `list` filtering), and
format-transition handling. **Not** `verify`, **not** `publish`, **not** `doctor` — see §2.

## 1. Why the tombstone design was abandoned

DC-60 specified `branch delete` as "remove the pointer, retain the log," which bricked repository-wide
commits because the system classifies pointer-absent-log-present as corruption. DC-61 v1 proposed teaching
the system to tolerate that state via a ref-log tombstone. Design review v1 found two reasons that is wrong.

**It is expensive.** Every ref-log record must be a signed `ObjectType::RefUpdate`
(`refs/log.rs:142` `require_signed_type`), and `RefUpdatePayload` cannot express a deletion — 
`new_ref_state_id` and `new_target_object_id` are non-optional `ObjectId`
(`prikk-object/src/payload/refs.rs:129-132`). So a tombstone needs a new object type or a payload schema
bump, plus a `verify` arm and a `publish` arm.

**It fights an existing repair path — the decisive reason.** `refs.rs:210` `recoverable_missing_ref` detects
exactly pointer-absent-log-present and returns a recovery candidate; `refs.rs:271`
`reconstruct_missing_ref_from_log` rebuilds the pointer from the log; and `doctor.rs:174` exposes
`ref_repair: Option<RefRecoveryRepair>` to users.

**So `doctor` would offer to resurrect every deliberately deleted branch.** That state is not merely
*classified* as damage — the system has a repair function for it, wired into a user-facing command. DC-61 v1
did not list either function among the places needing change, so the tombstone footprint was five sites, not
three.

**The conclusion is architectural, not preferential:** deletion must not manufacture a state the system has a
repair function for.

## 2. Design: closure, not deletion

**The pointer stays.** "Deleting" a branch publishes a final ref state marking it **closed**.

Everything that made the tombstone expensive disappears:

| | Tombstone | Closure |
|---|---|---|
| Pointer afterward | absent → the repairable-damage state | **present** |
| `verify` `classify_ref_state` | new arm, must not weaken corruption detection | **unchanged** |
| `publish` `classify_state` | new fifth arm | **unchanged** — an ordinary CAS update |
| `recoverable_missing_ref` / `doctor` | must learn not to resurrect | **unchanged** — returns `None` at `refs.rs:211-213` because the pointer is present |
| Ref-log format | new object type or schema bump | **unchanged** — an ordinary `RefUpdate` |

NFR-REL-01 is satisfied by construction rather than by argument: nothing is removed.

### 2.1 Encoding — a new `RefStatePayload` field, and this is still a format change

**Both candidate encodings are hard breaks for older readers.** Verified:

- `RefStatePayload::decode_canonical` rejects unknown field tags outright:
  `other => return Err(MalformedData("unknown RefState field tag: {other}"))`
  (`prikk-object/src/payload/refs.rs`, the `while let Some(field)` match).
- `RefKind::from_code` rejects unknown codes: `other => Err(MalformedData("unknown ref kind code"))`.

So there is **no forward-compatible route**. A new field and a new `RefKind` variant are the same class of
break, and DC-40's format-transition machinery applies either way. **DC-61 remains a format-change
increment** — smaller than the tombstone, not free. Do not let §2's table suggest otherwise.

**Choose a new field over a `RefKind` variant**, because `RefKind` encodes *what a ref is* (Branch, Tag) and
closure is *what state it is in*. Overloading kind with state forces `ClosedBranch`/`ClosedTag` and multiplies
with every future kind.

### 2.2 Closure must be signed

**Rejected: a non-object marker file** (e.g. `refs/closed/<key>`), which would need no format change at all
and was tempting for that reason.

The reason it fails is authority. Every other ref state change is maintainer-signed
(`prikk-object/src/signature.rs:49-50`, `Maintainer = 2`, "publishing/sealing a block or ref state"), and
DC-34/DC-39 built the signature-authority model around that. An unsigned marker would let a branch be hidden
by a plain file write, bypassing the authority every other ref mutation requires — a real weakening of
NFR-SEC-02's role-bound signatures for a saving in format work.

Closure is therefore an ordinary signed ref-state publication carrying the closed field.

Note this is the inverse of design review v1's reasoning about the dev team's marker proposal, and for a
different reason: there, a marker would have been load-bearing for *integrity classification*. Here the
pointer is present, so a marker would be load-bearing for *authority*. Both disqualify it; the mechanism
differs.

### 2.3 Command surface

- **`prikk branch close <name>`** — publishes the closing ref state. Named `close`, not `delete`, because
  nothing is deleted and the command should not claim otherwise.
- **`prikk branch list`** hides closed refs by default; **`--all`** shows them, marked.
- **Reopening** is an ordinary CAS update from the closed state. Permitted — nothing structurally prevents
  it, and refusing would be an arbitrary restriction.

Fail closed when: `<name>` does not exist; `<name>` is already closed; `<name>` owns a non-empty active WAL
(reuse `require_active_ref_for_non_empty_wal`, citing **DC-13 goal 4**, do not restate the rule).

**No "last remaining branch" guard is needed** — unlike deletion, closure leaves the ref and its history
reachable, so a repository whose only branch is closed is recoverable by reopening it.

**Output must state that nothing was reclaimed and the branch remains recoverable.** A user typing what they
think is "delete" should learn immediately that it is not.

## 3. Verification obligations — DISCHARGED at design review, 2026-07-30

*All three were discharged by the architect before acceptance, per v1's structure. Results below; the design
survives, with one cost it had not named.*

### Obligation 1 — format transition: expressible, but the decoder is schema-blind

**Mixed schemas are structurally supported.** `schema_version` is a per-envelope `u32`
(`prikk-object/src/envelope.rs:26`) and part of the ObjectId preimage (`envelope.rs:143`,
`writer.field_u32(2, self.schema_version)`). Ref-state envelopes are built by the *caller* and handed to
`publish` as `RefPublication.ref_state` (`refs.rs:354`), so `branch close` can emit schema 2 while every
ordinary publication stays schema 1.

**Existing identities are therefore untouched.** Only closed ref states carry the new field, so no existing
`RefState` payload changes and no existing ObjectId moves.

**The cost DC-61 had not named:** `RefStatePayload::decode_canonical` takes **only bytes**
(`payload/refs.rs:56`) — it is schema-blind, and rejects unknown field tags unconditionally. So a
schema-gated field cannot simply be added; **schema awareness must be threaded into decoding**, which touches
every caller. There are **10 non-test call sites** across `prikk-cli/src/seal/support.rs` (×2),
`prikk-store/src/{rollback_draft, refs/publication, merge_evidence, history, checkout, refs, patch_inverse/read}.rs`,
and `prikk-cli/src/branch.rs`; **22 including tests**.

That is real but bounded, and it is mechanical rather than architectural. **It must be in the handoff's scope
statement**, not discovered during implementation.

### Obligation 2 — `verify`, `publish`, `recoverable_missing_ref`, `doctor` need no change: CONFIRMED

- **`verify`** enumerates pointers and validates each. A closed ref is a normal signed ref state with its
  pointer present, so `classify_ref_state` takes its ordinary `(Some, Some)` arms.
- **`publish`** treats closure as an ordinary CAS update — arm 1, `current == expected && tip == expected`.
  **No same-target restriction exists** in `publication.rs`, so reusing the target object id is permitted.
- **`recoverable_missing_ref`** returns `None` at `refs.rs:211-213` because the pointer is present.
- **`doctor`** consumes `RefRecoveryRepair`, which only arises from the missing-pointer path. Nothing is
  offered.

Criterion 4 remains the falsification test: if a diff touches any of the four, this obligation's result was
wrong and the design's cost advantage must be re-examined.

### Obligation 3 — no existing code assumes refs in `by-id/` are live: CONFIRMED

Only **two** consumers enumerate ref pointers: `refs/verify.rs:44` `read_pointers`, and `refs.rs:177`
`list_ref_pointers` (added by DC-60). Every other path — checkout, log, status — resolves refs **by name**
via `DEFAULT_CHECKOUT_REF`, never by enumeration.

`verify` makes no liveness assumption. `list_ref_pointers` is the function this RFC already modifies. So the
filter has exactly one place to live.

**I predicted this obligation would bite and it did not.** Recorded because the prediction was wrong in a
useful direction: the ref layer's consumers are narrower than they appear, and the "existing consumer of a
changed state" risk that killed v1 does not recur here.

## Non-goals

- **No deletion.** Nothing is removed from disk. Garbage collection is NFR-REL-02.
- No `branch switch` or current-branch pointer — still deferred, still better after the queuing decision.
- No tagging (§6.6), no remote branches (§6.11).
- No change to `branch list`'s existing output shape beyond the closed filter, and no change to
  `branch create`, both shipped under amended DC-60.
- No relaxation of corruption detection anywhere. This design should not touch it at all.

## Risks

**An existing ref consumer that assumes liveness.** Obligation 3. The most likely defect.

**"Close" read as "delete" by users.** Mitigated by naming and output text, but the semantic difference is
real and permanent: disk usage never drops. State it plainly rather than softening it.

**Format transition scope creep.** A schema bump touches canonical encoding and identity vectors. If it
starts to look like DC-40, stop and report rather than absorbing it.

## Acceptance criteria

1. `branch close` publishes a maintainer-signed ref state carrying the closed field; the pointer remains
   present; `verify` passes cleanly afterward.
2. **A commit to an unrelated ref succeeds after a closure** — the DC-60 regression, tested by committing to
   a ref the closure never touched.
3. **Corruption detection is unchanged**: pointer-absent-log-present is still reported and still blocking, at
   every record count, tested by simulating pointer loss as
   `seal_rejects_missing_pointer_with_ref_log_history` does.
4. `verify`, `publish`, `recoverable_missing_ref`, and `doctor` are **unmodified** — evidenced by the diff.
   If any changed, criterion 3 and the design's justification both need re-examination.
5. `branch list` hides closed refs; `--all` shows them marked; both tested.
6. Reopening a closed branch succeeds as an ordinary CAS update; `verify` passes afterward.
7. `branch close` fails closed on a missing branch, an already-closed branch, and a branch owning a
   non-empty active WAL — each tested against constructed state.
8. Format transition evidenced per obligation 1, including behaviour when an older reader encounters a closed
   ref state.
9. Output states that nothing was reclaimed and the branch remains recoverable.
10. No identity artifact changes beyond those the schema bump requires, each accounted for individually:
    `vectors/snapshot.txt`, `vectors/hard.rs`, `state_root/tests/vectors.rs`, `text_span/vectors.rs`.
11. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

Criteria 2, 3, and 4 are load-bearing. **Criterion 4 is the design's own falsification test** — this RFC
argues closure is cheap *because* those four are untouched, so a diff that touches them refutes the argument
rather than merely complicating it.
