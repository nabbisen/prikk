# DC-92 Lineage Replay Memoization — Prerequisite Handoff v1

**Cleared to answer §4's five questions only.** Accepted 2026-08-11,
`rfcs/accepted/DC-92-LINEAGE-REPLAY-MEMOIZATION.md`. **Design follows the answers.**

## 1. Why this one matters more than its size suggests

`prikk verify` costs O(N³) — 82 ms at 5 sealed blocks, **34.2 seconds at 160**. That is not a
performance ticket sitting beside the product claim. Verification *is* the claim. The block chain that
bounds patch-algebra cost — prikk's structural answer to Darcs's exponential merge — is the same
structure `verify` re-derives from genesis for every block. **Prikk traded a merge-path cost for a
verification-path cost, and this is the bill.**

DC-78 made it worse in kind, not just degree: genesis-complete transfer means a receiver re-verifies
whole history on every import, so this is now a per-exchange cost, not an audit-time one.

## 2. Step zero, before any of §4: reproduce the baseline

Criterion 2 asks for before-and-after curves at the finding's own N values (5 → 160). **You cannot show
a curve improved if you cannot reproduce the original.** The measurement came from DC-75's prerequisite
investigation; reproduce it first and report whether your numbers match. If they do not, that is itself
the first finding and everything downstream depends on knowing it.

## 3. Where to start, and what I expect to be contentious

**§4.1 first — measure seal — because it decides this increment's scope.** `derive_next_state_root` has
three production callers and only one is `verify`: `seal.rs:156` and `merge_execute.rs:165` are the
others. On the seal path `parent` is the current tip, so the whole ancestor chain is re-verified on
every seal, unconditionally, with nothing on the path caching. **That reads as O(N²) per seal.**

**It is my hypothesis from control flow, not a measurement, and I may be wrong.** Refuting it is a good
outcome and narrows this increment to `verify` alone. Note that DC-59's harness cannot answer it as
built — it times `commit` and states four times over that its seals are untimed setup
(`dc59_commit_benchmark.rs:27`, `:30`, `:118`, `:315`). Extend it or write a sibling; say which and why.

If it holds, **NFR-PERF-01's evidence has a blind spot exactly where the cost is.** Report that as a
finding; do not attempt to re-claim or re-scope NFR-PERF-01 — that is the owner's on evidence.

**§4.5 is the one I will not let slide.** `verify`'s guarantee is that it re-derives rather than trusts.
Memoization must change **how many times** work happens, never **what is checked**. State the invariant
that preserves that, precisely, before relying on it. **The failure mode of caching a verifier is that
it quietly stops verifying, and a faster green run looks exactly like a correct one.** That is why
criterion 3 wants corruption injected at genesis, mid-chain, and tip positions — three separate proofs,
not one.

**§4.3 has an answer I expect but have not proved.** A memo table that is in-memory and lives for a
single `verify` invocation cannot persist, cannot be tampered with between runs, and cannot go stale —
so it likely sidesteps NFR-PERF-04 rather than needing to satisfy it. **If you find anything persisted
is unavoidable, stop and report**; that is a trust-argument question and it comes back to me before
design, not after.

## 4. Limits

- **No design in this pass.** Answers first.
- **No change to what `verify` checks or reports.** Faster, not laxer.
- **Do not touch DC-64's persisted lifecycle cache.** Different path, different trust argument, out of
  scope unless §4.3 forces the question.
- **Do not re-scope NFR-PERF-01 or NFR-PERF-03.** Report what you measure; the claims are the owner's.
- **"I could not determine this" remains a first-class answer.**

## 5. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer §4 in order, with §2's baseline reproduction first.
Findings outside scope go in the report; I register them in `FINDINGS.md`.

## 6. Sequencing

- **This is the only assigned increment.** DC-87 Stage 2 and its Stage 1 seam are deferred under the
  owner's accepted, tracked deferral; DC-91 is proposed and not yet accepted.
- Touches filesystem-backed state once it reaches implementation, so the **green three-platform CI rule
  will bind the eventual merge** — not this investigation, which writes no code.
