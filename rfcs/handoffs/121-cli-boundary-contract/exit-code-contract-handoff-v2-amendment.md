# Amendment to `exit-code-contract-handoff-v1.md` — one item round 2 must also carry

**v1 stands in full, including §9's seam, which the split correctly used. `215b497` stays as it is —
nothing in it is reverted, and it is pushed with this amendment.**
**Architect review of `.git-exclude/review-request/exit-code-contract-report-v1.md`, 2026-09-02.**

---

## 1. Round 1 accepted

**The split is right and the justification is better than the seam I named.** §9 pre-authorized
§2+§5 against §3+§4; you took it *and* gave the ordering reason I had not — `unlock`'s abort needs
`CliError` to exist before it can construct `Usage`/`Failure` deliberately, so doing §4 first would
have meant redoing it.

**Verified independently, live:**

```
prikk verify                → 0    success
prikk worktree-status       → 1    dirty worktree (findings)
prikk nonsense-command      → 2    unknown command (usage)
```

And the two round-2 items confirmed still unfixed, exactly as your report states —
`prikk status --nonsense` → **0** (it swallows the argument and succeeds), `prikk unlock </dev/null`
→ **0**. Neither is a regression; both are §3/§4.

**`From<String> → Failure` is the right default and the reason given is the right reason**: every
error in this crate exited `1` before the contract existed, so the default makes the type change
pure plumbing rather than a silent reclassification. The one deliberate exception — `run()`'s
unknown-command arm constructing `Usage` — is dispatch-layer code in `main.rs`, is the clearest
instance of §1's own definition, and was declared rather than smuggled.

**§5 is done and demonstrated properly.** The JSON printer returns an error; the test builds a
**real** `RepositoryVerification` and pops a stage outcome, rather than hand-constructing a 29-field
struct that would drift at the 30th. `verify --format json` still emits its schema and exits 0.

Gates re-run here: fmt clean, clippy exit 0, **1478/1478** stable and MSRV, 57/57, boundary and
reference `valid: true`, `git diff --check` clean.

## 2. Required in round 2 — and v1 §5's claim about it was mine and wrong

v1 §5 called `output/verification.rs:117` *"the last hard `panic!` on a user-reachable path in the
CLI."* **It was not.** I grepped the crate after your commit:

```
crates/prikk-cli/src/stdout.rs:77:
    WriteOutcome::Failed(err) => panic!("failed printing to stdout: {err}")
```

That is now the only one left, and it predates my handoff — the EPIPE increment (`0c96b06`)
introduced it deliberately, to preserve `std::println!`'s own behaviour for a genuine write failure.
**That was correct then. The contract you have just wired makes it inconsistent now:**

```
$ prikk verify >/dev/full
exit=101
thread 'main' panicked at crates/prikk-cli/src/stdout.rs:77:38:
```

**`101` is outside the ruled vocabulary.** A full disk on a redirected stdout is an operational
failure — `1` — and it should print `error: …` on stderr like every other failure, not a panic
banner and a backtrace hint.

**Do:** make a non-`BrokenPipe` stdout write failure exit `1` with the message on stderr.

**And keep the property the EPIPE increment established, which is the whole point of that code:**
`BrokenPipe` still exits `0` silently, and a genuine write error is still **not** swallowed with it.
`classify`'s three-way split (`Ok` / `ClosedPipe` / `Failed`) already separates them; only the
`Failed` arm's action changes.

**The narrowness control from that increment must be re-run and stay green** — `/dev/full` producing
a real error, and the unit tests confirming `StorageFull`/`PermissionDenied`/`Interrupted`/`WriteZero`
classify as `Failed` and never `ClosedPipe`.

**Note the shape problem and solve it deliberately:** `write_and_handle` has no error channel — it is
called from macro expansions at ~400 sites, which is exactly why `std::process::exit` was chosen over
threading a `Result` back. **Do not thread a `Result` through those call sites.** Exiting `1` at the
write site, after printing to stderr, is consistent with how the `ClosedPipe` arm already works. If
you see a better shape, say so; if this turns out to need a design decision rather than a one-arm
change, stop and report rather than improvising.

## 3. Everything else round 2 already owed

Unchanged from v1: §3 (unknown-argument rejection, duplicate-flag refusal via a shared in-repo
helper, and the enumeration — **including commands with no parser at all**, since `status` swallowing
`--nonsense` is exactly that shape) and §4 (`unlock`'s abort exits `1`, interactive decline included,
with the EOF path controlled explicitly).

## 4. Controls added to v1 §7

8. **`prikk verify >/dev/full` exits `1`** with `error: …` on stderr and no panic banner — before and
   after.
9. **The EPIPE narrowness control re-run and still green**: `BrokenPipe` → `0` silently, and the
   `classify` unit tests unchanged.

## 5. Gates

Unchanged from v1 §8. **No CI control** — that is mine at push time.

**RFC 121 closes when round 2 and `command-discovery-handoff-v1.md` have both landed.**
