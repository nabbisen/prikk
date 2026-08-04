# RFC (proposed) - DC-72 Path-Safety Conformance (NFR-SEC-03)

**Status.** **Accepted by the project owner on 2026-08-04.** Implementation may begin; handoff at
`handoffs/DC-72-path-safety-conformance/implementation-handoff-v1.md`.
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

~~**The last two are the design.**~~ **Both are now ruled — see above.** The first two remain, and are reading.

> **Both design questions were ruled by the project owner on 2026-08-04.**
>
> **Collision rule: ASCII case-folding, by constraint.** `tools/release-policy/src/boundary/placement.rs:11`
> permits `prikk-store` only `getrandom` and `rustix`; Unicode folding or NFC/NFD normalisation needs a
> crate, i.e. a DC-51 amendment for a path-comparison rule. **The Unicode cases — NFC/NFD equivalence and
> locale case rules — become a stated, recorded limitation, not an oversight.** Verify the constraint still
> holds; report if correct folding is achievable without a new dependency.
>
> **Existing repositories: reject at creation only, no retroactive validation.** No production use, pre-1.0
> with an explicitly unstable format and no migration promise, not self-hosted, five days on crates.io.
> **Record the decision** so nobody later assumes the check was always there. This is what makes the
> increment small.


## 3.5 Design ruling — the shape of the fix (architect, 2026-08-04)

The §2 table is discharged; these are the design decisions that follow from it, made here rather than left
to implementation. **What remains for the implementer is placement and mechanics, reported for review.**

### One folding definition, four call sites

There must be **exactly one** definition of "these two names collide," cited by every surface. Four
surfaces are in scope — repository paths, branch refs, tag refs, maintainer trust key ids — and four
independent implementations of ASCII folding would be four things to drift.

`prikk-replay::path::validate_no_path_collisions` (`path.rs:38`) already folds ASCII for repository paths
and is the existing prior art. **But it is in `prikk-replay`, and `prikk-object` does not depend on it** —
the dependency runs the other way (`prikk-replay/Cargo.toml:15`). Trust key ids live in `prikk-store`.

**So the primitive's home is a reading question, not a preference**: establish where it can live such that
all four surfaces reach it without inverting a dependency, and **report that placement before writing it**.
`prikk-object` is the lowest crate and the likely answer; confirm rather than assume. If no placement works
without a dependency change, that is a finding to report, not to route around.

### Reserved names on trust key ids: reuse, do not reimplement

`prikk-object::path::is_windows_reserved_name` already checks CON/PRN/AUX/NUL/COM1-9/LPT1-9 against a
component stem, host-OS-independently. **Apply that same function** to trust key ids. A second reserved-name
list would be a second thing to maintain and a second thing to be wrong.

### The check belongs at each surface's existing validation entry point

Not at a new choke point. Each surface already has one — `validate_repo_path`, `validate_local_branch_ref`,
`validate_local_tag_ref`, `maintainer_trust_key_path` — and adding the call there keeps the rejection where
every existing caller already passes.

### Collision scope is per namespace, not global

Two names collide only within the same surface. `heads/main` and a repository path `main` are unrelated.
Refs and tags are **separate namespaces** (`heads/` and `tags/` cannot collide with each other by prefix),
so fold within each, not across.

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
