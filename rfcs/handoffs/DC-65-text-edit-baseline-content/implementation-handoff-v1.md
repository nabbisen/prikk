# DC-65 Text-Edit Baseline Content - Handoff

**Cleared to start.** Accepted by the project owner on 2026-07-31, at
`rfcs/accepted/DC-65-TEXT-EDIT-BASELINE-CONTENT.md`. **Top of the development lane**, ahead of all
remaining performance work.
**Authored by** the architect.
**Found by** you, incidentally, while building DC-59's Axis C — and reported instead of fixed in place,
which was the right call.
**Size:** unknown until §1 is answered, and that is deliberate. It could be a few lines or a format
question.
**Touches:** `worktree_patch/node_authoring.rs`'s `plan_edit_text` at minimum; possibly `checkout`,
`patch_algebra`, and the invariant both halves of the codebase are supposed to share.

## The defect, confirmed independently

I built `6064da6` — before any DC-64 code — and ran create → seal → edit → seal → edit:

```
error: integrity error: baseline content Blob 72cc9ea9…d6 is missing
```

**Byte-identical to your transcript, including the blob id.** Your analysis is right in every particular:
`write_content_blob` has exactly two call sites (`:396` create, `:569` `ReplaceBinary`), neither on the
`EditText` path, while `plan_edit_text:537` reads `base.blob_id` as a stored `Blob`.

**Editing a file twice is the primary workflow of a version control system.** Treat this with the priority
that implies.

## 1. Answer these before designing a fix. All four are blocking

Your candidate fix — materialize through the same replay path `apply_edit_text` uses — is plausible and may
well be right. **Plausible is exactly what DC-56's and DC-64's first drafts were**, and both cost real work
before the premise was checked. So:

| Question | Why it blocks |
|---|---|
| Does `checkout`/materialization have the same defect? | It reconstructs from the same chain. If yes, the fix is a shared primitive, not a patch to one call site |
| Is `ReplaceBinary` affected? | It writes a blob per edit, so probably not — "probably" is what this program keeps getting wrong |
| Do `merge_evidence`/`patch_algebra` read baseline content the same way? | Same class of read against the same computed ids |
| **Is a node's `blob_id` supposed to name a stored object, or not?** | **The real question.** Authoring and replay currently disagree, and the fix follows from which is correct |

**That last row decides the increment.** Either every derived content identity must be materialized as a
stored `Blob` — making authoring symmetric with replay, at a storage cost — or `blob_id` is legitimately a
content identity that need not exist as an object, making `plan_edit_text`'s direct read the bug.

**Do not pick by convenience or by whichever is a smaller diff.** Whichever is correct, bring the other side
into line and state the invariant where both halves can see it. If the answer implies a format change, that
is a finding to report, not scope to absorb.

## 2. The coverage gap is the more important half

561 store tests, 80 object tests, a crash matrix, a fuzz campaign, property tests, and the whole of DC-41's
integrity-evidence campaign all passed over a repository that cannot edit a file twice. **The suite is
weighted toward adversarial and structural cases and away from ordinary use.**

Fixing this one bug without addressing that leaves the next one equally available. Criterion 5 asks you to
state what class of ordinary-workflow coverage is missing and what you added. Answer it as a finding, not a
checkbox — you found this one by accident, and that is the point.

Test the fix at **N ≥ 3 sealed edits**, not 2. Two is the boundary that was missed; three shows the chain
holds.

## 3. Traps

- **Fixing `plan_edit_text` alone because it is where the error surfaced**, before §1's fourth row is
  answered. The error site and the defect are not necessarily the same place.
- **Choosing the invariant by diff size.**
- **Moving an existing object's bytes or ObjectId.** Criterion 6. If the chosen invariant requires it,
  account for every movement individually and say so before you commit.
- **Treating criterion 5 as paperwork.**
- **Folding in DC-64's residual performance cost.** Tracked separately, unowned, explicitly a non-goal here.

## 4. Definition of done

§1's four questions answered and reported before a fix was designed; the `blob_id` invariant stated
explicitly with both authoring and replay conforming; editing one text file across **N ≥ 3** sealed commits
working and tested; the equivalent `ReplaceBinary` and checkout coverage added or its absence justified from
§1's answers; a stated coverage finding per §2; identity unaffected or every movement accounted for; full
gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9 with test counts before and after.

## 5. Submit with

The diff; §1's four answers as a document; the invariant statement; the N ≥ 3 test; the coverage finding;
test counts per touched crate before and after; an explicit statement of what did not change; and the full
gate set run on a **clean checkout of the commit**, stated as such.

## 6. Standing request

You found this by noticing that a benchmark axis failed for a reason that had nothing to do with the
benchmark, and you traced it rather than routing around it silently — you did route around it, but you
said so in a doc comment and reported the cause. **That is the single most valuable thing anyone has done
in this program.** If something here contradicts what the code actually does, stop and report it.
