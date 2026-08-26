# G1 — refresh the compatibility fixture to `0.25.0`

**Base:** current `main` (`f6963c3`). **Under `003-landing-work-on-main.md`.**
**Origin:** the recurring obligation each cut creates. `0.25.0` shipped; the fixture is still
`0.24.0`.

**Read §1 before starting. A sibling increment is outstanding on the same file.**

---

## 1. Another increment is pending on `release_compatibility_gate.rs`

`rfcs/handoffs/119-release-policy-reset/reverse-direction-test-declined-handoff-v1.md` (issued at
`01fd32f`) records the ruling that **the reverse-direction test is declined**, and writes it into this
same module's doc comment. **It has not landed** — I checked: the module carries no decline language.

**If it lands before you start, do not revert it.** Rebase onto it and say so. **If it is still
outstanding when you finish, say that too**, so whoever takes it second knows the file moved.

## 2. The hazard this refresh has and the last one did not

**`admitted_schemas` has not changed since `0.24.0`.** I diffed it. So a `0.25.0`-built fixture should
carry **exactly the same schema versions** the committed expectations already pin:

```
Patch [2, 2]   Block [2]   Blob [1, 1]   RefState [1, 1]   Tag [1]   RecognitionClaim [1]   Attestation []
```

**Last time the schemas moved** (`Patch` 1 → 2), so the arrays changed and gave incidental evidence
that the fixture was genuinely rebuilt. **This time nothing in the test suite can tell a real refresh
from renaming the directory and editing one path constant.** Every test would pass either way.

**So provenance is the entire deliverable, and it is unverifiable from the tests.** Build it properly:
a detached worktree at the `0.25.0` tag, `cargo build --locked -p prikk`, confirm `prikk --version`
prints `0.25.0`, and construct the repository with **that binary and no other**.

**Report the version output and the worktree commit.** They are the only evidence there is.

## 3. Determine whether the new fixture differs from the old at all

**I do not know the answer and you should not assume one.**

If prikk's object identities are fully content-addressed with no timestamp in any preimage, and the
harness uses a fixed key seed and fixed file contents, **a `0.25.0` fixture could be byte-identical to
the `0.24.0` one.** If any object carries a timestamp, it will differ.

**Find out, and report which.** `diff -r` the old fixture against the new one before deleting the old.

- **If they differ**, that difference is a real provenance signal — **say where it comes from.**
- **If they are byte-identical**, that is a finding worth stating plainly, not a problem to hide: it
  would mean this gate cannot distinguish a rebuilt fixture from a renamed one **by any mechanical
  means**, and that the process claim is all there is. **Do not fabricate a difference.**

## 4. The refresh itself

- **Replace, do not accumulate** — the settled ruling. Delete `rfc119_g1_0_24_0_repo`; the gate holds
  one fixture, from the last release.
- **Match the previous coverage or better**: six of seven persisted types, `Attestation` absent because
  no production path constructs one. **Report the coverage table.**
- **Re-derive the schema arrays from a probe against the real fixture**, as last time — **do not copy
  the numbers from §2.** I put them there so you can compare, not transcribe. If they differ from mine,
  **yours are right and mine are the finding.**
- **Watch the `.gitignore` `*.log` hazard.** It silently excluded four empty generation-log files from
  both previous fixtures. Compare `find | wc -l` against `git status --short | wc -l` before staging.

## 5. No `DECLARED_BREAKS` entry is owed

**`0.25.0`'s three breaking changes are API-only** — `ObjectType::ProjectGenesis` removed,
`has_blocking_defect()` removed, `MergeEvidenceDisplayItem` gained three fields. **No repository
written by any prior release became unreadable.** `DECLARED_BREAKS` is forward-direction persisted-object
decoding; none of these touch it.

**Do not add an entry**, and **do not remove the existing `0.22.1 -> 0.23.0` one**, which remains the
historical record.

## 6. Out of scope

- **The reverse-direction ruling** (§1), which is its own increment.
- **G1's shape** beyond the fixture and its expectations.
- **`DECLARED_BREAKS`** (§5).
- **Any product behaviour.**

## 7. Controls

1. **The fixture is genuinely `0.25.0`-vintage** — the process claim, with `prikk --version` output and
   the worktree commit quoted.
2. **Old-vs-new comparison** (§3), with the answer either way.
3. **The gate fires on an undeclared break** — a reverted code-path mutation, and **not the one the
   previous two refreshes used**. Quote it.
4. **Coverage remains load-bearing** — the committed expectations notice a shrunken or reschema'd
   fixture.
5. **The gate passes unmodified**, and the full suite is green. Say whether the count moved and why.

**Quote every failure.** After any control that deliberately fails a property test, **check
`proptest-regressions/`**.

## 8. What to report

1. **Provenance evidence** (§2) and the **coverage table**.
2. **Your §3 answer** — identical or different, and why.
3. **Your re-derived schema arrays**, and whether they match mine.
4. **Whether the reverse-direction increment had landed** when you started (§1).
5. All five controls (§7), quoted.
6. **Full gate set against the exact commit, after the last edit.**
7. **Every numbered requirement's disposition, including ones that went without incident.**
8. Anything here was wrong.

**Stop and escalate, do not guess**, if: the `0.25.0` fixture fails the gate immediately — **that would
mean current `main` already breaks the release we shipped, and it outranks everything else.**
