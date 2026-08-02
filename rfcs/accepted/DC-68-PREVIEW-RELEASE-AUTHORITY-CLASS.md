# RFC (accepted) - DC-68 Preview Release Authority Class

**Status.** **Accepted 2026-08-02 by the project owner's approval of Option C** at
`.git-exclude/reviewed/prikk-dc35-two-person-rule-options-v1.md`. That approval is the acceptance; this RFC
is the vehicle for it, not a second gate. Implementation may begin.
**Authored by** the architect, who holds minor/patch release scheduling and cycle management by owner
delegation of 2026-08-02.
**Amends.** `rfcs/accepted/DC-35-RELEASE-COMPATIBILITY-STATUS-CORRECTION.md`.
**Blocks.** Every release at every version until it lands.

## 1. Two blockers, one cause — both verified

**Blocker 1 — two-person rule.** `DC-35:219` requires "two distinct natural persons" per authority
transaction and states automation "cannot occupy either accountable approval identity." Populating
`release-signers.toml` (`authorized_primary_fingerprints = []`, fail-closed) is such a transaction.

**Blocker 2 — authority root.** The same section requires observed branch-protection review controls or a
reviewed equivalent, else "release remains blocked." **Verified 2026-08-02:
`gh api repos/nabbisen/prikk/branches/main/protection` → 404, "Branch not protected."** The declared
equivalent also demands "two accountable approvals," so it fails identically.

**Both fail because this project has one natural person.** One increment, because fixing either alone
leaves the other blocking.

## 2. Prerequisites — discharged 2026-08-02 by the architect, before amendment text

| Question | Answer |
|---|---|
| Can branch protection help? | `main` is unprotected. **Enable it regardless** — cheap and a real improvement — but a solo admin can bypass their own protection, so "observed **no-bypass** review path" stays unachievable. **It does not discharge blocker 2.** |
| Does `release-policy` encode these rules? | **No.** `tools/release-policy/src/oracle/self_test/` only copies `release-signers.toml` as a fixture; the two-person rule is prose only. **No oracle-case change is required** — recorded as a finding per criterion 4. |
| What does release evidence look like? | `release/fixtures/release-evidence-*.json`, `schema_version` plus a **`governance`** key. The preview declaration belongs there — an existing field, not a new artifact. |
| What else asserts two-person authority? | **`docs/src/reference/release-compatibility.md:180-181`** states it publicly. Must be reconciled or it becomes a false public claim. |

## 3. The amendment

**Official releases: nothing changes.** Two natural persons, branch protection or reviewed equivalent,
signer proofs — all stand.

**A `preview` class** is permitted under single-person authority and **must declare in its evidence's
`governance` record what it did not receive**:

- did **not** receive two-person signer authority;
- authority-change root was **not** an observed no-bypass review path;
- the single accountable identity, named;
- repository format unstable, no compatibility promised.

**The declaration is a release artifact.** A preview release whose `governance` record omits it is not a
valid preview release. **Class is a property of the release, not of a version number** — nothing about
`0.x` implies preview.

## 4. Why this and not a global relaxation

Amending the rule to one person globally converts a control that *prevents* a compromised or coerced
maintainer admitting a signer into one that merely makes it *detectable afterwards*, while leaving the
surrounding language intact — how a requirement comes to describe a system nobody built. NFR-PERF-02 did
exactly that and cost DC-57 four days.

**prikk's own thesis governs: guarantees are repository facts, not conventions.** A release that did not
receive two-person authority must not be labelled as one that did. Scoping makes the label true.

## 5. Acceptance criteria

1. DC-35 amended in place — official class unchanged, preview class defined — marked as an amendment and
   dated, not silently edited.
2. The preview declaration's contents specified precisely enough that its **absence is detectable**, not a
   judgement call.
3. `docs/src/reference/release-compatibility.md:180-181` reconciled — it currently makes the unscoped claim
   publicly.
4. Recorded that `release-policy` does not encode these rules, so no oracle change was needed (§2).
5. **A worked example**: what 0.18.0's `governance` preview declaration would say, concretely.
6. `release-signers.toml` is **not** populated here. Amending a rule and exercising it are separate acts.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus release-policy `check`, `boundary-check`,
   `reference-check`.

## 6. Independence, stated plainly

A security-control amendment written and reviewed by one architect, for a one-person project, to unblock
that project's releases. **That is a weak review position and is recorded rather than glossed.**

Compensating: it **narrows** rather than relaxes — no official-release control changes; criteria are
reproducible from the repository; and criterion 6 keeps the amendment separate from its first use.

## 7. Non-goals

Populating `release-signers.toml`; activating the release lane (still the three-authority commit, after
this); changing any official-release control; deciding when the project leaves preview — that is 1.0's
question.
