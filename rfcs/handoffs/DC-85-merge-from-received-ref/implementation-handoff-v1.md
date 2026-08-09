# DC-85 Merge From a Received Ref — Handoff v1

**Cleared to answer §3's four questions only.** Accepted 2026-08-09,
`rfcs/accepted/DC-85-MERGE-FROM-RECEIVED-REF.md`. **Design follows their acceptance.**

## 1. This exists because you found my design claim false

DC-78 §D4 said exchange needed no "pull" concept because you could *"receive, then merge, using
machinery that exists."* You showed `execute_merge:65` validates `from_ref` through
`validate_local_branch_ref`, which rejects `remotes/`. **The machinery exists and refuses the input.**
§D4 is corrected in the RFC; this increment closes the gap.

You were right to report rather than close it — it touches the evidence and trust machinery.

## 2. Answer §3's four questions before designing

**§3.1 may show this is larger than it looks**, so take it first: what is the merge baseline when one
side has no local publication chain? A received ref reaches its ancestors through imported **blocks**,
not a ref-log. **Does `ancestors_inclusive` already give you what DC-74 needs, or not?**

**§3.2 is the security question.** DC-78 §D2 ruled adopted keys grant **object** trust, not **ref**
authority. A merge from a received ref adopts patches sealed by a remote maintainer. **Confirm that
merging does not quietly convert object trust into ref authority** — and if it does, that is a
stop-and-report, not something to design around.

**§3.3:** is `validate_local_branch_ref` the right gate to relax, or the wrong one to reach for? It
protects every local branch path; widening it for merge may weaken unrelated surfaces.

**§3.4:** what does the resulting merge block record, given DC-75 already stores mainline, secondary and
baseline?

## 3. Limits

No transport. **No automatic trust adoption on import** — Stage 3 kept that manual deliberately and that
stands. No change to what `verify` checks. **Green macOS run before merge**, per the standing rule.

## 4. Sequencing

**DC-80 is also live**, and its handoff has just been brought current — see its addendum. Yours to order.
