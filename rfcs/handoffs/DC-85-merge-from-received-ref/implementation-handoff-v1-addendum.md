# DC-85 Handoff v1 — Addendum 1: §3 accepted, design cleared, one mandatory constraint

**Date:** 2026-08-09. **Authored by** the architect. **Answers folded into the RFC as §3A.**

## 1. §3.2 is the finding, and I verified every premise

**You were asked to stop and report if merging from a received ref would convert object trust into ref
authority. It would. You did. That is exactly right.**

I checked all three load-bearing facts independently:

- `verify_signer_trusted` (`merge_execute.rs:92`) gates the **local signer**, not adopted content — its
  own comment says so.
- `require_signed_type` (`refs.rs:371`) checks only `envelope.signatures.is_empty()` — **presence, not
  policy membership**. So `RefStore::publish` is not a trust gate either.
- `import_bundle` applies no trust check, deliberately.

**Your induction argument is the sharpest thing in this report**: local-to-local merge is safe not
because anything checks the adopted side, but because every block on a local ref got there through a
path already gated at creation. **Import breaks that induction**, and nothing downstream notices.

Naming *why* something is safe today — rather than observing that it is — is what made the gap visible.
A weaker analysis would have found no failing test and concluded there was nothing to report.

**This is now a mandatory acceptance criterion**, in the RFC: every adopted Block must be confirmed to
carry a **currently-trusted** signature **before or during** `execute_merge`, never deferred to a later
`verify`, by which point `into_ref` has advanced. **Your own suggestion is the shape to design toward** —
check trust during the existing `candidate_blocks`/`candidate_patch_ids` walk rather than as a second
pass.

## 2. The other three, accepted

**§3.1 makes the increment smaller, not larger.** `MergeEvidenceTarget::Block(ObjectId)` already bypasses
`RefStore`, so block ancestry is structurally sufficient. Verified. Your asymmetry note is carried into
the RFC: a received `RefState`'s embedded `ref_name` is the origin's, so the name-equality guard at
`merge_evidence.rs:113-118` must not be reused verbatim.

**§3.3 accepted in full** — a separate source resolver on `from_ref` only, never `into_ref`, using the
existing `validate_received_ref`. Your point that relaxing the shared validator cannot distinguish the
two sides is decisive, and `into_ref` must stay a real local branch since `RefStore::publish` only writes
`refs/by-id/`.

**And naming both dispatch precedents was worth more than picking one.** `run_log` prefix-routes;
`run_verify`/`branch list` list both unconditionally. Saying which shape fits "resolve one input" and
which fits "show me everything" — and warning that the more recently-touched one is the wrong model —
is the kind of thing that prevents a plausible mistake rather than catching it later.

**§3.4 accepted.** Nothing additional to record, for the reason §D3 already established.

## 3. Proceed to design

Under §3A's constraints. **Green macOS run before merge**, as ever. If the trust gate turns out to cost
materially more than a check folded into the existing walk, **report that** rather than accepting the
cost silently or dropping the gate.
