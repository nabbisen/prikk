# G1 — refresh the compatibility fixture to `0.27.0`

**Base:** current `main` (`0fbacbb`, `0.27.0` released and published). **Under
`003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This is the fourth refresh.** §5 and §6 are the two things that differ from last time; read them
before starting.

---

## 1. Most of the work is derived, as designed

`LAST_RELEASE_FIXTURE_VERSION` is `"0.26.0"` and **the fixture path derives from it**
(`.replace('.', "_")` → `crates/prikk-cli/tests/fixtures/rfc119_g1_<v>_repo`). **The code change is one
constant.**

`DECLARED_BREAKS` is empty and version-scoped: moving the constant to `"0.27.0"` makes any entry whose
`older_version` is `"0.26.0"` fail `every_declared_break_applies_to_the_current_fixture`, by design.
**The list is empty, so nothing should fire. If something does, stop and report.**

## 2. Provenance is the entire deliverable, for the third consecutive refresh

**I diffed `format.rs` and `crates/prikk-object/` across `0.26.0..0.27.0`: zero lines.** No format
change, no payload change, no object-encoding change at all.

**So the schema arrays will not move, and nothing in the test suite can distinguish a genuine rebuild
from editing one constant and renaming a directory.**

Detached worktree at the **`0.27.0` tag**, `cargo build --locked -p prikk`, confirm `prikk --version`
prints `0.27.0`, construct the fixture with **that binary and no other**. **Quote the version output
and the worktree commit** — they are the only evidence that exists.

**The fixtures cannot be byte-identical**, established two refreshes ago: `node_id_gen.rs` mints every
`NodeId` from the OS CSPRNG, so `Patch` payloads differ across independent builds regardless. **A
`diff -r` against the outgoing fixture must differ. If it does not, the rebuild did not happen** —
that is a finding, not a convenience.

## 3. The `*.log` hazard — four for four if it recurs

`.gitignore`'s `*.log` rule silently excluded the same four empty generation logs from **three**
fixtures before the last refresh caught them:

```
containers/generations.log
refs/containers/pointer-index-generation.log
refs/containers/received-index-generation.log
trust/policy-generation.log
```

**The outgoing `0.26.0` fixture is complete — 34 files on disk, 34 tracked, all four logs present and
tracked.** That is what a correct result looks like. **Compare `find <fixture> -type f | wc -l`
against the tracked count before staging, `git add -f` the four, and report both numbers.**

## 4. The refresh

- **Replace, do not accumulate.** Delete the `0.26.0` fixture in the same commit.
- **Match the previous coverage**: six of seven persisted types, `Attestation` absent because no
  production path constructs one. **Report the coverage table.**
- **Re-derive the schema arrays from a probe** against the real new fixture. **Do not copy them from
  the committed expectations** — if yours differ, yours are right and that is the finding.

## 5. Still no `DECLARED_BREAKS` entry — and this is the first refresh where that needs explaining

The three previous refreshes followed releases with no breaking changes of any kind. **`0.27.0` is
different: it shipped a breaking library API change.** `DoctorIssue` gained `active_session` and
`DoctorRepairReport` gained `active_repairs`; both are all-public-field structs with no
`#[non_exhaustive]`, so struct-literal construction breaks. It is recorded in `CHANGELOG.md`.

**No `DECLARED_BREAKS` entry is owed anyway, and the reason is the distinction that matters here:**
this gate is about **persisted object decode contracts** — whether current code can read an older
release's *on-disk objects*. **A Rust API break is not a decode break.** Zero format and payload lines
changed, so nothing on disk moved.

**Do not add an entry, and do not repopulate the list that was deliberately emptied.** If you conclude
otherwise, **stop and report rather than adding one** — a declared break the fixture cannot produce
makes the gate look for a failure that will never come.

## 6. Control 3 is running out of sites, and this was flagged for you

The last refresh's review recorded:

> **`Block`, `Patch`, and now `Tag` are spent. The next refresh needs `RefState`, `Blob`, or
> `RecognitionClaim`.**

**Use one of those three.** Control 3 proves the gate fires on an undeclared break by mutating a
decode path and observing the failure, then reverting — and re-using a spent type would prove the
gate still fails where it has already been shown to fail, which is not the same claim.

**Say which you chose and why**, and **record which remain** for the fifth refresh. **If you conclude
all three are unsuitable, stop and report** — a control with no remaining site is a real problem about
this gate's testability, not something to improvise around.

## 7. What must not change

- **G1's shape** beyond the constant, the fixture, and its committed expectations.
- **The gate itself must not be modified to accommodate the new fixture.** If it needs changing, the
  fixture is wrong or the gate has a defect — either way, stop and report.
- **Any product behaviour.**

## 8. Controls

1. **The fixture is genuinely `0.27.0`-vintage** — process claim, `prikk --version` and the worktree
   commit quoted.
2. **Old-vs-new differ** (§2), with the count of differing files.
3. **The gate fires on an undeclared break** — §6's site. **Quote the failure**, then revert.
4. **Coverage remains load-bearing** — the committed expectations notice a shrunken or reschema'd
   fixture.
5. **The gate passes unmodified**, full suite green, count unmoved.
6. **Full gate set against the exact final commit**, after the last edit.

**Quote every failure.**

## 9. What to report

1. **Provenance evidence** and the **coverage table**.
2. **Both file counts** from §3, and which four were force-added.
3. **Your re-derived schema arrays**, and whether they match the committed ones.
4. **§6's chosen site, the reasoning, and which remain.**
5. All six controls (§8), quoted.
6. **Every numbered requirement's disposition**, including the ones that went without incident.
7. **Anything in this handoff that was wrong.**

**Stop and escalate, do not guess, if the `0.27.0` fixture fails the gate immediately** — that would
mean current `main` cannot read what we published today, and it outranks everything else here.
