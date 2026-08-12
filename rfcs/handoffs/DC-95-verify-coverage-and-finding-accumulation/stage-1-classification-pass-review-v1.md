# DC-95 Stage 1, Classification Pass — Review v1

**Reviewing:** `0441bbd` on `dc-95-verify-coverage-and-finding-accumulation`.

**Accepted. Zero classifications flipped, and I reproduced that independently rather than taking it.**

**My stated expectation was wrong.** The round 5 review said *"I expect it to be non-trivial."* It was
trivial — 14 rows re-probed, none moved. §3 is why the pass was still necessary.

## 1. Reproduced, not accepted

The evidence for this pass lives in `evidence/gates-summary.txt` and a scratch module deleted before the
commit, so it is not reproducible from the tree. I rebuilt the re-fixturing myself — a real
`Ed25519MaintainerSigner`, adopted via `add_trusted_maintainer`, behind every block in round 1's shape
fixture — disabled `validate_block_v2_shape`, and printed the full report:

```
case "root-with-parent": CLEAN? trust=[] refpub=0 sig=0
```

**A genuinely empty report.** Round 1's classification holds under the corrected methodology, arrived at
independently.

**And the claim that rounds 3 and 4 were never exposed checks out structurally.**
`verify/objects.rs:255` gates trust verification behind
`matches!(object_type, ObjectType::Block | ObjectType::RefState)`, and those rounds' fixtures write a
Patch and a Blob. The confound could not have reached them. **They confirmed it by observation
(`checked_publication_trust_records: 0`) rather than resting on that reasoning** — which is the right
instinct, since resting on reasoning is what produced the original error.

Round 5's RefState-name-mismatch row correctly needed no re-probe: it was classified by a *different
issue code appearing*, which my own §3 said is unaffected by the baseline's cleanliness.

Gates clean at `0441bbd`: 624 tests, net 0 — doc comments only, as stated.

## 2. The differentiated account is the part that makes this trustworthy

A weaker report would have said "re-probed everything, all fine." This one says **which rows were
genuinely exposed to the confound (round 1's 8, round 2's 1, round 5's 2 — all writing Block/RefState
objects), which were structurally immune (rounds 3 and 4), and which needed no re-probe at all** (round
5's third row).

That distinction is checkable, and I checked it. A blanket "all fine" would not have been.

## 3. Why the pass was necessary even though nothing changed

**The classifications were correct. They were not established.** Those are different states, and only
the second is worth recording.

Before this pass, every "load-bearing" claim rested on a probe that could not have distinguished
load-bearing from downstream-redundant — the repository failed either way. That the answers happened to
be right was luck, not method. **A record that says "load-bearing" is only worth as much as the procedure
behind it**, and a future reader relying on these classifications is relying on the procedure, not on the
outcome.

So: an anticlimactic result, and the right one to have gone and got.

## 4. On deleting the scratch probe module

I considered whether this repeats DC-75's mistake — the missing benchmark harness that made DC-92's step
zero impossible, which I have criticised twice.

**It does not, and the difference is structural.** A classification probe works by *editing production
code* to disable a check. That cannot be committed as a test; it is inherently a manual act performed
against a working tree. There is no harness to retain. The durable artifact is the recorded
classification in the doc comments, and that is now in the tree.

Recording the reasoning so the parallel is not raised again later.

## 5. Standing

- **Classification pass: accepted.** Rounds 1–5's classifications are now established, not merely
  correct.
- **Round 6** next: the remaining ~10 checks in the `refs/verify.rs` + `scan.rs` cluster, across the
  technique groups round 5 enumerated (failpoints, format-1 flips, raw log-byte construction).
- **The corrected methodology applies from here**: adopted signer in the fixture, full report inspected
  on the probe, classification recorded per check.
- 20 of 36. Green three-platform CI before any merge.
