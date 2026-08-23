# CLI output currency: implementation handoff

**Base:** current `main` (`9d2901f`). **Code and its test assertions; no behaviour change beyond the
text users read.**
**Origin:** found by the `merge-plan` fix's own manual check — reading real CLI output turned up the
next instance of the same defect immediately.

**Why this class needs its own pass:** a stale *documentation* claim is found by reading docs. **A stale
*CLI string* is found only by running the program** — a test suite asserts it faithfully forever. The
`merge-plan` string survived two increments behind three passing tests for exactly that reason.

---

## 1. Scope — this command, and the count it actually produces

```sh
grep -rnE "remain (later|separate)|not implemented|not yet|unimplemented" crates/prikk-cli/src/
```

**8 lines.** I have now mis-stated a scope count twice — once by publishing a different command than
the one I counted with, once by asserting "two" from memory in the previous review. **This number came
from running the command above, immediately before writing this line.** If your run differs, say so
before proceeding.

The eight:

| # | Site | Note |
|---|---|---|
| 1 | `output.rs:96` | replay note — *"ReplaceBinary, renames, conflicts, and full patch algebra remain later increments"* |
| 2 | `output.rs:198` | *"rollback refs, authorization, conflicts, and full patch algebra remain later PRs"* |
| 3 | `output.rs:255` | *"rollback refs, authorization, worktree writes, and full patch algebra remain later PRs"* |
| 4 | `output.rs:287` | *"rollback refs, audit policy, and worktree writes remain later PRs"* |
| 5 | `output.rs:311` | *"seal, rollback refs, authorization, audit policy, and worktree writes remain separate"* |
| 6 | `main.rs:12` | module doc mirroring #7 |
| 7 | `main.rs:154` | **confirmed STALE** — commit note, printed on **every commit**: *"…audit plugins, and **sync** remain later increments"* |
| 8 | `main.rs:167` | *"audit plugins and patch-based worktree materialization remain later PRs"* |

## 2. Verdicts, per item, as the docs sweep did

**`STALE`** — corrected. **`CURRENT`** — left, with what makes it still true. There is no `TERM`
category here; these are all claims.

**Each note lists several features, and they must be adjudicated term by term, not note by note.** #7 is
the shape to expect: *"multi-operation text diff minimization, patch algebra, rename detection, audit
plugins, and sync"* — **sync has shipped and the rest have not**, so the note is neither wholly stale
nor wholly current. **A note is corrected by removing the shipped terms, not by rewriting the sentence.**

Only one is confirmed for you: **#7's `sync`.** Everything else you adjudicate.

**Watch for `patch algebra` in particular** (#1, #2, #3): `patch_algebra` exists as an internal
classifier and `merge-evidence`/`merge-plan` report from it, but *"full patch algebra"* in these notes
means something narrower about replay and materialization. **Read what the surrounding command actually
does before deciding** — this is the same `STALE`/`CURRENT` discrimination that made the docs sweep
non-trivial, and here the plausible-looking neighbours are terms inside one sentence.

## 3. "Later PRs" is itself a currency signal

Five of the eight say **"remain later PRs"**. This project has spoken in DC-numbers and RFCs for its
whole recorded history; *"PRs"* is vocabulary from before that. **It is evidence these lines have not
been read since they were written**, which is the finding, not a style nit.

**Do not do a vocabulary pass.** If a note survives as `CURRENT`, leave its wording alone — rewording
correct text inflates the diff and buries the corrections. **Mention it in your report; change nothing
for it.**

## 4. The test assertions

Some of these strings are asserted by tests, as `merge-plan`'s were. **Find them before editing** —
`grep` the tree for each string you intend to change.

**Tests that assert changed text must change**, exactly as in the `merge-plan` fix, and for the same
reason: this is a deliberate user-visible output change, so an unchanged assertion would mean the
output did not change. **Report how many you found and edited.**

## 5. The manual check, which is the point of this increment

**For every note you correct, run the command that prints it and paste the real output line.**

Not the test's expected string — **the program's actual stdout.** The defect class exists precisely
because assertions and prose can both be maintained while nobody runs the thing. An increment that
fixes stale output without reading output would be self-defeating.

Say which commands you ran and what they printed.

## 6. Out of scope

- **`prikk-store`'s two `println!`s** — different crate, and one is a benchmark harness. If either
  looks stale, **report it, do not fix it here.**
- **Module docs other than `main.rs:12`**, which is in scope only because it mirrors #7 in the same
  file.
- **Any behaviour change.** Only the text.
- **Rewording `CURRENT` notes** (§3).

## 7. What to report

1. **The eight lines, each with a verdict and a one-line reason** — the enumeration is the deliverable,
   as it was for the docs sweep. Terms adjudicated individually where a note lists several.
2. **The manual-check output** (§5) — actual stdout per corrected note.
3. **Test assertions found and edited** (§4).
4. **Re-run §1's command afterwards** and confirm every remaining hit is one you adjudicated `CURRENT`.
5. The **full gate set against the exact commit, after the last edit** — the standard nine.
6. Test counts before and after — **expected unchanged**; assertions edited, none added or removed.
7. Anything here that turned out to be wrong. **Say so plainly**, including my count of 8.

**Stop and escalate, do not guess**, if: a note is stale because the **feature** is half-shipped and you
cannot say cleanly whether it exists; correcting a string requires touching logic rather than text; or
the scope count differs from 8.
