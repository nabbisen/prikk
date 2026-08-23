# Staleness by omission: implementation handoff

**Base:** current `main` (`ed5c927`).
**Origin:** the CLI output currency increment. `main.rs`'s command inventory lists ~15 capabilities and
never mentions `bundle`, `sync`, `merge` or `tag` — and **matched no scope command, because it makes no
false claim. It simply omits.**

**This needs a different method from the three currency passes before it.** Those hunted false
assertions — *"X is not implemented"* where X shipped — and every one was findable by grep. **An
inventory that silently fails to mention a shipped command has no phrase to search for.** The direction
of travel has to reverse: **start from what exists, and ask which surfaces fail to mention it.**

---

## 1. Ground truth — the dispatch table, not a document

```sh
git show HEAD:crates/prikk-cli/src/main.rs | grep -oE 'Some\("[a-z-]+"\)'
```

**24 commands.** That match block is what a user can actually invoke, so it is the only inventory that
cannot itself be stale. **Every other list in this project is a claim about it.**

## 2. The surfaces to check, and which are meant to be exhaustive

| Surface | Exhaustive? |
|---|---|
| `crates/prikk-cli/src/output/help.rs` — `--help` | **YES. Ruled.** |
| `crates/prikk-cli/src/main.rs` module doc | **YES** — it purports to say what the CLI exposes |
| `README.md` | **No** — a summary; major capabilities only |
| `docs/src/index.md` | **No** — an overview |
| `docs/src/SUMMARY.md` / guide pages | **No** — not every command warrants a page |

**`--help` must list every dispatched command.** A command a user cannot discover is, for practical
purposes, unshipped. This is the one absence that is a defect on its face rather than a judgment call.

**On the non-exhaustive surfaces, absence is a finding to report, not automatically a defect.** Say what
is missing and let the pattern speak; do not start writing pages.

## 3. The deliverable: a matrix, then verdicts

**A table of 24 commands × 5 surfaces, present or absent in each.** That is the artefact — it is what
makes an omission sweep checkable, the same way the enumeration did for the docs sweep.

Then, for every absence:

- **`GAP`** — an exhaustive surface (§2) is missing a command. **Fix it.**
- **`EXPECTED`** — a non-exhaustive surface does not mention it, reasonably. **Leave it; say why.**
- **`REPORT`** — a non-exhaustive surface's omission that still looks wrong. **Do not fix; describe it.**

## 4. One confirmed `GAP`, to calibrate — not to bound

**`bundle` appears in `help.rs` zero times**, while `main.rs:88` dispatches it. Verified. It also has no
guide page and no `SUMMARY.md` entry, so **a user has no way to discover `prikk bundle` exists** —
despite DC-78, a full test suite, and the closure validation increment.

**Check whether the omission is deliberate before fixing it.** If there is a comment, an RFC, or any
record saying `bundle` is intentionally undocumented — internal, deprecated, superseded by `sync` —
**stop and escalate.** Do not "fix" a deliberate hiding into an accidental exposure. **I found no such
record, but I looked from the outside; you will be reading the file.**

Note the plausible story: `sync` supersedes `bundle` for exchange. **Plausible is not recorded.** If
that is the intent, it belongs written down, not inferred by the next person from an absence.

## 5. Fix scope — narrow, deliberately

**In scope:** `help.rs` and `main.rs`'s module doc. Text only.

**Out of scope:** writing guide pages, adding `SUMMARY.md` entries, editing `README.md` or
`docs/src/index.md`. **Those are judgment about what deserves documenting, which is a different
decision from "the help text is incomplete."** Report them; do not act on them.

**Also out of scope:** the module doc's *other* staleness — it calls `seal` "a local no-audit seal
scaffold". That is a false-claim problem, the earlier sweeps' territory, and correcting it here would
mix two methods in one diff. **Report it.**

## 6. What to report

1. **The 24 × 5 matrix.** The deliverable.
2. **Every absence with its verdict** and a one-line reason.
3. **For each `GAP` fixed:** the actual `prikk --help` output afterwards, pasted. **Read it.** Three
   increments running, reading real output has found the next defect.
4. **Whether `bundle`'s omission is recorded anywhere as deliberate** (§4) — the answer matters either
   way.
5. The **full gate set against the exact commit, after the last edit** — the standard nine.
6. Test counts before and after, and whether any test asserts help output.
7. Anything here that turned out to be wrong. **Say so plainly**, including my count of 24.

**Stop and escalate, do not guess**, if: `bundle`'s omission turns out to be deliberate (§4); a command
in the dispatch table is not meant to be user-facing at all — **`--version` is in that match block and
is not a command**, so expect at least one such judgment; or the matrix turns up an absence pattern that
suggests a decision was made and never written down.
