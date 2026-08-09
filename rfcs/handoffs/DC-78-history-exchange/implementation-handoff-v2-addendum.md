# DC-78 Handoff v2 — Addendum 1: §D7 accepted, four rulings, implementation cleared

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-78-d7-questions-review-v1.md`.

## 1. Accepted. The `remotes/` find is the best thing in the report.

**`remotes/` is already a reserved prefix** — verified at `refs.rs:386-389`, beside `tags/` and
`rollback/`. A namespace was carved out for not-locally-authoritative refs and never implemented. **§D4's
"distinct namespace the local operator never seals to" already had a reserved name waiting, and the
design did not know it.** Use it.

Also verified: `branch list` has no prefix filter, `log --ref` checks only non-emptiness, the key-id
comparison short-circuits at `trust.rs:203` before `verify_ed25519` at `:215`, and
`policy_rejects_multikey_shape` asserts exactly the input that must now become valid.

## 2. Ruling 1 — the parser change, as you described it

Keep the 3-line structure, the literal `[maintainer]` / `required = 1` checks, and the bracket stripping;
split the inner content, validate each candidate through the existing `Signature::validate_key_id`, fail
closed on an empty list, a duplicate, a bad candidate, or malformed syntax. **Hand-rolled and
fixed-shape, per DC-11.**

**`policy_rejects_multikey_shape` inverts — change it deliberately, do not delete it.** The version worth
having asserts *"two keys are accepted, and malformed two-key syntax is still rejected."* The second half
preserves what the original test was actually protecting.

## 3. Ruling 2 — §D7.2 is a clean confirmation and that is a complete answer

`required` is never parsed as a number nor stored; the `.any(...)` model already generalizes. **I asked
you to report if something had assumed otherwise. Nothing had.** A clean confirmation is a valid result
and should not be inflated into a finding — you didn't, and that's right.

## 4. Ruling 3 — namespace awareness is IN SCOPE, narrowly, with one distinction

**In scope: the three sites you named** — `branch list`, `verify`'s ref counting, and `log --ref`'s
presentation. **Not** a general namespace framework.

**Why:** §D4 asserts a distinct namespace, and *a namespace the tools do not honour is not a
separation.* A user seeing received work listed identically to their own branches is the mistake the
design exists to prevent.

**One precision, because it changes how much you build:** this is a **presentation** requirement, not a
provenance one. RΔ5 is satisfied at the object level by the sealer's key id inside the signature (§D3),
and that holds whatever `branch list` prints. Getting the display wrong would be a usability and safety
defect — **not a failure of the provenance guarantee.** Keep them separate and do not over-build.

**`log --ref` reading a received ref is legitimate** and must keep working. It simply must not present it
as local.

## 5. Ruling 4 — §D7.4 accepted

O(objects × keys) in **string comparisons**, with `verify_ed25519` still at most once per signature, and
additive to rather than nested inside the cubic lineage term. **Reasoned from measured structure, not
intuition** — which is the distinction that made it acceptable.

## 6. One finding recorded, not yours

**`log --ref` accepts any non-empty string** with no ref-name validation, where sibling surfaces refuse
malformed input. No security impact — the name is hashed, so no traversal — but it is a robustness gap.
**Pre-existing and unrelated to DC-78**; recorded in `FINDINGS.md` as unowned. **Do not fix it here.**

## 7. Proceed

Implementation cleared under §D and rulings 1–4. **Handoff v2 §4's four negative controls stand
unchanged** — and the first of them, that adopting a second key must leave existing history verifying, is
still the one that decides whether this increment succeeded.
