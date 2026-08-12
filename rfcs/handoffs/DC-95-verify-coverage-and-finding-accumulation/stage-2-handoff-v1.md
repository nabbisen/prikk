# DC-95 Stage 2 — Prerequisite Handoff v1

**Cleared to answer §3.3 and §3.4 only.** Authorized by the project owner 2026-08-12, after Stage 1
closed and merged at `5477df5`. **No design, no implementation, no production code.**

§3.1 and §3.2 were Stage 1's and are discharged.

## 1. What Stage 2 is, and why it waited

> **Change `verify` to collect findings rather than stop at the first.**

The RFC's own §2 states the ordering rationale, and it is worth quoting because it is now load-bearing
rather than aspirational:

> *"Stage 2 changes error handling throughout `verify`. Doing that on top of a suite that cannot detect
> a check silently going missing is how a verifier loses a check during a refactor. **Stage 1 is the
> instrument Stage 2 is measured with.**"*

That instrument now exists: 41 checks classified, the classification in `verify.rs`'s own module doc,
green on three platforms.

**The two stages have different proofs and must not be bundled.** Stage 1's was *"disabling each check
now fails a test."* Stage 2's is *"the same defects are reported, and now all of them at once."*
Bundled, a reviewer cannot tell which half a failure came from.

## 2. The safety net has seven known holes — this is the most important thing on this page

Stage 1's tests will catch a load-bearing check that disappears during the refactor. **They will not
catch all of them.**

| Class | Count | Protected by an end-to-end test? |
|---|---|---|
| Resolved (load-bearing or downstream-redundant) | 34 | **Yes** |
| Excluded — non-blocking | 4 | **No** |
| Provably unreachable | 3 | **No** |

The seven have no end-to-end test **by design and by ruling** — an excluded check backs no blocking
predicate, and an unreachable one cannot be reached to test. **But "no test" means the refactor can
delete or bypass them and every gate stays green.**

Name them, and watch them by inspection rather than by suite:

- **Excluded:** `LEGACY-TIMESTAMP`; `RefLog`-source signature-envelope; `ActiveWal`-source
  signature-envelope; `MissingForEmptyWal`/`ValidForEmptyWal`.
- **Unreachable:** topological cycle; duplicate ref-pointer identity; duplicate ref-log identity.

**The three unreachable ones are the sharper risk.** Round 6 ruled them kept precisely because their
unreachability is a property of *today's* canonical path scheme, not a stated invariant. A refactor that
changes error propagation is exactly the moment someone deletes a check that "can never fire."

## 3. §3.3 — the accumulate-versus-hard-error boundary

> *"What does `verify` already accumulate, and why the split? Report the existing boundary and whether
> it is principled or incidental — Stage 2 needs to know which it is before moving anything across it."*

**This question has a designated open item to close, registered since Stage 1 round 4:**
`signature_envelope_issues` is populated from every source into one `Vec` on `RepositoryVerification`,
and **no `has_*` predicate reads it.** So it is observed and never blocking.

**Four of Stage 1's exclusions rest on that single fact.** If §3.3 concludes the boundary is incidental
and that vector should back a blocking predicate, **all four excluded rows reopen** and Stage 1's
classification changes with them. Say so explicitly if you reach that conclusion; do not treat it as a
side effect.

Start from `verify.rs`'s `RepositoryVerification` and the eight failing predicates `run_verify` consults
in `prikk-cli/src/main.rs`. Report the boundary as it is, then judge it.

## 4. §3.4 — what breaks if `verify` stops short-circuiting

> *"Enumerate callers and tests that depend on the first error being *the* error, including exit codes
> and any CLI output contract. If a caller relies on early termination for cost reasons on a damaged
> repository, say so."*

Three things to be specific about:

1. **The `Err`-shaped checks are the population most affected.** Round 9 established that several
   checks surface as `Err` from `scan.rs`/`verify.rs` rather than as report entries — those are exactly
   the ones accumulation has to convert, and their tests assert `Ok(_) => panic!`, which will need to
   change. Enumerate them.
2. **Exit codes and output are a contract**, not an implementation detail. If accumulation changes what
   `prikk verify` prints or returns, that is a user-visible change and needs stating as one.
3. **Cost on a damaged repository is a real concern, not a hypothetical.** Short-circuiting bounds work
   on a repository that is badly broken. If removing it makes `verify` walk a large damaged repository
   to completion, quantify it rather than noting it.

## 5. The rule Stage 1 ends with, which applies directly here

`verify.rs`'s module doc now states it generally, from three independent instances (rounds 10, 11, 12):

> **A check's own code being present does not establish that a defect reaches it.**

**A refactor of error propagation changes which upstream gate intercepts what.** A check that is
load-bearing today can become unreachable tomorrow purely because an earlier gate now returns a finding
instead of an `Err` — with no test failing, because the defect is still reported, just by something
else. **Watch for that specifically; it is the failure mode this codebase has produced three times.**

## 6. Constraints

- **No production code this round.** Both prerequisites are investigation with a written answer.
- **A stop-and-report is a complete outcome**, as it was for RFC 101 and for DC-87 Stage 2. If §3.4
  finds a caller contract that accumulation cannot preserve, say so and stop.
- **Do not begin the refactor from these answers.** Stage 2's design is a separate, reviewed step.
- Green three-platform CI before any merge, unchanged.

## 7. Where to start

**§3.3 first.** It is the cheaper of the two, and its answer determines whether four of Stage 1's
classifications survive — which changes what §3.4 is enumerating over.
