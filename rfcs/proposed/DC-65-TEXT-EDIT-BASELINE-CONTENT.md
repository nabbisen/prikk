# RFC (proposed) - DC-65 Text-Edit Baseline Content Availability

**Status.** **Proposed 2026-07-31.** Opened on a confirmed, reproduced, severe correctness defect.
**Awaits owner acceptance.** Recommended as the **top of the development lane**, ahead of all performance
work.
**Authored by** the architect.
**Found by** the DC-64 implementation, incidentally, while building DC-59's Axis C. Reported rather than
fixed in place — correctly.
**Independently reproduced** by the architect on `6064da6`, before any DC-64 code.
**Requirement.** Core patch-authoring correctness. No NFR names this because nothing anticipated it.

## 1. The defect

**Editing the same text file in two separate sealed commits fails.**

```
init; write "hello world"; commit; seal
write "hello world, edit1";  commit; seal
write "hello world, edit2";  commit
  error: integrity error: baseline content Blob 72cc9ea9…d6 is missing
```

Reproduced by the architect on the pre-DC-64 parent, byte-identical to the reporter's transcript including
the blob id. **This is not a DC-64 regression** — it is long-standing.

## 2. Root cause, confirmed

- `write_content_blob` (`node_authoring.rs:701`) has **exactly two call sites**: `:396` (fresh create) and
  `:569` (`ReplaceBinary`). **Neither is on the `EditText` path.**
- `plan_edit_text:537` calls `read_file_blob_bytes(object_store, base.blob_id)`, requiring the baseline
  node's `blob_id` to name a **stored** `Blob`.
- `EditText` records a diff; `apply_edit_text` (`lifecycle_cache/replay/effect.rs`) reconstructs content by
  splicing the original blob plus the diff chain, so replay never needs the derived blob to exist.

So a node's `blob_id` after an edit is a **computed identity that was never durably written**. The first
edit after a create works because its baseline is the real create-time blob. Every edit after that fails.

## 3. Why this matters more than the performance program

Edit, commit, edit, commit is **the primary workflow of a version control system**. Prikk cannot do it.
NFR-PERF-01 governs how fast a working path runs; this governs whether the path works at all.

**And the coverage gap is the more important finding.** The suite creates-then-edits, and never edits the
same text file across two sealed commits. 561 store tests, 80 object tests, a crash matrix, a fuzz campaign,
property tests, and an entire integrity-evidence increment (DC-41) all passed against a repository that
cannot perform its core operation twice. **The gates are weighted toward adversarial and structural cases
and away from ordinary use.** This increment must close both.

## 4. What must be established before a fix is designed

**Mandatory, and blocking.** DC-56's and DC-64's first drafts were both wrong because a plausible mechanism
was adopted before its premise was checked. The obvious fix here — have `plan_edit_text` materialize through
the same replay path `apply_edit_text` uses — is exactly that shape of plausible.

| Question | Why it must be answered first |
|---|---|
| Does `checkout` / materialization have the same defect? | It reconstructs content from the same chain. If it does, the fix is a shared primitive, not a patch to one call site |
| Is `ReplaceBinary` affected? | It *does* write a blob per edit, so probably not — but "probably" is what this program keeps getting wrong |
| Does `merge_evidence` / `patch_algebra` read baseline content the same way? | Same class of read against the same computed ids |
| What is the intended invariant — is a node's `blob_id` supposed to name a stored object, or not? | The two halves of the codebase currently disagree. **The fix follows from which answer is correct**, and that is a design question, not an implementation detail |

**The last row is the real question.** Either every derived content identity must be materialized as a
stored `Blob` (making authoring symmetric with replay, at a storage cost), or `blob_id` is legitimately a
content identity that need not exist as an object (making `plan_edit_text`'s direct read the bug). **Do not
pick by convenience.** Whichever is chosen, the other side must be brought into line and the invariant
stated where both halves can see it.

## 5. Acceptance criteria

1. §4's four questions answered and reported **before** a fix is designed.
2. The intended invariant for a node's `blob_id` is **stated explicitly** and both authoring and replay
   conform to it.
3. Edit-the-same-text-file-across-N-sealed-commits works, tested at **N ≥ 3** — two is the boundary that
   was missed.
4. The equivalent test exists for `ReplaceBinary` and for whatever §4 finds about checkout, or their
   absence is justified from §4's answers.
5. **A coverage finding**: state what class of ordinary-workflow test the suite lacks and what was added.
   Closing this one bug without that leaves the next one equally available.
6. Identity is unaffected — no existing object's bytes or ObjectId move — or every movement is accounted
   for individually.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

## 6. Non-goals

- Performance. DC-64's residual cost is tracked separately and does not belong here.
- Redesigning the patch format. If the answer to §4's last row implies a format change, that is a finding
  to report, not scope to absorb.
