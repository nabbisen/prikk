# RFC (proposed) - DC-61 Branch Deletion and Ref-Log Tombstones

**Status.** Proposed. Requires design review before implementation may begin. **Three verification
obligations must be discharged during that review before the design below may be adopted** — see §4.
**Split from.** DC-60, whose scope was amended 2026-07-30 to `list` and `create` only.
**Requirement.** `specs/prikk-app-requirements-v1.2.md` §6.5, the deletion half.
**Touches.** Ref-log record format, `refs/verify.rs` `classify_ref_state`, `refs/publication.rs`
`classify_state`, and a new `branch delete` CLI subcommand. **This is a format change** — the reason it is
not part of DC-60.

## Problem

DC-60 specified `branch delete` as "remove the pointer, retain the ref log." Implementation proved that
unshippable, with reproducible evidence
(`.git-exclude/reviewed/prikk-dc60-delete-divergence-ruling-v1.md`).

**Retaining the log produces "pointer absent, log present", and the shipped system classifies that state as
corruption.** That predates DC-60 by a long way — `seal_rejects_missing_pointer_with_ref_log_history`
tests it as corruption, and DC-13 goal 3 records "missing pointer plus non-empty log is *not* genesis."

The blast radius is repository-wide, and there is **no working case**:

| Log state after deletion | Classification | Effect |
|---|---|---|
| `record_count == 1` | `verify.rs:145` arm pushes a `blocking_issue` | `publication_issues` non-empty → `ensure_no_incomplete_publication` returns `LockConflict` |
| `record_count > 1` | falls to `verify.rs:161` `_ => Err(Integrity)` | propagated by `?` |

`ensure_no_incomplete_publication` (`refs.rs:31-42`) runs at the top of every mutation path
(`node_authoring.rs:183`, `active.rs:58`) and `verify_refs` walks **every** ref. So deleting one branch
blocks commits to all of them.

Separately, `publish`'s `classify_state` (`publication.rs:154-189`) has four arms, each requiring the live
pointer value to equal `expected` or `proposed`. An absent pointer with an advanced log matches none, so
recreating a deleted branch is not expressible in the current CAS model.

**The underlying problem is that one on-disk state means two different things.** "Pointer absent, log
present" can be a deliberate deletion or a lost pointer. They need different remedies. Conflating them
costs either corruption detection or deletion — which is why DC-60 could not resolve it by adjusting a
guard.

## Design

### 1. Deletion appends a typed tombstone to the ref log

`branch delete` removes the pointer **and appends a deletion record** to that ref's log. The state stops
being ambiguous: the log itself records why the pointer is gone.

Chosen over a separate marker file because it keeps one artifact with one integrity mechanism — the log's
existing `log_record_checksum` — rather than creating a second source of truth that can disagree with the
first. It also serves §6.5's "logs must support rollback detection and recovery" instead of straining it.

### 2. `verify` distinguishes deletion from corruption

Extend `classify_ref_state`:

- **pointer absent, log tip is a deletion record** → legitimate. Not an issue at all, or at most
  informational. Must **not** enter `publication_issues`, or `ensure_no_incomplete_publication` will block
  mutation exactly as it does today.
- **pointer absent, log tip is an ordinary record** → **unchanged.** Still corruption, still blocking, at
  every record count. No existing detection is weakened, including the `record_count == 1` arm.

That second bullet is the acceptance condition for the whole design. If the change cannot be made without
relaxing corruption detection for lost pointers, it is the wrong change.

### 3. `publish` gains one recognised transition

Recreating a deleted branch becomes an ordinary transition from a recognised state: pointer absent, log tip
is a tombstone. `classify_state` gains a fifth arm for exactly that, with the tombstone as predecessor.

**Specified here, not inferred at implementation time.** `publish` is the CAS core of every ref publication;
DC-60's implementation correctly refused to extend it unreviewed, and that refusal stands until this RFC's
design review settles the arm's exact shape.

### 4. Verification obligations — discharge these before adopting §1-§3

*The design above is a recommendation from the DC-60 ruling. It has not been checked against the codebase.
Three of this program's recent RFCs failed by specifying work whose interaction with existing code was
unverified; this section exists so that check happens at design review rather than at implementation.*

| Must verify | Why it could sink the design |
|---|---|
| A new ref-log record type is expressible **without breaking format-1 / format-2 compatibility** | The ref log has a versioned record format. If a tombstone is unreadable by an older reader, deletion becomes a format break, and DC-40's transition rules apply |
| `replay_log` and `log_position` handle a tombstone tip | `log_position` feeds `classify_state`. If it cannot represent a tombstone tip, §3 has nothing to match on |
| What `doctor` does with a tombstoned ref | `doctor` offers repair. It must not "repair" a deliberate deletion by resurrecting the pointer |

If any obligation fails, the alternatives are the separate-marker approach DC-60's implementer proposed, or
accepting that deletion cannot retain the log and reopening the NFR-REL-01 question with the owner.

## Non-goals

- No `branch switch` and no current-branch pointer — still deferred, and still better designed after the
  multi-patch queuing decision.
- No tagging (§6.6), no remote branches (§6.11).
- **No garbage collection.** Deletion reclaims nothing; NFR-REL-02 owns that. The command must say so.
- No change to `branch list` or `branch create`, which shipped under amended DC-60.
- No relaxation of corruption detection for pointer loss — see §2.

## Risks

**Weakening corruption detection while trying to permit deletion.** The central risk. §2's second bullet is
the guard, and the design review must confirm the implementation preserves the existing behaviour for
non-tombstone cases rather than testing only the new path.

**A tombstone that older readers misread.** Covered by §4's first obligation. Worst case is silent
misclassification by an older binary, which is worse than a clean failure.

**`doctor` resurrecting a deleted branch.** Covered by §4's third obligation.

**Scope creep into `publish`.** The fifth arm is one transition. Any broader change to the CAS model is a
separate increment with its own review.

## Acceptance criteria

1. Deletion removes the pointer and appends a tombstone; the log, its prior records, and all objects are
   demonstrably retained.
2. `verify` passes cleanly on a repository with a tombstoned ref — no blocking issue, no `Integrity` error,
   at **both** `record_count == 1` and `record_count > 1`.
3. **Mutation is unaffected**: a commit to an unrelated ref succeeds after a deletion. This is the
   regression DC-60 hit, and it must be tested by committing to a ref the deletion never touched.
4. Corruption detection is **unchanged** for pointer-absent-with-ordinary-tip, at every record count —
   tested by simulating pointer loss as `seal_rejects_missing_pointer_with_ref_log_history` does, and
   confirming the existing classification and blocking behaviour still hold.
5. Recreating a tombstoned branch publishes at `last_seq + 1` with the tombstone as predecessor; `verify`
   passes afterward.
6. `doctor` behaves per §4's resolved answer on a tombstoned ref, tested.
7. Format compatibility per §4's resolved answer, evidenced.
8. `branch delete` fails closed on a missing branch, on a branch owning a non-empty active WAL (reusing
   `require_active_ref_for_non_empty_wal` and citing DC-13 goal 4), and on the last remaining branch.
9. Output states that no objects were reclaimed.
10. No identity artifact changes: `vectors/snapshot.txt`, `vectors/hard.rs`,
    `state_root/tests/vectors.rs`, `text_span/vectors.rs` all byte-identical.
11. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

All eleven are verifiable from the repository. Criteria 3 and 4 are the ones that matter most: 3 is the
defect that stopped DC-60, and 4 is the property most easily lost while fixing it.
