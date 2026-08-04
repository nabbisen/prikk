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

## 3. Both design questions were ruled by the owner on 2026-08-04 — read this before §2's table

**They are settled. What remains is verifying the reasoning, not choosing.**

### 3a. Folding scheme: ASCII case-folding, by constraint

`tools/release-policy/src/boundary/placement.rs:11` permits `prikk-store` **exactly two** third-party
crates: `getrandom` and `rustix`. Real Unicode case-folding or NFC/NFD normalisation needs
`unicode-normalization` or `icu` — which requires a **DC-51 amendment**, a release-policy control-surface
change, for a path-comparison rule.

**So ASCII case-folding is the expected answer, chosen by constraint rather than preference.** Verify that
constraint still holds; if you find a way to fold correctly without a new dependency, report it rather than
assuming ASCII is mandatory.

**The Unicode cases become a stated, recorded limitation, not an oversight.** Both of these still collide
on macOS and will not be rejected:

- `café` as `c a f é` (U+00E9) versus `c a f e` + U+0301 — different bytes, same file after macOS
  normalisation;
- locale-dependent case rules such as Turkish `İ`/`i` and German `ß`/`SS`.

**Write that limitation where a user can find it.** An undocumented gap in a security guarantee is the
thing this increment exists to stop.

### 3b. Existing repositories: reject at creation only

**Ruled: do not retroactively validate.** prikk has no production use, is pre-1.0 with an explicitly
unstable format and no migration promise, is not yet self-hosted, and has been on crates.io for five days.
There is no repository whose verifiability is worth protecting here.

**This collapses criterion 4 from a design question into a recorded decision, and it is what makes this
increment small.**

**Still record it**: state plainly that pre-existing collisions are not detected and why. When prikk does
have real repositories, nobody should assume the check was always there.

## 4. What §2's table is still for



**What is the collision rule?** ASCII case-folding, Unicode case-folding, or normalisation as well — `á`
(one codepoint) and `á` (a + combining acute) also collide on macOS. **ASCII-only is a defensible answer;
an unstated answer is not.** Write it where a user can find it, not only in code.

**What happens to repositories that already contain a collision?** A rule that makes existing history
unverifiable is a format break, not a fix. It must fail closed at **creation** without retroactively
condemning what exists — or that consequence is stated and accepted. This is the one that decides whether
the increment is small.

## 5. Why it matters more this week than last

On a case-insensitive filesystem — macOS default, Windows — two paths prikk treats as distinct nodes
collide in the worktree. One silently overwrites the other on checkout, and **`verify` cannot see it**:
at the object level the history is entirely consistent. The damage is in the working tree, which is
precisely where prikk's guarantees stop.

**DC-71 made read-only commands run on exactly those platforms yesterday.**

## 6. Traps

- **Fixing the case clause and calling NFR-SEC-03 met.** It is one line covering five properties.
  Criterion 5: anything else found unmet is **reported**, and fixed here only if trivial.
- **Treating §3a as permission to stop thinking.** ASCII folding is ruled, but the *limitation* it leaves must be written down, per §3a.
- **Adding retroactive detection anyway** because it seems more thorough. §3b ruled against it.
- **Treating this as a filesystem-compatibility feature.** It is about rejecting collisions, not tolerating
  them.
- **Trusting `MILESTONES.md`'s description of the gap.** It was already wrong once; see §2.

## 7. Definition of done

§2's per-clause, per-surface table reported before design; the collision rule stated with its rationale
somewhere user-visible; collisions rejected at creation on every surface, tested per surface; existing
repositories still verifiable or the consequence explicitly accepted; any other unmet clause reported;
`MILESTONES.md`'s NFR-SEC-03 row updated to its resolved or narrowed state; full gate set per
`rfcs/EXECUTION-ORDER.md` §6 rule 9 with test counts before and after, **commands verbatim**.

## 8. Standing request

Three defects this week were found by running a sequence rather than reading code, and one requirement's
recorded description was wrong in a way one grep exposed. **The table in §2 is that grep, done
systematically.** If it contradicts anything here — including my claim that no collision rejection exists —
stop and report it.
