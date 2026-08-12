# DC-95 Stage 1, Round 7 — Review v1

**Reviewing:** `5819c15` on `dc-95-verify-coverage-and-finding-accumulation`.

**Accepted, no conditions.** 23 of 36. Both classifications reproduced independently, including the
reclassification.

## 1. Reproduced, and the report inspection is what separated them

Probed both checks. **The test assertions alone could not tell them apart** — disabling either produces
the same `"expected verify_repository to reject …"` panic, because the `Ok` arm discards the report.
That is round 5's gap, still visible in the assertion shape, and exactly why the *probe* is a separate
act from the test.

Inspecting the full report:

```
coherence  → trust=[]                                  refpub=[] sig=0
unsigned   → trust=["PRIKK-TRUST-PUBLICATION-UNTRUSTED"] refpub=[] sig=0
```

**Load-bearing and downstream-redundant respectively, exactly as reported.** The reclassification is
correct, and the structural reason they give holds: an object with zero signatures trivially has none
matching a trusted key, so `PublicationTrustVerifier` catches it for any `Block`/`RefState` — not a
property of this one fixture.

Gates clean at 627 tests.

## 2. Two fixture bugs, and the second one is the pattern

**Bug 1 was found by probing**: the log-only first draft hit `classify_ref_state`'s *"pointer missing
while committed log history exists"* arm unconditionally, so the probe measured the fixture's own defect
rather than `verify_update`. Fixed by adding a real matching pointer.

**Bug 2 was found before running anything** — two empty `Root` blocks intended as distinct targets are
the same content-addressed object. **That is the third time this specific hazard has appeared** (round
3's Block-trust fixture, round 7's here, and the near-miss in round 5's copied pointer), and this time it
was caught at construction rather than by a confusing probe result.

**That progression is the thing worth noting:** round 3 discovered it, round 5 recognised its shape in a
different guise, round 7 anticipated it. The lesson has moved from finding to habit.

## 3. Recording the round 6 ruling in the module doc

They put the duplicate-identity ruling into `ref_cluster.rs`'s own module documentation rather than
leaving it in the review archive, on the grounds that it is a permanent fact about the code and belongs
in the file a future reader opens.

**Right, and worth generalising.** The classifications are the durable half of Stage 1 — I said so at
round 2 — and a classification that lives only in a review nobody re-reads is worth much less than one
in the file. Keep doing this.

## 4. One requirement for Stage 1's end, stated now so it is not assembled retroactively

The running picture has shifted since round 3, where an early sample suggested an even split. With
round 1's eight shape arms all load-bearing and several redundancies since, the balance is now
different, and **I am deliberately not asserting a tally I have not carefully computed.**

**Stage 1 should close with the classified inventory as an explicit deliverable** — every one of the 36
rows with its final classification, the probe evidence, and the rows that turned out unreachable or
structurally immune. Assembled as Stage 1 goes, not reconstructed at the end from seven review
documents.

That artifact is what a future reader consults to know which checks are the last line of defence — and
it is worth more than the tests, which is the position this review has taken since round 2.

## 5. Standing

- **Round 7: accepted.** 23 of 36.
- **Round 8** next: the remaining checks needing failpoints, format-1 flips, or raw log-byte
  construction — the hardest of round 5's technique groups.
- Green three-platform CI before any merge.
