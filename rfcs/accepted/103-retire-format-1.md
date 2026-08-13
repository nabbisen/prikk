# RFC (accepted) - 103 Retire Format-1

**Status.** **ACCEPTED by the project owner 2026-08-13**, on the owner's direction: design *"clean,
simple as possible, reasonably functional and sophisticated, without concern about migration."*
**Risk accepted explicitly, same date:** *"We are in early development stage. The risk is accepted."*
§10's cost — that any format-1 repository in the wild becomes unopenable by every future version — is
therefore an accepted risk, not an open one. **Acceptance clears §8's prerequisites, not the
implementation.**
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The format-1/format-2 duality surfacing as a complication in four consecutive DC-95
rounds, and the owner's ruling that migration from an older prikk need not be preserved.
**Target.** Owner's call. **Not a prerequisite to RFC 102** — see §5; they share a direction, not a
dependency.

## 1. The decision

**Format-1 is not supported. A format-1 repository is rejected at open, with a message that says so.**

Not read-only support, not automatic upgrade, not a compatibility shim. Each of those keeps every
dual-path branch alive, which is the cost this RFC exists to remove.

## 2. What this removes

Measured, not estimated: **22 `LegacyV1` sites across 13 files**, plus five pieces of machinery that
exist only to serve format-1:

- `active.rs::finish_legacy_active_publication_cleanup` and `authorize_legacy_active_cleanup`
- `wal.rs::truncate_empty_for_legacy_recovery`
- `verify.rs::legacy_state_roots_unverifiable` — a field, its predicate, and its assignment
- ~~`test_support.rs::legacy_rollback_marker_signature`~~ — **REMOVED 2026-08-13, misattributed.**
  It backs `rollback_verify.rs:210`'s `key_id == LEGACY_ROLLBACK_MARKER_KEY_ID` check, which runs
  **unconditionally** — a hardcoded placeholder key id, not a format condition. It sits three lines above
  the check that genuinely is format-1-only, and DC-95's own module doc already keeps the two rows apart.

**Added the same date, found by §8's independent enumeration and missed by the literal-token grep:**

- **`PublicationState::LegacyLogLeading`** — enum variant plus six format-1-gated usage sites
  (`refs/publication.rs`), mirrored in fixtures.
- ~~**The vestigial format-1 missing-pointer reconstruction subsystem**~~ — **WRONG, and withdrawn
  2026-08-13** after Increment A's stop-and-report. I classified this as one unit of format-1 machinery.
  **It is two things, and neither is format-1-gated:**

  - **`RefRecoveryCandidate` / `recoverable_missing_ref` is live, general, format-2 machinery.** It
    carries no `RepositoryFormat` reference and is called from `branch.rs:145-154`'s `run_create` to fail
    closed on a surviving ref log with no live pointer (DC-61). **Removing it as this RFC originally
    instructed would have deleted a live corruption check from `branch create`.** Its own doc comment
    says "the format-1 log is valid" — **the comment is what is wrong, not the machinery.** *Ruled: keep
    it, fix the comment, recategorise as ordinary format-2 code.*
  - **`RefRecoveryRepair` / `reconstruct_missing_ref_from_log` / `DoctorRepairOptions::reconstruct_main_ref`
    is a permanently-refused placeholder**, refused for *any* format. Its message merely says "format-1."
    It is wired to a live, documented CLI flag (`prikk doctor --repair-main-ref`). *Ruled: out of scope
    for this RFC.* Deleting user-facing surface because a string mentions format-1 is a different
    decision from retiring format-1, and it does not ride along on this one.
- **`signature_diagnostics.rs` — flagged, not removed.** Its logic stays and is load-bearing; it carries
  **no `RepositoryFormat` gate at all** and is format-1-only in *practice*, via the same upstream gating
  DC-95 round 11 established. What goes stale is its doc comment and issue-message framing, which label
  it format-1 compatibility machinery. **Correct the framing; keep the code.**

And three checks whose only reason to exist is the duality:

| Check | DC-95 Stage 1 classification | Effect of this RFC |
|---|---|---|
| `PRIKK-VERIFY-REF-LEGACY-LOG-LEADS` | **Downstream-redundant** (round 10) | Deleted. Its format-2 sibling already catches the same defect — that is what round 10 proved |
| `validate_read_schema`'s `LegacyV1` branch | *(not a classified row of its own)* | Deleted — the format it branches on is gone. **Its format-2 branch stays and remains load-bearing** (inventory row: strict-signature-shape, round 4). Do not confuse the two |
| Rollback WAL **wrong signature length** | **Load-bearing** via non-blocking sibling (round 11) | **Becomes provably unreachable.** Round 11 established it is reachable end-to-end *only* under format-1: under format-2, `Wal::replay()`'s own `validate_read_schema` rejects a malformed-shape signature before this check runs. **Keep it, untested, with the argument recorded** — round 6's ruling on unreachable checks. **Do not delete it** |
| `legacy_state_roots_unverifiable` | Precondition fact, not a stage output | Deleted — it can only ever be false |

**Every one of those three is documented, classified, and probed** by DC-95 Stage 1. That is why this
RFC is cheap to review: the coverage question was answered before the removal was proposed.

## 3. What this does *not* remove — and the distinction matters

**The `created_at == 0` check survives, and stops being conditional.**

`refs/verify.rs:46-52` rejects a `CurrentV2` repository containing any ref-log record with
`created_at != 0`. With format-1 retired, that is no longer *"a format-2 repository contaminated by
format-1 records"* — it is simply **malformed data**, and the check becomes an unconditional invariant
rather than a format-conditional one.

**It gets simpler; it does not get weaker.** Anyone reading §2's list and assuming "legacy checks go"
would delete it, and that would remove real malformed-data detection. Stage 1 classified it
**load-bearing**.

Similarly out of scope: DC-40's state-merkle format *design* stays. What goes is the machinery for
tolerating repositories that predate it.

## 4. The rejection contract

A format-1 repository must fail at open with a message that is **actionable, not merely accurate**:

- name the detected format and the required one,
- name the last prikk version that supported format-1,
- state the remedy — export via bundle from that version, import here.

**A bare `malformed persisted data` is not acceptable.** The one user this affects is a user upgrading,
and they will hit it exactly once, with no context. Detection must be by the `FORMAT` file's own
content, not by a downstream decode failure.

## 5. Consequence for RFC 102 — asked, and answered

RFC 102's constraint 6 required *"a format migration must exist for repositories already written in the
current format"* — the single largest cost item in a container-based storage redesign.

This RFC originally flagged the question and declined to take it: dropping migration for a *retired*
format is a different decision from dropping it for the *current* one, and the second is much bigger.

**Answered 2026-08-13. The owner's direction extends to both:** *"We are in early development stage. The
risk is accepted."* **RFC 102's constraint 6 and its paired acceptance criterion 5 are withdrawn**, and
its §9 cost no longer carries a migration. That is recorded in RFC 102 itself, marked in place.

**Consequence for sequencing: RFC 103 is no longer a prerequisite to RFC 102 in any strong sense.**
Neither blocks the other. They share a direction, not a dependency.

## 6. Consequence for DC-95's classified inventory

**Corrected 2026-08-13 on re-check.** An earlier draft of §2 named `validate_read_schema`'s branch as the
round-11 format-1-only finding. It is not: round 11's finding is about the **rollback wrong-signature-length**
check, and `validate_read_schema`'s own strict-shape row is round 4 and not format-1-only. Followed
literally, that error would have deleted a load-bearing check and missed a fourth affected row entirely.

Three of the 41 classified rows change status — `LEGACY-LOG-LEADS` (deleted), rollback
wrong-signature-length (**load-bearing → provably unreachable, kept**), and
`legacy_state_roots_unverifiable` (deleted). **The inventory must be updated in the same increment,
not left to drift** — it is the map a future reader consults, and DC-95 Stage 1 spent twelve rounds
making it trustworthy.

## 7. Non-goals

- **Read-only support for format-1.** Rejected in §1; it preserves the duality.
- **Automatic in-place upgrade.** That is a migration tool, which is what the direction removes.
- **Changing format-2 itself.** This RFC deletes the alternative, not the survivor.

## 8. Blocking prerequisites

1. **Enumerate every format-1 site independently** — the 22 measured here are a starting figure from one
   grep, not a derived set. Four consecutive investigations this month found the architect's counts
   narrower than the code.
2. **Confirm each of §2's three checks is genuinely format-1-only**, from Stage 1's classification and
   the code, not from this table.
3. **Establish what a format-1 repository looks like at open today** — which code path first notices, and
   what it currently reports. §4's contract cannot be written against a guess.

## 9. Acceptance criteria

1. **No `RepositoryFormat::LegacyV1` remains in production code.** The enum variant itself may stay only
   if detection requires naming the rejected format. **Amended 2026-08-13: this criterion is not
   sufficient on its own.** §8's enumeration found format-1-specific identifiers and dead compatibility
   stubs carrying no `LegacyV1` token — `PublicationState::LegacyLogLeading` and the reconstruction
   subsystem — which a token-based check passes over entirely. **The criterion is "no format-1-specific
   machinery remains," and the token is one instrument for finding it, not the definition.**
2. **A format-1 repository is rejected at open with §4's message**, proven by a test using a real
   format-1 fixture, not a hand-built one.
3. **The `created_at == 0` check still fires**, unconditionally — proven by the DC-95 method: disable it,
   observe the specific failure, restore.
4. **DC-95's classified inventory updated** in the same increment.
5. Green three-platform CI.

## 10. The risk, stated rather than absorbed

**Prikk has shipped releases. Format-1 repositories may exist in the wild, and this RFC makes them
unopenable by any future version.** The remedy — bundle export from an older release — requires the user
to still have that release, or to fetch it.

**The owner has directed that migration need not be preserved, and this is that decision's cost.** It is
recorded here rather than left implicit, per the register's rule that significant risk is never silently
accepted. If the owner wants it reduced, the cheapest mitigation is a **detection-only** stub retained
indefinitely: enough format-1 knowledge to recognise and explain, never enough to read. That is §4's
contract and it is already the minimum this RFC requires.
