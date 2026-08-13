# RFC (accepted) - DC-85 Merge From a Received Ref

**Status.** **ACCEPTED by the project owner 2026-08-09.** **Cleared for §3's four questions only** —
design follows their acceptance, and §3.1 in particular may show this increment is larger than it looks.
**Authored by** the architect. **Independence.** Author-reviewed — the standing ceiling.
**Arises from.** DC-78 Stage 3, which showed that **the architect's §D4 claim was false**: a received ref
cannot be a merge input. The developer identified the gap, declined to close it unilaterally, and
reported it — correctly, since it touches the evidence and trust machinery.

## 1. What is actually broken

`execute_merge` (`merge_execute.rs:65`) validates `from_ref` through `validate_local_branch_ref`, which
**rejects `remotes/`** by design (`refs.rs:386-389`) — the same validator protecting every local branch
path. So `prikk merge --from remotes/<name>` is refused outright.

**DC-78 §D4 justified having no "pull" concept on the grounds that receive-then-merge used machinery that
already existed.** It does not. **Exchange today is complete for an auditor and incomplete for a
collaborator:** you can receive, inspect, verify, and adopt trust — but not incorporate.

## 2. Why it is not a small change

`prepare_merge_evidence` takes `MergeEvidenceTarget::Ref` and assumes both sides are **ref-log-backed
local branches** whose `previous_ref_state_id` chain is reachable through `RefStore`. A received pointer
has **no ref-log and no CAS semantics** — it is a single overwritten pointer in a separate store with its
own format, deliberately so (a received RefState's embedded `ref_name` is the origin's and can never
agree with a local pointer name).

So this needs a new evidence target and a decision about **what confluence means against a source with no
local publication history.** That is a design question, not an integration task.

## 3. The questions this increment must answer before designing

1. **What is the merge baseline when one side has no local publication chain?** DC-74 requires a common
   ancestor proven from both sides; a received ref reaches its ancestors through imported **blocks**, not
   through a ref-log. Is block ancestry sufficient, and does `ancestors_inclusive` already give it?
2. **Does adopting a received ref as a merge source weaken DC-74's guarantees?** The merge would adopt
   patches sealed by a **remote** maintainer key. DC-78 §D2 ruled adopted keys grant *object* trust, not
   ref authority — **confirm that merging does not quietly convert one into the other.**
3. **Is `validate_local_branch_ref` the right gate to relax, or the wrong one to reach for?** It protects
   every local branch path; widening it for merge may weaken unrelated surfaces. A separate
   received-ref-aware validator may be correct.
4. **What does the resulting merge block record?** DC-75 records mainline, secondary, and baseline. If the
   secondary parent came from a received ref, is anything additional needed to keep the merge
   re-checkable by a third party?

## 3A. §3 answered 2026-08-09 — design cleared, with one mandatory constraint

**§3.1 — block ancestry is sufficient; the surface is *smaller* than feared.** The whole
confluence-proving stack (`merge_evidence.rs:180-342`) reads only `ObjectId`s and
`BlockPayload.parent_block_ids`; **`MergeEvidenceTarget::Block(ObjectId)` already bypasses `RefStore`
entirely.** The `Ref` arm exists only to translate a name into an `ObjectId` — a lookup, not a
structural dependency. A received pointer yields that id in two hops. **Verified.**

*Carry into design:* a received `RefState`'s embedded `ref_name` is the **origin's**, so
`merge_evidence.rs:113-118`'s name-equality guard must not be reused verbatim for it.

### 3A.1 — §3.2 is a confirmed stop-and-report, and it is now a mandatory criterion

**Merging from a received ref, with only the validator relaxed, would seal untrusted content into live
history with no trust gate at merge time.** The developer traced it and the architect verified every
premise:

- **`verify_signer_trusted` (`merge_execute.rs:92`) gates the *local signer* about to seal** — not the
  provenance of what is being adopted. Its own comment says so.
- **`RefStore::publish` does not gate on trust either** — `require_signed_type` (`refs.rs:371`) checks
  only `envelope.signatures.is_empty()`, i.e. *presence*, not policy membership.
- **Local-to-local merge is therefore safe by induction, not by an explicit check**: every block on a
  local ref got there through seal or merge, each gated by `verify_signer_trusted` at creation.
- **Import breaks the induction.** `import_bundle` performs no trust check, deliberately (DC-78 Stage 3
  §4). So a received ref's blocks may be signed by a key absent from this policy.

**That is object trust — or its complete absence — becoming ref authority by omission, which §D2 forbids.**

**Mandatory acceptance criterion:** every adopted Block must be confirmed to carry a **currently-trusted**
signature **before or during** `execute_merge` — never deferred to a later `verify` run, by which point
`into_ref` has already advanced. The candidate set is already walked once by
`candidate_blocks`/`candidate_patch_ids`; **checking trust during that walk rather than as a second pass
is the shape to design toward.**

**§3.3 — do not relax `validate_local_branch_ref`.** It gates both merge sides, `resolve_target`'s `Ref`
arm, and `branch create --from`. Widening it would loosen **`into_ref`**, which must remain a genuine
local branch — `RefStore::publish` only ever writes `refs/by-id/`. **Add a separate source resolver**
(a third `MergeEvidenceTarget` path) using the existing `validate_received_ref`, applied to `from_ref`
**only**.

*Dispatch precedent:* follow `run_log`'s prefix-routing shape, **not** `run_verify`/`branch list`'s
list-both shape — the latter answers "show me everything", not "resolve one input".

**§3.4 — nothing additional to record.** A secondary parent's `ObjectId` is identical bytes however it
was reached, and a third party re-derives confluence from the two parent ids and the baseline. Same
reasoning as §D3: provenance the format cares about is intrinsic to the signed object.

## 4. Non-goals

Transport. Automatic trust adoption on import — DC-78 Stage 3 deliberately kept that manual and that
stands. Any change to what `verify` checks.

## 5. Relationship to the status-claim criteria

**Criterion 1 ("sync exists — two machines can exchange sealed history, and both verify it") is satisfied
as worded by DC-78.** Whether the criterion's *wording* was strong enough — given a collaborator cannot
incorporate what they receive — **is an owner question**, recorded in `MILESTONES.md` rather than settled
here by the person who wrote the criterion.
