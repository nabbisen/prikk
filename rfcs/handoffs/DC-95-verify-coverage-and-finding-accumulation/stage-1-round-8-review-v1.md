# DC-95 Stage 1, Round 8 — Review v1

**Reviewing:** `1839d98`, and the resubmitted classified inventory.

**Accepted with one required correction to the inventory's evidence, not to the code (§1).** The
classification is right; the account of how it was established is not.

## 1. "Unprobeable by construction" is wrong — and I probed it

The report states there is no disable-and-restore probe for `PRIKK-TRUST-POLICY-INVALID` because *"it's
the direct, sole consequence of `load_maintainer_trust_policy` failing… There's nothing to toggle off,"*
and classifies it **load-bearing by construction**.

**There is something to toggle off.** `verify/trust.rs:39-54`: on load failure the verifier pushes the
issue, sets `policy_issue_added`, and **early-returns for that envelope and every subsequent one** — so
`verify_trusted_publication_envelope` is never reached and no `PUBLICATION-UNTRUSTED` issue can arise.
Suppressing the push is therefore a clean, meaningful probe of exactly the Stage 1 question.

I ran it:

```
verify_repository_detects_invalid_trust_policy ... FAILED
  assertion failed: report.has_publication_trust_issues()
```

With the push suppressed, the blocking predicate goes false. **Load-bearing — established by probe, not
by construction.**

**Why this matters when the answer is unchanged.** The whole arc of rounds 1–7, and the classification
pass in particular, is that reasoning about classification is unreliable and probing is what settles it.
My own words at the classification pass: *"the classifications were correct. They were not established."*
Round 8 reverts to reasoning for a check that is straightforwardly probeable, and records the reasoning
as the evidence.

**Required:** correct that row's evidence column to record the probe and its result. **"Unprobeable"
should be a rare claim, and when it is made it should be demonstrated rather than argued** — the same
standard every other row has been held to.

No code change; the test is fine.

## 2. What the test gets right, and it is not incidental

It asserts both that the specific code reaches `publication_trust_issues` **and that
`report.has_publication_trust_issues()` is true** — the blocking surface `run_verify` actually decides
from. That is stronger than the `Ok`/`Err` assertion shape rounds 1–7 used, and it is the shape that
would have made round 5's confound visible immediately. Adopt it for the remaining rows.

Reusing the existing malformed-policy content from
`malformed_policy_is_reported_once_while_count_advances` rather than inventing new malformed bytes is
right, and both sub-cases passing on first construction — after several rounds of fixture bugs — suggests
the construction lessons have stuck.

## 3. The inventory's restated totals check out, with one stale line

The table is internally consistent: 23 + 4 + 3 + 11 = 41, and every per-section row sums correctly.
§4 dropping from 9 to 4 is the granularity ruling applied properly — the two "4 checks" groups are one
inventory row each.

**One stale figure:** line 9 still reads *"Stage 1's remaining 12 rows"* against the restated 11 below
it. Fix it. In a document whose whole value is being the authoritative count, disagreeing with itself is
not a cosmetic defect.

## 4. Retire "36"

The inventory now totals **41 rows**, including those already covered before DC-95. "36" was the count
of rule-matching rows among the 44 non-"Yes" ones — a different population, and no longer the useful
denominator now that a complete inventory exists.

**From here, the inventory's own table is the count.** Eleven rows remain. Do not restate progress
against 36; it will only reproduce the drift the granularity ruling just corrected.

## 5. Standing

- **Round 8: accepted**, subject to §1's evidence correction and §3's stale line.
- **§6 is complete.** Eleven rows remain: 5 in §2, 4 in §4, 1 in §5, 1 in §7.
- **Round 9** next: §2's remaining five — failpoints, format-1 flips, raw log-byte construction.
- Green three-platform CI before any merge.
