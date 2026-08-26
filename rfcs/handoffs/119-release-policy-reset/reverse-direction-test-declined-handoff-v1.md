# G1 — the reverse-direction test is declined, on the record

**Base:** current `main` (`8829078`). **Under `003-landing-work-on-main.md`.**
**Origin:** every G1 fixture refresh has re-raised this, and I owed a decision instead of another
deferral. **This is the decision.**

---

## 1. The ruling

**prikk will not build a reverse-direction compatibility test.** G1 checks the **forward** direction
only — current code reading a last-release fixture — and that is the whole of the guarantee.

**Six grounds, in the order that decides it:**

1. **Forward is the direction that protects users.** Users upgrade. G1 covers it, gated, with a
   fixture refreshed each release.
2. **The reverse direction is deliberately broken by design when a schema bumps.** `0.24.0` did
   exactly that: a repository written by `0.24.0` cannot be read by `0.23.0`. **A test asserting the
   reverse holds would assert a promise this project does not make.**
3. **A test expected to fail on every deliberate format change trains its own allowlist to grow
   unexamined.** That is "recorded, not rejected" — the pattern already rejected here as *the very
   technical debt*.
4. **The failure mode is fail-closed, and I verified it rather than assuming it.**
   `format.rs::validate_format2_schema` refuses an unadmitted schema with an `Integrity` error naming
   the object type, the schema found, and the accepted set. **An old binary refuses; it does not
   misread.** So an *undeclared* reverse break surfaces as a loud, specific refusal — never silent
   corruption.
5. **What users need already exists**: the per-release reverse-break statement in `CHANGELOG.md`,
   which the release handoff now requires.
6. **Cost is real**: building and caching an old binary per platform across three targets, in CI, for
   the direction that matters least.

**What this gives up, stated plainly:** an **accidental** reverse break goes undetected until someone
downgrades. The consequence is a refusal with a clear error, recoverable by upgrading again.
**Acceptable at pre-1.0. Say so in the record — do not omit it.**

## 2. Record it where the question keeps arising

`crates/prikk-store/src/release_compatibility_gate.rs`'s **module doc** already explains that the gate
is forward-only. **It does not say the reverse direction was decided against** — which is why every
refresh re-opens it.

**Add the ruling there**: the decision, the six grounds compressed to their essentials, what is given
up, and the revisit trigger (§3). **A future reader must be able to tell "decided against" from "not
yet built."** That distinction is the entire point.

**Keep it proportionate** — this is a doc comment, not an essay. The reasoning belongs in it; the
history does not.

## 3. The revisit trigger, which must be stated

**This ruling is void if either becomes true:**

- **prikk supports downgrade as a documented workflow**, or
- **a format change is made that does not fail closed** — i.e. old code could misread new data rather
  than refuse it.

**The second is the one that matters.** Ground 4 is load-bearing: if a future change makes an old
binary *misinterpret* new bytes instead of refusing them, this decision no longer holds and the test
becomes necessary. **Say that explicitly**, so the trigger is checkable rather than a vague "revisit
someday."

## 4. Out of scope

- **Building any part of the reverse test**, including scaffolding "for later".
- **Changing G1's forward behaviour**, its fixture, or `DECLARED_BREAKS`.
- **The `CHANGELOG.md` reverse-break convention**, which stays as it is.
- Any other module's documentation.

## 5. Controls

1. **The claim in ground 4 is true** — show that an unadmitted schema is refused, with the actual
   error text quoted from a real call, not from reading the source.
2. **The gate still passes unmodified**, and the suite is green with **no count change** — this is a
   doc-only edit and the number must not move.

**If the count moves, something other than a doc comment changed. Stop and say so.**

## 6. What to report

1. The ruling as written into the module doc.
2. **Control 1's quoted error**, from a real invocation.
3. **Full gate set against the exact commit, after the last edit.**
4. Every numbered requirement's disposition.
5. Anything here was wrong — **including the ruling itself.** If ground 4 does not hold on some path,
   **that is a finding that reverses this decision**, and I would rather have it now.

**Stop and escalate, do not guess**, if: you find a path where an old binary would misread rather than
refuse. **That voids §1 outright.**
