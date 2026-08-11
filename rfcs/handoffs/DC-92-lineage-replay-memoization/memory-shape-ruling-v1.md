# DC-92 — Memory Shape Ruling, and the Bounding Round

**Reviewing:** `.git-exclude/review-request/prikk-dc-92-memory-shape-findings-v1.md`, commit `c0f3734`.

**The measurement is accepted and the condition is discharged as a measurement. The answer is that the
bound is warranted: DC-92 does not merge as it stands.** §4 is the next round; it is the same increment
continuing, not a new one.

## 1. Verified

`verify_objects` does iterate in encoded-filename (ObjectId) order — `verify_prefix_dir` sorts entries
by bytes. Their obstacle to lead (a) is real, not an excuse.

The harness measures peak `VmHWM` by `.spawn()` plus `/proc/<pid>/status` polling, DC-62's technique, no
new dependency; the axes are what the report says (`MEMORY_DEPTH_TREE_SIZE = 1_000`,
`MEMORY_DEPTH_VALUES = [5, 40, 100, 160]`, `MEMORY_TREE_DEPTH = 160`).

Their linear fit holds: `-545 + 418.7·N` predicts 16,203 KB at N=40 against 16,204 measured, and 66,447
at N=160 against 66,452. The tree axis converges on 10x per 10x. **O(N × tree_size), bilinear** — the
shape the mechanism predicts.

**I did not re-run the 24-minute measurement.** I verified the method and the internal consistency of
the numbers instead, and I am recording that rather than implying I reproduced it.

## 2. The caveat is the most valuable line in the report

**Churn never edits text.** Every generation deletes a file and creates a new one, so no `EditText`
occurs, so `TextCache` stayed empty in every trial. **The measured curve excludes the materialized
file-content term entirely** — the half that motivated `TextCache` existing at all, and the half most
likely to dominate on a repository with real edit history.

Flagging that as unmeasured rather than estimating it is exactly right, and it means 599 MB at
N=160 / 10,000 files is a **floor**, not the figure.

## 3. Ruling: the bound is warranted

599 MB measured at a modest corner; the fitted line puts a still-ordinary N=10,000 north of 37 GB; and
an unmeasured content term sits on top of both. This is the trade I said in the implementation review I
would not accept — *before this change `verify` was slow; after it, it may not complete* — and the
measurement moves it from "unbounded by construction" to "unbounded, and here is the slope."

**So: DC-92 does not merge as it stands.** The O(N³) → O(N) time fix is real, independently reproduced,
and must not be lost — this is a continuation, not a retreat. DC-92's acceptance criteria are amended
to require a bounded memo, because "verify is fast" is not delivered by something that may exhaust
memory instead.

## 4. The next round: evaluate the cause, not only the symptom

**4.1 — Lead (b) is dead as a sufficient answer, and their own analysis is what kills it.** Dropping
`TextCache` from the memo caps only the content half — and the depth-axis numbers *already exclude*
`TextCache`, so lead (b) would not move the measured curve at all. Correctly reasoned; do not spend
more on it as a primary route. It may still be worth doing on top of a real fix, for the unmeasured
content term.

**4.2 — Lead (a)'s arithmetic is more favourable than the report's framing suggests, and this is the
route to evaluate first.** They costed a precomputed pass over the lineage DAG as "an extra full read
of every Block object, before the loop that already reads every Block object once." True — and that
pass is **O(N)**, weighed against an O(N³) → O(N) improvement already banked. Doubling an O(N) I/O pass
to convert O(N × tree) memory into O(frontier × tree) is, on its face, a good trade. Their framing is
right about the *effort* and understates how favourable the *arithmetic* is.

**And the payoff is larger than eviction bookkeeping.** If `verify`'s outer loop iterated in
**topological order** rather than ObjectId order, the memo would only ever need the *frontier* — one
entry for a linear history, bounded by the number of open branches otherwise. That is not evicting
entries you no longer need; it is never creating them. Evaluate that shape first: it addresses the
cause, and it makes any residual bound trivially correct rather than delicately reasoned.

**4.3 — A third option neither of us named: a fixed-capacity memo.** Cap entries, evict, accept
recomputation on a miss. Memory bounded by construction with no topological pass. Cost it — but note
that under *ObjectId-ordered* iteration the hit rate could be poor, which is itself an argument for
4.2. If 4.2 lands, this becomes a cheap belt-and-braces bound rather than the mechanism.

**4.4 — Measure the content term.** Whatever route is taken, the harness needs an edit-heavy variant so
the `TextCache` half stops being invisible. Their caveat identified the gap; closing it is part of this
round, not a later one.

**4.5 — Report the shape before implementing**, as before. If the answer is that no route bounds this
without an unacceptable cost, that is a stop-and-report and it comes back to me — with DC-92's time fix
still unmerged, which is the honest state rather than a regression shipped for speed.

## 5. What they did right, recorded because it is the pattern to keep

They were asked to measure and to report the shape before implementing, and they did exactly that —
including declining both leads with specific, checkable reasons rather than implementing one to close
the condition and leaving the other gap unstated. The `verify_objects` iteration-order check that kills
the naive form of lead (a) is the kind of thing that would have surfaced only after a wasted
implementation.

## 6. Standing

- **DC-92: continues.** Time fix implemented and correct; merge blocked on §4's bound.
- Green **three-platform** CI before the eventual merge.
- Nothing else is assigned. DC-91 remains proposed and awaiting the owner.
