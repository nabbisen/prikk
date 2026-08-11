# DC-95 Verify Coverage and Finding Accumulation — Prerequisite Handoff v1

**Cleared to answer §3's four questions only.** Accepted 2026-08-11,
`rfcs/accepted/DC-95-VERIFY-COVERAGE-AND-FINDING-ACCUMULATION.md`. **No design, no production change.**

## 1. Why this ranks above the tooling work

`prikk verify` *is* the product's claim. Everything else exists so that it can re-derive rather than
trust. Two findings say the command carrying that claim is under-instrumented and under-reported, and
the first one is the sharper of the two:

**I disabled block state verification entirely — twice — and the whole workspace suite passed both
times.** Once on DC-92's Phase A collection, and once on the pre-DC-92 inline call, which proves the
hole predates DC-92. DC-92 closed it for that one path. **What else `verify` does that no end-to-end
test would notice is open, and that is Stage 1.**

## 2. Start with §3.1, because it sizes everything

Enumerate what `verify` checks, from `verify.rs` and `verify/`'s modules — **not from the finding
text**. For each check, answer one question: is there a test that reaches it through
`verify_repository`, or only a unit test calling it directly?

**That inventory is the increment.** Its shape decides how big Stage 1 is, and it is the thing I most
want to see before any code is written.

## 3. The judgement call, and my proposition to accept or reject

Not every check needs an end-to-end control, and pretending otherwise makes Stage 1 unbounded. §3.2 asks
for a rule. Mine, offered as a starting proposition and not a ruling: **any check whose silent absence
would let a repository verify clean when it should not.**

That is the class Finding A is about. If you think the line belongs elsewhere — narrower, wider, or cut
differently — say so with the reasoning; the rule matters more than my version of it.

## 4. Stage 2, and why it waits

`verify` returns structured findings for some classes (publication trust issues, ref divergence) and
hard-errors via `?` for others. **§3.3 asks whether that boundary is principled or incidental**, because
Stage 2 moves things across it and needs to know which.

**Stage 1 precedes Stage 2 and the ordering is the whole point.** Stage 2 rewires error handling
throughout `verify`. Doing that on top of a suite that cannot detect a check silently going missing is
exactly how a verifier loses a check during a refactor — and DC-92 already demonstrated that this suite
could not detect it. **Stage 1 is the instrument Stage 2 gets measured with.**

§3.4 matters more than it looks: enumerate what depends on the first error being *the* error — callers,
tests, exit codes, any CLI output contract. If something relies on early termination to avoid cost on a
badly damaged repository, that is a real constraint and I would rather hear it now.

## 5. Limits

- **No design in this pass.** Answers first.
- **Stage 1 changes no production behaviour** — tests only. Anything that appears to require a
  production change is a finding to report, not to absorb.
- **No change to what `verify` checks.** Better proved and better reported; not stricter, not laxer.
- **Fail-closed is preserved** in Stage 2. Accumulating findings must never turn a hard failure into a
  warning.
- **Do not regress DC-92's performance work.** The benchmark harness is committed; use it if Stage 2
  touches the hot path.

## 6. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer §3 in order. Findings outside scope go in the
report; I register them in `FINDINGS.md`.

## 7. Sequencing

- **DC-93 and DC-94 are also accepted** and are release-tooling work. This one is the product claim and
  the architect ranks it above both — but all three are independent, so order them as suits you.
- Touches `crates/prikk-store`, so **the three-platform CI rule binds the eventual merge**, unlike
  DC-93/DC-94.
