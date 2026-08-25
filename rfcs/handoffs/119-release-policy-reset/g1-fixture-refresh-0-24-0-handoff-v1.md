# G1 — refresh the compatibility fixture to `0.24.0`

**Base:** current `main`. **Under `003-landing-work-on-main.md`.**
**Origin:** the debt the `0.24.0` cut created, recorded in
`.git-exclude/reviewed/release-0-24-0-changelog-review-v1.md` §5.

**G1 currently compares against `0.23.0`, which is superseded.** Every release that passes without a
refresh lets the gate check an older baseline.

**Read §2 before touching the declared-breaks list. The obvious edit there is wrong.**

---

## 1. The refresh

**Build a `0.24.0`-vintage repository fixture**, the same way `rfc119_g1_0_23_0_repo` was built:

- a **detached worktree at the `0.24.0` tag**, `cargo build --locked -p prikk`, confirm
  `prikk --version` prints `0.24.0`;
- construct the repository with **that binary only** — `init`, `commit`, `trust maintainer add`,
  `seal`, `tag create`, and a real two-repository `sync have`/`build`/`accept` to obtain a
  `RecognitionClaim`;
- copy `.prikk` verbatim; **do not regenerate with current code.**

**Match `0.23.0`'s coverage or better**: six of seven persisted types, `Attestation` absent because no
production path constructs one. **Report the coverage table**, and the committed-counts assertion must
move with it.

**`0.24.0` writes `Patch` at schema 2**, so the new fixture exercises a type/schema pair the old one
could not. **Say so** — it is the first fixture to cover schema 2.

## 2. The declared-breaks list — do NOT simply append `0.23.0 -> 0.24.0`

**`DeclaredBreak` has no direction field**, and its doc reads *"Which persisted object type stops
decoding"* — **implicitly the forward direction**, which is the only one G1 checks: newer code reading
older data.

**`0.24.0`'s break is the other direction.** `0.24.0` reads `0.23.0` fine; **`0.23.0` cannot read
`0.24.0`.** Appending it as an ordinary entry would **assert a forward break that does not exist**, and
the gate would then look for a failure the fixture will never produce.

**Two honest resolutions. Pick one, say which, and say why:**

- **(a) Keep the list forward-only** and make that explicit in its name or doc — the reverse break stays
  recorded in `CHANGELOG.md`, where it already is. **Simplest, and matches what G1 actually checks.**
- **(b) Add a direction field** and record both kinds, accepting that reverse entries are documentation
  rather than gate inputs until the reverse direction is testable.

**What is not acceptable is leaving the list looking like it holds all declared breaks when it holds
one direction's.** That is the unbound-claim defect this project has spent a month removing.

## 3. Replace, or accumulate? — adjudicate, do not assume

**Today the gate holds one fixture, from the last release.** My track C handoff specified that, and I did
not examine the alternative.

**Transitivity does not hold**: `0.25.0` reading `0.24.0`, and `0.24.0` reading `0.23.0`, does **not**
guarantee `0.25.0` reads `0.23.0`.

**Argument for replacing** (status quo): pre-1.0, no production users, and G1's pre-1.0 form is *declare*
rather than *prevent* — one baseline is proportionate.

**Argument for accumulating**: it catches a break against any retained release, not just the last.

**Adjudicate and report.** **If you replace, say what is thereby no longer checked.** If the answer needs
an owner ruling, **stop and say so** — this is a scope question, not an implementation detail.

## 4. Out of scope

- **The reverse direction as a *test*.** Still needs an old binary; still deferred.
- **Changing G1's shape** beyond §2 and §3.
- **Deleting the `0.23.0` fixture** before §3 is decided.
- **Any product behaviour.**

## 5. Controls

1. **The new fixture is genuinely `0.24.0`-vintage** — say how you established it, as the `0.23.0` one
   did (a process claim, not a runtime-testable one).
2. **The gate fires on an undeclared break** against the new fixture — the reverted code-path mutation
   the `0.23.0` fixture's controls used.
3. **Coverage remains load-bearing** — the committed-counts assertion notices a shrunken fixture.
4. **The gate passes unmodified.**

**Quote every failure.**

## 6. What to report

1. **The fixture**, its provenance, and the **coverage table** (§1).
2. **Your §2 choice**, with reasoning — this is the substantive decision.
3. **Your §3 adjudication**, and what is no longer checked if you replace.
4. **All four controls** (§5).
5. **Full gate set against the exact commit, after the last edit.** Test counts may move.
6. Anything here that was wrong.

**Stop and escalate, do not guess**, if: §3 needs an owner ruling; the `0.24.0` fixture cannot reach
`0.23.0`'s coverage; or **the new fixture fails the gate immediately** — that would mean current `main`
already breaks `0.24.0`, which is a live defect and stops everything else.
