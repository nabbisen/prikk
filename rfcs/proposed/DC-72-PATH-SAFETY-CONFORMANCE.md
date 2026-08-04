# RFC (proposed) - DC-72 Path-Safety Conformance (NFR-SEC-03)

**Status.** **Proposed 2026-08-04.** Awaits owner acceptance.
**Authored by** the architect. **Independence.** Author-reviewed — the standing ceiling.
**Requirement.** **NFR-SEC-03**, `specs/prikk-non-functional-requirements-v1.1.md:89`:

> Path safety. **Absolute paths, `..`, reserved names, symlink escape, and case-insensitive collisions are
> rejected.** Gate: M1/M3.

**Gate status.** Product **M1/M3** — **missed and carried.** A stated security guarantee that is not met.

## 1. The finding, and it is wider than previously recorded

`MILESTONES.md` has carried this since 2026-07-30 as a **ref-name** problem: `validate_local_branch_ref`
has no case-collision rule, so `heads/Main` and `heads/main` coexist as distinct refs.

**That understated it.** Grepping `crates/prikk-store/src` for `to_ascii_lowercase`,
`eq_ignore_ascii_case`, and `case_insensitive`, excluding tests, returns **nothing**. There is no
case-collision rejection anywhere — not for ref names, and not for repository paths.

**So the clause is unimplemented on every surface it covers, not on one.**

## 2. Why this one is worth doing now

It is a **security requirement the project states and does not meet**, and it has been unowned for five
days while other work proceeded. Unlike NFR-PERF-01 it is not expensive or possibly-inherent; unlike the
M4 work it needs no product decision. It is the cheapest false claim to stop making.

**Concrete harm it permits:** on a case-insensitive filesystem (macOS default, Windows), two paths prikk
treats as distinct nodes collide in the worktree. One silently overwrites the other on checkout, and
`verify` cannot see it because at the object level the history is consistent. DC-71 just made read-only
commands run on exactly those platforms, which raises the odds of someone meeting it.

## 3. What must be established before designing — blocking

**Do not assume the case clause is the only gap.** NFR-SEC-03 names five hazards; the recorded finding
named one, and §1 shows that record was already too narrow.

| Question | Why it blocks |
|---|---|
| **Which of the five clauses are actually met?** Absolute paths, `..`, reserved names, symlink escape, case collisions — each traced, per surface | The requirement is one line covering five properties. **Report a table**, not a verdict |
| **Which surfaces does "path" cover?** Repository paths, ref names, tag names — at minimum | The recorded finding covered ref names only and was incomplete |
| **What is the collision rule?** ASCII case-folding, Unicode case-folding, or NFC/NFD normalisation too? | `á` and `á` collide on macOS. **Choosing ASCII-only is defensible but must be a stated decision, not an accident** |
| **What happens to repositories that already contain a collision?** | A new rule that makes existing history unverifiable is a format break, not a fix. It must fail closed at *creation* without retroactively condemning what exists — or that consequence is stated |

**The last two are the design.** The first two are reading.

## 4. Acceptance criteria

1. §3's four questions answered and reported **before** a fix is designed, including the per-clause,
   per-surface table.
2. The collision rule stated explicitly — folding scheme and rationale — in a place a user can find, not
   only in code.
3. Collisions rejected at creation on every surface §3 identifies, tested per surface.
4. **Existing repositories containing a collision remain verifiable**, or the consequence is stated and
   accepted explicitly. No silent retroactive invalidation.
5. Any of the other four clauses found unmet is **reported**, and either fixed here if trivial or recorded
   as its own finding. **Do not silently widen scope.**
6. `MILESTONES.md`'s NFR-SEC-03 row updated to its resolved state, or to a narrowed one naming what remains.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after.

## 5. Non-goals

- Case-insensitive **filesystem support** as a feature. This is about rejecting collisions, not tolerating
  them.
- Unicode normalisation of stored paths. If §3's third question chooses ASCII folding, normalisation is a
  separate question to record, not to absorb.
- Reopening DC-54's path validation symmetry, unless §3's table shows it left a clause unmet.
