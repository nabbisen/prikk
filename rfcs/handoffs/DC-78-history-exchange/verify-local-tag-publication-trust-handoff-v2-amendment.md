# Amendment to `verify-local-tag-publication-trust-handoff-v1.md` — ruling on the escalation

**Supersedes v1's §2 insertion point and §3 caller table. Everything else in v1 stands.**
**Ruling by the architect, 2026-08-24, on
`.git-exclude/review-request/verify-local-tag-publication-trust-escalation-v1.md`.**

**The escalation was correct and the stop was right.** v1's §3 enumerated
**`ensure_ref_target_valid`'s** callers — a narrower question than **`verify_refs`'s** callers, and
`verify_refs` has one that `ensure_ref_target_valid` does not: `ensure_no_incomplete_publication`
(`refs.rs:122`), a **pre-mutation structural guard** reached from eight sites including
`ActiveLock::acquire`, `add_trusted_maintainer`, rollback draft, worktree commit authoring and `doctor`.

**My §3 asked the wrong question and my §2 built on the answer.** Threading a trust verifier through
`verify_refs` would have put trust-policy I/O on eight mutation paths this increment never scoped — with
`add_trusted_maintainer`, the operation that *establishes* trust, paying a trust read as a side effect
of an unrelated guard. **That is the finding I said I most wanted, and it arrived.**

## The ruling: none of the three options — a fourth

**Do not change `verify_refs`'s signature. Do not touch `ensure_no_incomplete_publication`. Do not add a
parameter to any shared function.**

**Add a self-contained `LocalTagTrust` stage to `verify`'s own pipeline**, modelled line-for-line on the
**`RefUpdateSchemaTrust`** stage already in `verify.rs:1024-1035`. That stage is the existing precedent
for exactly this shape: a separate pipeline stage that runs `trust_verifier.verify(envelope)` over a set
of envelopes, **while `verify_refs` itself never sees the verifier.**

The stage does its own enumeration rather than taking a list from `verify_refs`:

1. `RefStore::new(layout).list_ref_pointers()` (`refs.rs:284`).
2. Filter to **`RefKind::Tag`** — local tag refs only. **Received tags are not reachable this way**,
   which is what makes the provenance principle hold structurally rather than by a flag.
3. Resolve each to its `Tag` envelope and `trust_verifier.verify(envelope)`.

**Why this beats all three options you offered:**

- **vs. option 1 (accept the cost):** the eight mutation paths pay **nothing at all** — not a policy
  read, not an extra object read, not an allocation. `verify_refs` is untouched.
- **vs. option 2 (skip-flag):** no parameter on a shared function. v1's §3 rejected that shape one level
  down; **accepting it one level up needed a principle neither of us had**, and you were right to say so.
- **vs. option 3 (split the read path):** no split needed. The separation already exists — it is the
  stage boundary, and `RefUpdateSchemaTrust` is proof the codebase already works this way.

**The cost is one re-read of each local tag envelope on the `verify` path only** (the ref scan reads it
too, inside `ensure_ref_target_valid`, and drops it). **That is acceptable**: `verify` is an audit, tags
are few, and the alternative is contaminating a shared signature to save a handful of reads. **If that
cost turns out not to be small — say, if enumeration forces a full container decode per tag — stop and
report, do not absorb it.**

## What I verified, and what I did not

**Verified:** `RefUpdateSchemaTrust`'s shape at `verify.rs:1024-1035`; that `verify_refs` never receives
the verifier today; that `RefStore::list_ref_pointers` exists at `refs.rs:284`;
`ensure_no_incomplete_publication`'s caller set as you reported it.

**Not verified — check before building on it:** that `RefPointerSummary` carries the `RefKind` needed to
filter, and that resolving a summary to its `Tag` envelope is clean from inside `verify.rs`. **If either
does not hold, escalate again rather than working around it** — a second wrong insertion point is worth
another stop.

## Your current working tree

**Discard it.** The `check_local_tag_publication_trust` helper in `scan.rs`, the `pub(super)` →
`pub(crate)` widening, and the `verify_refs` signature change are all superseded. **Nothing in it was
wasted** — it is what proved v1's design wrong, and the escalation document is the deliverable.

## Unchanged from v1

**§1** (why the blanket type-based check is wrong), **§2's principle** (*publication-trust expectation
follows provenance, not type* — the ruling changes only where it is enforced), **§4** (all four tests,
including the tag-travel `verify` assertion, which remains the control), **§5**, **§6**, **§7**.

**The `053e442` precedent for negative controls still applies:** mutate by keeping the call and
swallowing the refusal, not by deleting the line.
