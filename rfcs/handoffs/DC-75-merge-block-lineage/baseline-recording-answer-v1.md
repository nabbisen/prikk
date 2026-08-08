# DC-75 — §3 Answered: Baseline Recording and Verification

**Handoff followed:** `implementation-handoff-v1-addendum-2.md` §3.
**Governing RFC:** `rfcs/accepted/DC-75-MERGE-BLOCK-LINEAGE.md`

Per addendum-2 §3 ("answer this in §4's discipline... measure it; do not take my lean as the answer"),
this reports before any design or production code. No source files are changed by this document.

## 1. The measurement

§3's fork turns on cost: is recomputing a merge base cheap enough to do unconditionally during
`verify`, or does it need gating? The candidate walk (`merge_evidence.rs`'s `candidate_blocks`) is pure
parent-pointer traversal plus patch decode — no state-root replay — and is structurally the same shape
a merge-base/LCA walk would be. `prikk merge-plan` exercises exactly this walk, so it was used as the
proxy, timed end-to-end (subprocess included) via `Command::current_dir`, per the project's live-probing
convention.

| N (sealed blocks walked) | `merge-plan` time | ratio vs. previous |
|---:|---:|---:|
| 5 | 1.329 ms | — |
| 20 | 1.794 ms | 1.35× |
| 80 | 5.834 ms | 3.25× |
| 160 | 11.342 ms | 1.94× |
| 320 | 21.296 ms | 1.88× |

From N=80 onward, doubling N multiplies time by ~1.9× — the signature of linear growth, not the
accelerating 2.08×→6.39× cubic signature §0 measured for `verify` itself. The N=20→80 point (4× N,
3.25× time) is consistent with a small fixed per-invocation floor (subprocess start, ~1.3 ms at N=5)
being amortized away as N grows, not with superlinear walk cost.

**Directly comparable to §0's numbers, same N:** at N=160, this walk costs 11.3 ms against `verify`'s
34,155 ms — roughly 3,000× cheaper, on the same tree, same day. The cubic defect and the ancestry walk
are different code paths with different costs; nothing here re-measures or depends on §0's defect.

## 2. Why this generalizes to the actual merge-base computation, not just this proxy

`candidate_blocks` walks one single-parent chain with a cycle guard (`visited: BTreeSet`). A true
merge-base/LCA walk over two chains is the same primitive run at most twice, stopping at the first
block already seen — bounded by the shorter distance to the common ancestor, not by total history
length, and deduplicated by construction. It does not compound: a merge nested inside another merge's
ancestry is visited once, not re-walked per enclosing merge, because the visited-set is per-invocation
and the walk terminates the moment it hits a block the other side already reached. This is the
structural reason to expect linearity, and §1's table is the measurement confirming it rather than
asserting it.

This differs in kind from §1.a's demoted cost argument. §1.a rested on `verify`'s cubic cost being a
*defect* — fixable, and therefore not a reason to pick a weaker design. Here, the cheapness is a
*structural property* of a parent-pointer walk with deduplication, not a bug being worked around. There
is nothing to fix that would change this number's shape.

## 3. Answer: both readings, as leaned — confirmed, not assumed

**(b) Record the baseline.** One more optional field on `Merge` blocks, same `Option<ObjectId>`-at-a-tag
pattern as §2's ruled mainline-parent field (`snapshot_blob_ref`'s established shape) — `None` for every
existing `Root`/`Normal` block, unchanged canonical bytes. History then states what the sealer actually
checked against, which this project states everywhere else it has sealed evidence to state.

**(a) The verifier re-derives it — unconditionally, in ordinary `verify`, not a separate "deep" mode.**
The lean's caveat ("record it as a claim, don't just trust it") is affordable precisely because §1's
measurement shows the re-derivation costs single-digit-to-tens-of-milliseconds at history sizes where
the existing cubic defect already costs tens of seconds. Gating this behind an opt-in mode would be
solving a cost problem that does not exist here — the thing that's expensive is the unrelated §0 defect,
not this walk. If the recorded baseline and the derived merge-base disagree, `verify` reports it as a
distinct integrity finding on that block (a declared baseline that is not the true merge base) — the
same "state what was checked, then check it" posture as every other sealed claim in the format, and
exactly the gap `verify`'s design deliberately leaves nowhere else.

## 4. What I did not do

No production code changed. No test changed. The temporary probe (`zz_dc75_baseline_probe.rs`) is
removed before this commit, per its own doc comment — its numbers are captured in §1's table. Did not
re-measure §0's defect; cited it only for scale comparison.

## Request

Report only, per §4's discipline. This is the last blocking question addendum-2 raised — per its §5,
once this is answered and reported, §5 implementation is cleared.
