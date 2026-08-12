# DC-95 Stage 1, Round 5 — Review v1

**Reviewing:** `8e5cb85` on `dc-95-verify-coverage-and-finding-accumulation`.

**The three tests are accepted as regression guards. The classifications from rounds 1–5 are not
established, and that includes this round's two.** §2 is the finding; §4 is the condition.

**This is downstream of their own §2 finding.** They identified that `Ok(_)` proves only "no hard error,"
not "clean report," and **explicitly declined to claim rounds 1–4 were unaffected** — calling their own
read an inference rather than a re-verification. That refusal is what made me go and check. It was worth
more than a clean report would have been.

## 1. What I checked

I re-ran the probes with the **full report** inspected rather than the `Ok`/`Err` discriminant.

**Round 1 — shape validation disabled:**
```
case "root-with-parent": trust=[PRIKK-TRUST-POLICY-INVALID] blocking=true
```

**Round 2 — snapshot-blob check disabled:**
```
case "missing-snapshot-blob": trust issues = 1
```

**Round 5 — `ensure_ref_target_valid` disabled:**
```
DANGLING: refpub=0 trust=["PRIKK-TRUST-POLICY-INVALID"] blocking_trust=true
```

Round 5's own reported figure — `ref_publication_issues: []` — is correct. It is also only one of the
report's vectors.

## 2. The root cause, and it is shared by every round

**None of these fixtures establishes a trust policy.** With no policy file, `PublicationTrustVerifier`
emits `PRIKK-TRUST-POLICY-INVALID` for the repository, and `has_publication_trust_issues()` is one of the
eight predicates `run_verify` treats as failing.

**So the baseline repository never verifies clean** — before the check under test is disabled, and after.
`prikk verify` fails on these fixtures either way, for a reason unrelated to the check being studied.

That makes the question *"does removing this check let a repository verify clean?"* **unanswerable
against these fixtures.** Not answered wrongly — unanswerable.

**This is the same confounding class as round 1's arbitrary state roots and round 5's own copied pointer,
one level up.** Not "another check catches this defect," but "the fixture carries an unrelated blocking
defect, so the verdict never moves."

## 3. What this does and does not invalidate

**The tests stand.** Every one asserts `Err` and would still catch its check being removed — the
`Err`→`Ok` transition is real and is what a regression guard needs. Nothing needs rewriting to keep
working.

**The classifications do not stand.** Every "load-bearing" in rounds 1, 2, 3 and 5 rests on a probe that
could not distinguish load-bearing from downstream-redundant, because the repository failed regardless.

**And that is the half I called the more durable one.** My round 2 review: *"a future reader learns more
from 'this check is redundant with `validate_v2_lineage`'s read' than from the test itself."* That
statement still holds — which is exactly why the classification cannot be left as recorded.

**Round 4 I have not re-checked**, and I am not going to assume it is fine on the same reasoning that
just failed. Its argument traced the `has_*` surface directly, which is structurally stronger, but its
fixture is likely to carry the same trust issue. **Treat it as unverified with the rest.**

**The redundant classifications are safe.** Rounds 2, 3 and 5's *downstream-redundant* findings were
established by observing a *different specific error or issue code* appearing — that observation does not
depend on the repository otherwise being clean.

## 4. Condition: re-classify with fixtures that can verify clean

**Establish a real trust policy in each fixture before writing the defect**, so the baseline repository
genuinely verifies clean and the probe's question becomes answerable. **The technique is already in this
file** — round 3's `Block` publication-trust test calls `add_trusted_maintainer` before writing its
untrusted block, for exactly this reason.

Then re-run every probe from rounds 1–5 with the **full report** inspected, and correct the recorded
classification wherever it moves. **Report which ones changed** — that list is the finding, and I expect
it to be non-trivial.

**Do not rewrite the tests to assert an empty report.** They are regression guards and their `Err`
assertion is the right one. The classification is established by the *probe*, which is a separate act
from the test — keeping those two distinct is what let this be caught at all.

**If a fixture cannot be made clean** — some checks may only be reachable in a repository that is
already irregular — say which and why. That is a legitimate outcome and it belongs in the record.

## 5. What is right in this round, independent of the above

- **Splitting into `verify/tests/ref_cluster.rs`** rather than growing one file past readability, with
  ~10 more checks coming. Correct.
- **The per-cluster table question, asked again and answered "no"** with six distinct technique groups
  named. That is the second time it has been asked properly rather than assumed.
- **The copy-versus-move fixture correction.** The first draft left the original pointer in place, so
  disabling the check produced `"duplicate pointer identity"` — a different check firing. Caught by
  probing before trusting, and fixed by moving rather than copying.
- **The `#[cfg(test)]` re-export**, mirroring the existing one immediately above it and justified by
  `validate_coherent_publication` making the fixture unreachable through `publish`. Minimal and
  precedented.

Gates clean at `8e5cb85`: 624 prikk-store tests, fmt, clippy, both toolchains, `git diff --check`,
`cargo audit`, all three release-policy checks.

## 6. Standing

- **Round 5's tests: accepted.** 20 of 36 covered as regression guards.
- **Classification pass required** before round 6, covering rounds 1–5. It is cheap — the probes already
  exist; only the fixtures' trust setup and the report inspection change.
- Green three-platform CI before any merge.
