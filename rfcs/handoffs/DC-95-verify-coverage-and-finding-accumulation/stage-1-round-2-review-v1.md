# DC-95 Stage 1, Round 2 — Review v1

**Reviewing:** `b4baf3d` on `dc-95-verify-coverage-and-finding-accumulation`.

**Accepted, no conditions.** One trivial note (§4).

**The finding is worth more than the three tests, and it changes how the remaining 33 should be
understood — §3.**

## 1. Verified, per check, independently

I probed each check separately rather than trusting uniformity, which is what their own round was about:

**Snapshot-blob check disabled:**
```
case "missing-snapshot-blob": expected verify_repository to reject a missing reference
```
`verify_repository` returned `Ok`. **A genuine clean pass** — this row demonstrates Stage 1's rule
directly.

**Parent-block check disabled:**
```
case "missing-parent": expected error containing "references missing parent block",
got: integrity error: format-2 parent Block 8cbe59ac… is missing
```
Caught downstream by `validate_v2_lineage`'s own read, with a different message. **Downstream-redundant,
exactly as reported.**

Their classification of all three is accurate. The old `verify_repository_detects_block_with_missing_patch`
is genuinely removed rather than left passing trivially alongside. Gates clean at 615, net unchanged.

**They also corrected their own arithmetic** — round 1's summary said 7 remaining, the inventory says 8 —
rather than carrying the slip forward. Small, and the right instinct.

## 2. What they got right that I would not have asked for

I would have accepted three fixtures with arbitrary roots and a note that correct roots were impossible.
**Instead they probed each check to find out what disabling it actually does**, and discovered the three
are not alike. That is the round-1 standard applied with judgement rather than mechanically: the standard
was *confirm the repository genuinely verifies clean*, and when two rows could not meet it, they
established **why** and reported it rather than either forcing a fixture or quietly lowering the bar.

Their distinction is the important sentence in the report: **"can this fixture have a replay-correct
root" and "does this check matter under Stage 1's rule" are two different questions** that happened to
align in round 1 and do not always.

## 3. The consequence for the remaining 33, which neither of us has stated

**The §3.2 inventory's "36 rule-matching checks" was derived by reasoning about each check's role. Round 2
shows that reasoning can be wrong — and that the only way to know is to disable the check and look.**

Two of these three were classified as rule-matching in the inventory. Empirically, they are not: something
downstream catches the same defect. **So 36 is an upper bound, not a count**, and Stage 1 is in effect an
audit of its own inventory as well as a test-writing exercise.

**Required going forward — cheap, and the more durable half of the work:** record the probe result for
every check as it is covered. Load-bearing (disabling it lets the repository verify clean) or
downstream-redundant (something else catches it, with which message and from where). A future reader
learns more from *"this check is redundant with `validate_v2_lineage`'s read for format-2"* than from the
test itself.

**And record it as a fact about today's code, not a property to rely on.** If `validate_v2_lineage` ever
stops reading parents, `missing-parent` silently becomes load-bearing again. The classification is
time-bound; the tests are not, which is why both rows are still worth having.

**On keeping the redundant rows:** yes. Their argument — that "which check said so" is real diagnostic
value for an operator reading a failure — is right, and a regression guard on the specific message is
worth its line count. But it should be *labelled* as that rather than counted as a rule-matching control,
so the record does not overstate what Stage 1 has proved.

## 4. One trivial note, not a condition

`verify/tests.rs:132`'s doc comment on `verify_repository_detects_block_with_state_root_mismatch` still
cites `verify_repository_detects_block_with_missing_patch` as a construction precedent — the test this
round removed. A dangling reference of exactly the kind I left in `rfcs/README.md` yesterday. **Fix it in
round 3**, since that file is being edited anyway; not worth a round-trip on its own.

## 5. Standing

- **Round 2: accepted.** 11 of 36 covered.
- **Round 3** next — the remaining 5 in the `verify/objects.rs` cluster, each needing its own
  construction technique. Splitting them out rather than bundling all 8 into one round was the right
  call.
- Green three-platform CI before any merge.
