# RFC 119 track A — park the post-1.0 machinery

**Base:** current `main`. **Under `003-landing-work-on-main.md`.**
**RFC:** `rfcs/accepted/119-release-policy-reset.md` §10 track A — **owner-ordered second, 2026-08-25.**

**Parking is not deletion, and it is not "recorded, not rejected" either.** §2 states the difference,
which this project spent a week learning.

---

## 1. What is parked

**Only these three. The scope shrank after a verdict of mine was corrected — see §5.**

- **The 43 signer oracle cases** — `signer-challenge` (16), `signer-governance` (16),
  `signer-authority` (10), `signer-authority-live` (1). Sampled at case level:
  `authority-grammar`, `bootstrap-two-signers-verified`, `automation-as-approver`. **DC-35's regime.**
- **DC-35's signer-authority rules** as release policy.
- **The official-release boundary** in `release-compatibility.md`.

**Why:** all three serve **G4 as *authority*** — *who* may sign — which the derivation places **post-1.0**.
prikk has **one maintainer publishing under their own key**, an **empty allowlist**, and **no release
that has ever passed the signer audit**. The owner has already ruled criterion 4's two-person
requirement *"not wrong and just too early to be applied."* **This applies the same judgment to the
machinery that implements it.**

## 2. What "parked" must mean, precisely

**Three requirements, and the third is the one that makes this honest:**

1. **It does not run.** Not skipped-but-counted, not run-and-ignored. **Off.**
2. **It is not deleted.** The cases, the rules and the reasoning stay in the tree, findable.
3. **It says what would un-park it.** Each parked thing carries the condition that revives it — for
   these, prikk entering the official-release regime, which requires criterion 4's signer bootstrap.

**Requirement 3 is the difference between parking and the pattern this project has been removing.**
*"Recorded, not rejected"* was a theme with no user benefit that lingered because being recorded read as
being handled. **A parked mechanism with a stated revival condition is a decision; one without is a
deferral wearing a decision's clothes.**

**And requirement 1 is not optional.** The derivation's §8: *"needed later" is only honest if accompanied
by "and therefore does not run now."* **A parked check that still runs is the worst of both — it costs
what it always cost and protects nothing prikk has.**

## 3. The visible consequence, which must not be hidden

**`check`'s output will change**: *"all 154 oracle cases passed"* becomes a smaller number.

**Do not preserve the 154.** The count is a fact about what ran; keeping it would be a false claim of
exactly the kind RFC 118 exists to prevent. **Report the new number.**

**If anything asserts on 154** — a test, a doc, a workflow — **find it and update it.** Report every such
site. **A parked case that some other check still counts is not parked.**

## 4. Out of scope

- **`publication-allowlist`** — **corrected to NOW** (§5). **Do not park it.**
- **Tracks B and C.** The FINER items, the NEVER removals, and G1.
- **`release-signers.toml`.** It stays as it is; parking the checks does not change the file.
- **Deleting anything** (§2.2).
- **Any product behaviour.**

## 5. A verdict of mine was wrong, and it narrowed this track

**The reconciliation verdicted `publication-allowlist` as LATER, "G4 authority — who may publish."
Wrong.** It validates the **eight packages in topological publish order** and **sixteen
`cargo package`/`cargo publish` procedures** — **the exact sequence executed by hand to publish
`0.23.0`**, where a wrong order leaves a half-published release that can only be yanked.

**Corrected to NOW.** I inferred from the category name instead of reading the function.

**Treat my three remaining items with the same suspicion.** I sampled the signer suites at case level
after making that error, and they held — **but check them yourself before parking anything, and report
if any of the three turns out to protect something live.**

## 6. Controls

1. **The parked cases genuinely do not run** — show the count before and after, and that no parked case
   appears in the run.
2. **Un-parking works**: temporarily revive one, confirm it runs and passes, re-park. **A parked thing
   that cannot be revived has been deleted by another name.**
3. **Nothing else changed**: the full gate set passes, and the non-parked oracle cases still pass at
   their new count.

## 7. What to report

1. **What you parked and by what mechanism** (§2.1).
2. **The revival condition recorded for each** (§2.3).
3. **The count before and after**, and **every site that asserted on 154** (§3).
4. **All three controls** (§6).
5. **Anything among my three that turned out to protect something live** (§5).
6. **Full gate set against the exact commit, after the last edit.**
7. Anything here that was wrong.

**Stop and escalate, do not guess**, if: parking a case makes an unrelated one fail — **that is a
dependency between cases and a finding**; the revival condition cannot be stated for something (§2.3) —
**then it is not parked, it is abandoned, and that needs a different decision**; or a parked rule turns
out to be the only thing asserting something prikk needs today.
