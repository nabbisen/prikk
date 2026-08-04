# DC-72 Path-Safety Conformance - Handoff

**Cleared to start.** Accepted by the project owner on 2026-08-04, at
`rfcs/accepted/DC-72-PATH-SAFETY-CONFORMANCE.md`.
**Authored by** the architect.
**Size:** unknown until §2's table exists — deliberately. It could be one validator or five.
**Touches:** `crates/prikk-store/src/path.rs`, `refs.rs`'s ref-name validators, and whatever else §2 finds.

## 1. What this closes

**NFR-SEC-03** (`specs/prikk-non-functional-requirements-v1.1.md:89`):

> Path safety. Absolute paths, `..`, reserved names, symlink escape, and **case-insensitive collisions**
> are rejected.

**A security guarantee the project states and does not meet.** Unowned since 2026-07-30.

## 2. Start here, and do not skip it

**The recorded finding is too narrow, and I know that because I checked before writing this.**
`MILESTONES.md` describes it as a ref-name problem. Grepping `crates/prikk-store/src` for
`to_ascii_lowercase`, `eq_ignore_ascii_case`, and `case_insensitive`, excluding tests, returns **nothing** —
so there is no collision rejection for ref names **or** repository paths.

**That one grep already widened the finding.** Assume the rest of the record is equally approximate.

**Produce a table before designing anything**: each of the five clauses × each surface (repository paths,
ref names, tag names, and any other you find), marked met or unmet, with the code that does or does not
enforce it. **Report the table, not a verdict.**

## 3. The two questions that are the actual design

**What is the collision rule?** ASCII case-folding, Unicode case-folding, or normalisation as well — `á`
(one codepoint) and `á` (a + combining acute) also collide on macOS. **ASCII-only is a defensible answer;
an unstated answer is not.** Write it where a user can find it, not only in code.

**What happens to repositories that already contain a collision?** A rule that makes existing history
unverifiable is a format break, not a fix. It must fail closed at **creation** without retroactively
condemning what exists — or that consequence is stated and accepted. This is the one that decides whether
the increment is small.

## 4. Why it matters more this week than last

On a case-insensitive filesystem — macOS default, Windows — two paths prikk treats as distinct nodes
collide in the worktree. One silently overwrites the other on checkout, and **`verify` cannot see it**:
at the object level the history is entirely consistent. The damage is in the working tree, which is
precisely where prikk's guarantees stop.

**DC-71 made read-only commands run on exactly those platforms yesterday.**

## 5. Traps

- **Fixing the case clause and calling NFR-SEC-03 met.** It is one line covering five properties.
  Criterion 5: anything else found unmet is **reported**, and fixed here only if trivial.
- **Choosing a folding scheme implicitly** by reaching for `to_ascii_lowercase` because it is at hand.
- **Making existing repositories unverifiable.** Criterion 4.
- **Treating this as a filesystem-compatibility feature.** It is about rejecting collisions, not tolerating
  them.
- **Trusting `MILESTONES.md`'s description of the gap.** It was already wrong once; see §2.

## 6. Definition of done

§2's per-clause, per-surface table reported before design; the collision rule stated with its rationale
somewhere user-visible; collisions rejected at creation on every surface, tested per surface; existing
repositories still verifiable or the consequence explicitly accepted; any other unmet clause reported;
`MILESTONES.md`'s NFR-SEC-03 row updated to its resolved or narrowed state; full gate set per
`rfcs/EXECUTION-ORDER.md` §6 rule 9 with test counts before and after, **commands verbatim**.

## 7. Standing request

Three defects this week were found by running a sequence rather than reading code, and one requirement's
recorded description was wrong in a way one grep exposed. **The table in §2 is that grep, done
systematically.** If it contradicts anything here — including my claim that no collision rejection exists —
stop and report it.
