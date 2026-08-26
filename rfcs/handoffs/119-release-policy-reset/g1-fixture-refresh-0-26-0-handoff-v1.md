# G1 — refresh the compatibility fixture to `0.26.0`

**Base:** current `main` (`b4fbe66`, `0.26.0` released). **Under `003-landing-work-on-main.md`.**
**Independent of the install-page handoff issued alongside this one** — either order is fine, they
touch nothing in common.

**This is the third refresh. Two things have changed since the last one; read §1 and §2.**

---

## 1. Most of the work is now derived

`LAST_RELEASE_FIXTURE_VERSION` exists (`"0.25.0"` today) and **the fixture path derives from it**. So
the code change is **one constant**, not a path edit plus a constant. That was built last increment
precisely so this refresh would not need two edits kept in sync.

**`DECLARED_BREAKS` is empty and version-scoped.** Moving the constant to `"0.26.0"` would make any
entry whose `older_version` is `"0.25.0"` **fail** `every_declared_break_applies_to_the_current_fixture`
— by design. The list is empty, so nothing should fire. **If something does, stop and report it.**

## 2. The provenance hazard, for the second consecutive refresh

**No format or payload change between `0.25.0` and `0.26.0`** — I diffed `format.rs` and
`crates/prikk-object/src/payload/`: **zero lines.** So the schema arrays will not move, and **nothing
in the test suite can distinguish a genuine rebuild from changing one constant and renaming a
directory.**

**Provenance is the entire deliverable.** Detached worktree at the `0.26.0` tag,
`cargo build --locked -p prikk`, confirm `prikk --version` prints `0.26.0`, construct with **that
binary and no other**. **Quote the version output and the worktree commit** — they are the only
evidence.

**You established last time that the fixtures cannot be byte-identical anyway**: `node_id_gen.rs`
mints every `NodeId` from the OS CSPRNG, so `Patch` payloads differ across independent builds
regardless. **A `diff -r` against the outgoing fixture should therefore differ. If it does not, that is
a finding** — it would mean the rebuild did not happen.

## 3. The `*.log` hazard is a named step now, not a warning

The `.gitignore` `*.log` rule has silently excluded the same four empty generation-log files from
**all three** previous fixtures:

```
containers/generations.log
refs/containers/pointer-index-generation.log
refs/containers/received-index-generation.log
trust/policy-generation.log
```

**Three for three is a pattern.** Compare `find <fixture> -type f | wc -l` against
`git status --short | wc -l` before staging, and `git add -f` the four. **Report both counts.**

## 4. The refresh

- **Replace, do not accumulate.** Delete the `0.25.0` fixture in the same commit.
- **Match the previous coverage**: six of seven persisted types, `Attestation` absent because no
  production path constructs one. **Report the coverage table.**
- **Re-derive the schema arrays from a probe** against the real new fixture. **Do not copy them from
  the committed expectations** — if yours differ, **yours are right and that is the finding.**

## 5. No `DECLARED_BREAKS` entry is owed

**`0.26.0` shipped zero public API changes and zero format changes.** Nothing to declare. **Do not add
an entry**, and do not repopulate the list that was deliberately emptied.

## 6. Out of scope

- **G1's shape** beyond the constant, the fixture, and its expectations.
- **The install-page handoff** issued alongside this.
- **Any product behaviour.**

## 7. Controls

1. **The fixture is genuinely `0.26.0`-vintage** — process claim, with `prikk --version` and the
   worktree commit quoted.
2. **Old-vs-new differ** (§2), with the count of differing files.
3. **The gate fires on an undeclared break** — a reverted code-path mutation on a type **none of the
   three previous refreshes used**. `Block` and `Patch` are spent; `Tag` is now a viable site again,
   since the stale entry that would have absorbed it was retired. Quote the failure.
4. **Coverage remains load-bearing** — the committed expectations notice a shrunken or reschema'd
   fixture.
5. **The gate passes unmodified**, full suite green, count unmoved.

**Quote every failure.**

## 8. What to report

1. **Provenance evidence** and the **coverage table**.
2. **Both file counts** from §3, and which four were excluded.
3. **Your re-derived schema arrays**, and whether they match the committed ones.
4. All five controls (§7), quoted.
5. **Full gate set against the exact commit, after the last edit.**
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: the `0.26.0` fixture fails the gate immediately — **that would
mean current `main` cannot read what we shipped hours ago, and it outranks everything.**
