# Amendment to `ignore-mechanism-handoff-v1.md` — the mechanism was reverted off `main`

**Commit one (`de5a8c1`, the README disclosure) stands and is on `main`.**
**Commit two (`b9a6fd8`, the mechanism) was reverted at `2235af3`.** It broke `commit` outright on
Windows.

**Two things went wrong, and the first one is mine.**

---

## 1. What I did wrong, stated first

**I pushed `de5a8c1` and `b9a6fd8` to `main` without reviewing them and before your report reached
me.** I was working on an unrelated README change, ran my pre-push check, and pushed — and my check
only asked whether I was *behind* `origin/main`, not what I was *ahead* by. Earlier increments this
week printed the ahead-list before pushing, which would have shown two commits I had not read. I had
dropped that step.

**So the CI failure landed on `main` rather than being caught in review, and that is my error, not
yours.** Your work was committed locally with a report written, exactly as the workflow requires.

I reverted the mechanism to get `main` green (15/15) and kept the disclosure, which is correct,
independent, and was commit one for exactly this reason.

## 2. The defect

```
Windows mutation test suite → -p prikk --test rfc124_ignore_mechanism
error: invalid name: backslashes are not allowed in repository paths
```

Six of the eight new tests fail this way, and **`commit` fails outright on Windows whenever the walk
descends into a subdirectory** — not only when a `.prikkignore` exists.

**Cause:** `worktree_files.rs`'s new `walk_dir` does

```rust
if let Some(rel) = path.to_str() { … }
```

`Path::to_str()` renders the platform separator, so on Windows `rel` is `dir\file`. That string is a
**repository** path, and `validate_repo_path` rejects backslashes — correctly, and by RFC 125's own
grammar, which this project tightened two days ago.

## 3. The fix already exists, in one of the two files you were binding into

`worktree_status.rs:274`:

```rust
fn pathbuf_to_slash_string(path: &Path) -> Result<String> {
    // walks path.components(), rejects non-UTF-8, joins with "/"
}
```

**That is the separator-independent converter, and it is the third path-building shape this crate
would otherwise have.** Share it — move it somewhere both walks can call, or have the ignore layer
take an already-converted repo-path string rather than a `Path`. **Do not add a fourth.**

**Check the other new `Path`→string conversions in the reverted diff by the same standard**, not just
the one the test caught: any place the mechanism turns a filesystem path into something compared
against a repo path has this bug shape.

## 4. The reasoning to retire, because it is the one that let this through

Your report said:

> Cross-target clippy: not run — no `#[cfg(target_os)]`/`cfg(unix)`/`cfg(windows)` in this diff

**That heuristic is wrong here twice over.** The absence of a platform `cfg` says nothing about
portability when the code **builds paths from filesystem text** — that is precisely the case this
project has been bitten by before. And cross-target clippy would not have caught it anyway: it
*compiles*, it does not *run*. This is a runtime defect, and the only thing that surfaces it is the
Windows job actually executing the code.

**So the rule is not "run cross-target clippy too."** It is: **when a diff converts an OS path to a
repository path, that conversion is a platform surface regardless of what `cfg`s appear.**

## 5. Required — a control you can run without a Windows machine

You cannot run the Windows suite, and neither can I before pushing. **So the invariant has to be
testable on Linux:**

**A unit test asserting that every repo-path string the walk produces contains no `\`** — driven by
constructing paths through the same converter, not by walking a real filesystem. `pathbuf_to_slash_string`
already has the right shape to test directly. A test that only exercises a real Linux worktree
cannot fail on this bug and is not evidence.

**State in your report which construction sites you audited**, not only that the tests pass.

## 6. Everything else in v1 still applies

§3's one-derivation requirement, §4's five design questions and their recommendations, §5's
prohibitions, and §6's other controls are unchanged — and nothing in the review so far suggests the
*design* is wrong. **This is a path-construction defect in an otherwise well-shaped increment**, and
the module doc's own framing of the scan-layer boundary and the discovery-not-removal rule read
exactly as the RFC intended.

**Re-land as one commit** on top of the revert. The disclosure is already in.

## 7. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9, and **note it grew on 2026-09-02**: it now includes
`RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`.

**CI is my control and I will run it before this is considered done** — properly this time.
