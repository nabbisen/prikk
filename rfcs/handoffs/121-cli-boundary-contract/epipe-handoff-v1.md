# A closed pipe must not panic — implementation handoff

**Authority:** `rfcs/done/121-cli-boundary-contract.md` §2.1.
**Base:** current `main` (`94b6cb7`). **Under `003-landing-work-on-main.md`.**
**The repository moved to `prikk-vcs/prikk` (RFC 129) — confirm your remote before you start.**

**Scope: EPIPE only.** RFC 121 also covers the exit-code contract, per-command help, arg-parser
hygiene and the four undocumented flags. **Those are a separate increment. Do not do them here.**

---

## 1. The defect

```
$ prikk verify | head -3
thread 'main' panicked at library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Reproduced at `3a8d730` with stderr captured separately. Rust's default `SIGPIPE` disposition is
`SIG_IGN`, so `println!` gets `EPIPE` and the macro panics.

**Every command is affected**, and the user-facing shapes are ordinary: `| head`, quitting `less`
early, `| grep -q`, any script that stops reading. **A tool whose entire claim is trustworthiness
should not print a panic and a backtrace hint because someone piped it to `head`.**

## 2. The required behaviour

**A closed stdout is not an error.** The consumer asked for less output than was available and got
it. Under RFC 121 §6's ruled exit-code contract this is `0` — *"the operation succeeded and did what
was asked"* — with nothing written to stderr.

## 3. The constraint that decides the design

**`prikk-cli` has zero third-party dependencies and `tools/release-policy/src/boundary/placement.rs`
enforces it** — its `ALLOWED_THIRD_PARTY` entry is `("prikk", &[])`. You may not add one.

Three shapes, with what I have already verified about each:

| Shape | Reality |
|---|---|
| **A. Route CLI output through a helper macro** that writes to a locked stdout and, on `ErrorKind::BrokenPipe`, exits `0` silently | Pure `std`. **399 `print!`/`println!` sites** across `prikk-cli/src` — concentrated in `output.rs` (120), `output/verification.rs` (66), `sync.rs` (46), `main.rs` (44), `output/merge_evidence.rs` (41), `output/worktree.rs` (26), `bundle.rs` (21), `unlock.rs` (11). Large but mechanical |
| **B. Restore `SIGPIPE` to `SIG_DFL` at startup** via a `#[cfg(unix)]` helper exported from `prikk-store` | One line at the call site — but `prikk-store` enables `rustix` with `features = ["fs", "process"]` only, and signal APIs are not in those. **Adding a feature to reach them is a dependency-surface decision that is not yours to make in this increment** |
| **C. `catch_unwind` around `main`** and inspect the panic payload | **Rejected.** Recognising this panic means string-matching `"failed printing to stdout"`, which is a std-internal message. Do not build on it |

**My recommendation is A, and the deciding reason is not the dependency rule — it is Windows.**
`SIGPIPE` does not exist there; a write to a closed pipe returns an error and `println!` panics
exactly the same way. **B fixes one platform and leaves the panic on another**, in a project whose
CI runs the read-only command set on `windows-latest` precisely so this class of gap cannot hide.

**If you conclude B is right anyway, stop and escalate rather than adding a `rustix` feature** —
write the finding into `.git-exclude/review-request/` and I will rule. Escalating is the correct
outcome there, not a failure to deliver.

## 4. Points to get right in shape A

- **Locked stdout, once.** 399 unlocked `println!` calls each take and release the lock; a helper is
  the moment to stop doing that. Do not make this a performance project, but do not make it worse.
- **stderr is a separate question.** There is exactly one `eprintln!` (`main.rs`'s error printer). A
  closed *stderr* is not this increment's problem — say what you did and why.
- **Exit, do not unwind.** Exiting `0` on `BrokenPipe` at the write site is the simple correct thing;
  threading a `Result` back through 399 call sites is not.
- **Do not silence real write errors.** `ENOSPC` on a redirected stdout is a genuine failure and must
  not be swallowed by the same arm that swallows `BrokenPipe`.

## 5. Controls

1. **The §1 reproduction, before and after** — `prikk verify | head -3` with stderr captured
   separately, showing the panic at base and empty stderr on your commit.
2. **The exit code is `0`**, shown explicitly, not inferred.
3. **A second command and a second consumer** — e.g. `prikk log | head -1` and
   `prikk verify | grep -q .` — so the fix is not shown only where it was written.
4. **A non-`BrokenPipe` write error still fails.** Construct one (a redirect to a full or unwritable
   destination, or a unit test over the helper) and show it is not swallowed. **This is the control
   that proves the fix is narrow.**
5. **An automated test**, so this cannot regress silently. If a piped-consumer test is impractical in
   the harness, say so plainly and test the helper directly.

## 6. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against your final commit, **clippy as a single
invocation per target with the exit code captured explicitly**. **Re-check your own diff for
`#[cfg(unix)]`/`#[cfg(windows)]`/`#[cfg(target_os)]`** — shape A may introduce none, but shape B
certainly would, and the cross-target requirement follows the diff, not this sentence.

One commit on `main`, local, **no push, no tag**.

## 7. Out of scope

The exit-code contract itself, `unlock`'s abort path, per-command `--help`, unknown-argument
rejection, duplicate-flag refusal, the four undocumented flags, and the JSON-printer `panic!`
(`output/verification.rs:117`). All are RFC 121, none is this increment.
