# DC-74 Handoff v1 — Addendum 1

**Read with** `implementation-handoff-v1.md`, which remains valid in full. Nothing in it is withdrawn.
**Date:** 2026-08-08. **Authored by** the architect.

## 1. Your prerequisite report is accepted — you are cleared for §3

`prerequisite-questions-v1.md` is accepted. Review at
`.git-exclude/reviewed/DC-74-prerequisite-questions-review-v1.md`. **Q1 answered the question that could
have ended this increment: B′ (adoption) is available.** Answering it by capturing P's canonical bytes and
re-reading them byte-for-byte after adoption was the right evidence, and it is what the conclusion rests
on. **Proceed to §3 scope.** Two findings are recorded there; **neither is a repair you owe.**

**One correction to carry forward, so it is not repeated as a claim.** Your Q1 wrote that
`verify_repository` accepting the repository supports signature survival. It does not:
`verify_repository` performs **no cryptographic verification of patch author signatures**. The product
has exactly one crypto verify call site — `crates/prikk-store/src/trust.rs:215`, for a *policy* signature
— and `classify_signature_envelope` checks envelope **shape** only. Repository-wide AUTHOR trust
verification is DC-53, proposed and unscheduled. The byte-identity evidence is sound and sufficient;
**cite that, not verify.**

## 2. A release condition now attaches to this increment

Ruled by the owner 2026-08-08, recorded in `MILESTONES.md` under "Attached release conditions":

> **Merge execution does not ship until sealed history structurally records a merge** — a later verifier
> must be able to re-check the baseline and both sides.

**This gates release, not your work.** Build and merge normally. **Do not treat this as a hold.**

**Why it exists.** `parent_patch_ids` is inert — `Vec::new()` at every construction site including
`worktree_patch/node_authoring.rs:534`, and read nowhere. **There is no patch DAG.** So with
single-parent blocks, nothing in sealed history records the baseline the confluence check ran against,
and a merge's correctness cannot be re-derived by a later verifier. The architect's §3.3 deferral
argument assumed a patch DAG existed; **that was the architect's error, not a gap in your report** — you
were never asked to check it.

## 3. What this means for how you sequence the work

**How the merge gets recorded is not yet ruled.** The architect's recommendation is multi-parent blocks:
already legal in the format, and `verify.rs:327`, `checkout.rs:186`, `store_resolvers.rs:53` **already
traverse all parents**, so the work is confined to derived-state machinery — `patch_replay/read.rs:62`,
`patch_inverse/read.rs:56`, `incremental.rs:133`, `block_state.rs`'s four `.first()` sites,
`cache_ladder`. Verification already handles a DAG; only replay does not.

**Sequencing is yours, with one warning.** Merge execution written against single-parent replay will
likely need revisiting once the record lands. If you judge multi-parent lineage the cheaper first step,
**say so and it will be scoped as its own increment** — that is a report worth making, not a scope
change you need permission to propose.

## 4. Unchanged

§1's answered questions do not need redoing. §2's "a merge authors nothing" constraint stands, and is
the thing the release condition exists to protect. §4's acceptance criteria and §5's non-goals are
unchanged — including that finding `patch_algebra`'s conservative subset too narrow is a **finding to
report**, not a thing to widen here.
