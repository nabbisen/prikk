# The two exit-contract residues RFC 121 left behind — implementation handoff

**Authority:** `ROADMAP.md`'s corrective program, rows **AUD-09** and **AUD-10**, whose completion
conditions cite `rfcs/done/121-cli-boundary-contract.md` §6c and §6d.
**Base:** current `main` (`8608db0`). **Under `003-landing-work-on-main.md`.**

**Scope: AUD-09 and AUD-10 only.** AUD-05 through AUD-08 and RFC 126 §6a are a separate handoff
(`rfcs/handoffs/126-verification-infrastructure-coverage/hygiene-sweep-handoff-v1.md`). Neither item
here needs a design decision; both are small. **They are first in the queue because AUD-09 is a live
violation of a contract this project ruled on 2026-09-01 and shipped in 0.28.0.**

---

## 1. AUD-09 — the both-streams case still exits 101

### What is actually true today

RFC 121's round-2 amendment changed `stdout.rs`'s `WriteOutcome::Failed` arm from a panic to
`exit(1)`. **It fixed the single-stream case and left the case AUD-09 was written about.** I built
`8608db0` and ran both halves:

```
prikk --help >/dev/full 2><file>     -> exit=1   "error: failed printing to stdout: No space left on device (os error 28)"
prikk --help >/dev/full 2>/dev/full  -> exit=101
```

**The second line is AUD-09's own reproduction command.** `eprintln!` panics when its own write
fails, so when both streams are full the process dies with `101` — outside the `0`/`1`/`2` vocabulary
RFC 121 §6a rules as the whole contract.

### The comment is the more interesting half of the defect

`crates/prikk-cli/src/stdout.rs:79-82` justifies `eprintln!` as:

> `eprintln!` … is never itself BrokenPipe-fed here: a write failure on stdout says nothing about
> whether stderr is still open.

**That statement is true and answers a question nobody asked.** AUD-09 is not about `BrokenPipe` on
stderr — it is about `ENOSPC` on *both* streams, where the two failures **share a cause** rather than
being independent. As written, the comment reads to the next maintainer as if the case had been
considered and dismissed. **Rewrite it to name the both-streams case explicitly**, or the next person
to look will conclude, as I nearly did, that the item is closed.

### The fix

```rust
WriteOutcome::Failed(err) => {
    let _ = writeln!(io::stderr(), "error: failed printing to stdout: {err}");
    std::process::exit(1);
}
```

Ignoring the result is the point, not sloppiness: **there is no third stream to report the failure of
the failure report on**, and the contract's requirement is that the *exit code* stay inside `0`/`1`/`2`
even when the message cannot be delivered. Confirm `std::io::Write` is in scope (the same module
already calls `write!` on a `StdoutLock`).

### One sweep already done for you — do not redo it

The obvious worry is that other call sites reach `std::println!` directly and panic the same way.
**They do not.** `stdout.rs:18-37` defines shadowing `println!`/`print!` macros exported as
`pub(crate) use`, and I checked every `prikk-cli` source file that contains `println!` against every
file importing the shadow. **Exactly one file appeared to use `println!` without importing it —
`verify_verdict.rs` — and its single occurrence is inside a doc comment at line 19, not code.**

**So AUD-09 is one site.** Record that in the report; it is the kind of negative result that stops
the next person paying for the same sweep.

### The test

`/dev/full` is Linux-only, so the test needs a `#[cfg(target_os = "linux")]` gate — **and this is a
per-diff obligation, not a formality**: a test that shells out to `/dev/full` on macOS or Windows CI
fails the job, and this workspace runs both. Model it on the existing
`crates/prikk-cli/tests/rfc121_epipe.rs`, which already solves "invoke the built binary with a hostile
stdout and assert the exit code" — reuse its shape rather than inventing a second one.

**Assert the exit code, not the message.** The message is precisely what cannot be delivered in this
case; a test asserting on stderr content would pass for the wrong reason.

---

## 2. AUD-10 — the signer is acquired before arguments are parsed

### The two sites named

```
main.rs:184  fn run_seal(args: Vec<String>) -> ... {
main.rs:185      let root = current_dir()?;
main.rs:186      let signer = maintainer_signer_from_env()?;      <- before any parsing
main.rs:187      let result = seal::run_seal(root, args, &signer)?;

main.rs:196  fn run_merge(args: Vec<String>) -> ... {
main.rs:197      let signer = maintainer_signer_from_env()?;      <- before any parsing
main.rs:198      let report = merge::run_merge(args, &signer)?;
```

Parsing happens *inside* the callees — `seal.rs:50` (`parse_seal_args`) and `merge.rs:14`
(`parse_merge_execute_args`). So `prikk seal --bogus-flag` with no key configured reports
**"maintainer signing is required"** instead of the usage error, violating §6a's own
*"detected before any repository work begins"*.

`run_commit` (`main.rs:141`) and `run_rollback_draft` (`main.rs:557`) both parse on their first line.

### The constraint that decides the shape

**Do not fix this by moving `maintainer_signer_from_env()` inside the callees.** The
`signer: &impl MaintainerSigner` parameter exists so tests can inject a signer; acquiring it from the
environment internally would remove that seam. **Hoist the parse instead** — parse in `main.rs`,
pass the parsed value in — and accept the callee signature change that implies.

### My count is the part to distrust

**The ROADMAP row says "two sites against two, not a house style". That count covers `main.rs` only,
and it is the kind of claim this project has repeatedly found too narrow.** There are **eight**
production `maintainer_signer_from_env()` call sites, not four:

```
main.rs:186, main.rs:197, sync.rs:144, sync.rs:334, sync.rs:345, tag.rs:105, branch.rs:168, branch.rs:302
```

**`tag.rs:105` is the one I would look at first**: it acquires the signer *after*
`compute_patch_set_digest_and_count_from_block`, which is real repository work — a different
ordering error than AUD-10's, possibly not an error at all, but not something my two-against-two
framing ever examined.

**Check all eight and report what you find, including "the other six are fine".** If the ordering
principle holds elsewhere, say so explicitly; if a third instance exists, fix it and say that the row
undercounted. **Do not widen the change beyond what you can justify per site** — a mechanical sweep
that reorders working code is worse than the defect.

---

## 3. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run verbatim from there against your final
commit — **not from this file**. Handoffs deliberately point at that list rather than reproduce
it: `reference-check` treats a policy-command line outside its registered sites as an
`unregistered-reference`, and this handoff tripped exactly that on its first draft.

**The set grew on 2026-09-02** and now includes
`RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`.

**No CI control — that is mine at push time.**

**Commit locally on `main`; do not push, tag, or publish.** Report to
`.git-exclude/review-request/`, and state in it:

1. The before/after exit codes for **both** `/dev/full` invocations, run by you.
2. That the `println!`-shadow sweep was inherited from this handoff rather than redone — or, if you
   redid it and got a different answer, that answer.
3. What all eight signer sites turned out to be.
4. Any place where this handoff's own claims proved wrong. **The counts above are mine and this
   project's handoffs have a consistent record of understating them.**
