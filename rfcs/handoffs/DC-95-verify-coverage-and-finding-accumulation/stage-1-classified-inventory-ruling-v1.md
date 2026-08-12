# DC-95 Stage 1 — Classified Inventory Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-95-stage1-classified-inventory-v1.md`.

**Accepted as the living document.** It is what §4 of the round 7 review asked for and it is better than
asked for — it raises two open questions rather than resolving them silently. Both are ruled below.

## 1. §0's granularity question: both counts are right, and they measure different things

The running "N of 36" counted sub-cases (8 shape arms as 8). The inventory's own rows count
`validate_block_v2_shape` once. Neither is wrong; they are answers to different questions:

- **Classification granularity = what can be independently absent.** A classification says *"if this
  check vanished, would a bad repository verify clean?"* You cannot make one match arm of
  `validate_block_v2_shape` vanish in practice — the probe disables the function. **One check, one
  classification.**
- **Coverage granularity = what needs its own fixture.** A `Root`-with-parent fixture does not exercise
  `Merge`-without-mainline. **Eight defect shapes, eight fixtures.**

**Ruling:**

1. **"36" refers to the original inventory's row granularity.** That is where the number came from —
   the prerequisite report's §3.2 counted *36 of 44 non-"Yes" rows* matching the rule, over a table
   whose rows are checks, not arms. Verified: line 55 of that report lists *"Block format-2 shape
   validation (8 arms)"* as a single row.
2. **The classified inventory keeps row granularity**, as it already does. Correct.
3. **The per-round "N of 36" figures are withdrawn, not reconciled.** They counted fixtures against a
   denominator of checks, so they overstated progress. Do not restate them; state the row count from
   here.
4. **Show both.** Each row carries its sub-case count where it has one — the classification unit and
   the fixture count are both useful, and conflating them is what produced the drift.

**The honest consequence, and it should be said plainly: more work remains than the round reports
implied.** Roughly 16–17 rows rather than the ~13 that "23 of 36" suggested. That is a correction to the
project's own progress reporting, and I would rather have it now than at Stage 1's end.

## 2. §4's open question: the `ActiveWal` exclusion is confirmed, not assumed

They flagged that round 4's generalisation across `SignatureEnvelopeSource` variants was asserted for
`RefLog` and not re-checked for `ActiveWal`. **Correct to flag it. Now checked, and it holds — for a
stronger reason than the generalisation offered.**

`signature_envelope_issues` is a single `Vec` on `RepositoryVerification` (`verify.rs:84`), populated
from every source into that one field (`:278`, `:281`, `:289`). **No `has_*` predicate reads that field
at all.** So the exclusion is not "true for each source variant in turn" — the *source is irrelevant*,
because the vector itself backs no blocking predicate. It cannot vary by variant.

**Ruled: all `SignatureEnvelopeSource` rows are Excluded**, including `ActiveWal`. §4 drops to 8 rows.

**And the standing caveat travels with it**: this rests on `signature_envelope_issues` being non-blocking,
which is the registered open question from round 4 (*"should the MALFORMED variant be wired into a
blocking predicate?"*). If that is ever answered yes, every one of these exclusions reopens. The
inventory should carry that dependency on the rows themselves, not only in a review.

## 3. The observation in §6 that deserves to be pulled out

`PRIKK-TRUST-POLICY-INVALID` is listed as *"observed as incidental baseline noise in nearly every fixture
this whole project… but never itself the subject of a dedicated end-to-end test."*

**That is the check which confounded rounds 1 through 5, and it is itself uncovered.** The thing that
made every early probe unanswerable is a check nobody has deliberately tested. It should be an early row
in a coming round rather than left near the end of the list — it is well understood by now, its fixture
is trivial (a repository with no trust policy, which every earlier fixture accidentally built), and
closing it converts a long-running accident into a deliberate control.

## 4. Two smaller confirmations

- **Row 50's relabel** — the catch-all divergence fallback moved from "Partial" to "Not yet covered"
  because the test that appeared to reach it actually trips an earlier `scan.rs` chain-divergence `Err`.
  That is the inventory correcting the prerequisite report, which is exactly what a living document
  should do.
- **The topological-cycle row** correctly notes that DC-92's unit-level substitute already exists and
  that no end-to-end version is needed or attempted. Consistent with round 6's ruling on the
  duplicate-identity rows.

## 5. Standing

- **Inventory: accepted**, with §1's ruling applied to its counting and §2's to its §4.
- **Restate the remaining-row total once** under the ruled granularity, so there is one number going
  forward.
- **Round 8** next. `PRIKK-TRUST-POLICY-INVALID` is a good candidate to take early, per §3.
- The inventory belongs in the code's documentation before Stage 1 closes, per the round 7 ruling — it
  is currently a review-request document, which is the right place to draft it and the wrong place to
  leave it.
