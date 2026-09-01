# The exit-code contract, and the arguments that produce it — implementation handoff

**Authority:** `rfcs/proposed/121-cli-boundary-contract.md` §2.2, §2.3, §2.4, §2.6, and §6/§6a's
ruled contract.
**Base:** current `main` (`42d0d16`). **Under `003-landing-work-on-main.md`.**

**Scope: the contract and everything that produces a usage error.** §2.1 (EPIPE) shipped at
`0c96b06`. **§2.5 (per-command `--help`, the four undocumented flags) is a separate handoff** —
`command-discovery-handoff-v1.md`. Do not do it here.

---

## 1. The contract, already ruled

RFC 121 §6a, ruled by the architect 2026-09-01:

```
0  the operation succeeded and did what was asked
1  operational failure — verification findings, integrity failure, refusal,
   a dirty worktree, or any refusal to carry out the request
2  usage error — unknown argument, missing required flag, malformed flag value,
   duplicate flag; detected before any repository work begins
```

**The invariant, from which each site derives rather than deciding for itself:
no command may exit `0` while refusing to do what it was asked.**

**A fourth code for findings-versus-integrity was considered and refused**: `verify --format json`
already carries that verdict, structured and three-valued, and duplicating a lossy subset of it into
an integer is what RFC 118 exists to prevent. **Do not add one.**

## 2. The design question this forces, and my recommendation

`main()` today is:

```rust
match run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(msg) => { eprintln!("error: {msg}"); ExitCode::from(1) }
}
```

Every command's `run` is `fn(Vec<String>) -> Result<(), String>` — the type stored in
`Command.run` (`commands.rs`). **A `String` cannot say "this was a usage error", so `2` is
unreachable without changing that type.** That is the real cost of the contract and it should not be
discovered mid-increment.

**Recommendation: a two-variant error at the CLI boundary only** — a usage variant and a failure
variant, both carrying the message they carry today — with `main` mapping them to `2` and `1`.
`Command.run`'s signature changes once, in the registry, and every command's return type follows.

**Alternatives are yours to weigh**, and if you find a shape that avoids the signature change without
smuggling classification into message text, say so in the report. **What is not acceptable is
classifying by string matching on the message** — that is the "couples message text to scripts" trap
RFC 121 §2 already names.

## 3. Arguments: reject unknown, refuse duplicate

### 3.1 Unknown arguments are silently swallowed

```
$ prikk status --nonsense
prikk repository: …
exit=0
```

Reproduced. `status` ignores every argument. `init` takes the first positional as the path and
discards the rest.

### 3.2 Repeated flags silently take the last value

`parse_export_args` (`bundle.rs:217-248`) assigns `--ref` and `--output` into a single `Option`,
last write wins. `prikk bundle export --ref heads/main --ref heads/other -o x` exports
`heads/other`, silently.

**This one has a named victim.** `docs/src/guide/backup-restore.md`'s "A bundle is one ref" section
is exactly where a reader learns they have several branches — and the next thing such a reader types
is two `--ref` flags. They would get a backup of one branch and no indication which.

### 3.3 The parsers, and how to enumerate them

Nine files define a `parse_*_args`: `args.rs`, `args/checkout.rs`, `args/merge_evidence.rs`,
`args/merge_execute.rs`, `branch.rs`, `bundle.rs`, `seal.rs`, `sync.rs`, `tag.rs`.

**That list is a floor.** RFC 125 was named for two files and the class was in seven, then one level
deeper in an eighth. **Enumerate by mechanism** — every site that consumes `Vec<String>` and matches
on argument strings, including commands with no parser at all (`status` has none, which is *why* it
swallows everything) — **and report what you searched.**

**A shared helper, not nine independent fixes.** RFC 121 §3's whole point is that these are one
absence, not five bugs; fixing them site-by-site leaves the next flag free to disagree again.
**`prikk-cli` has zero third-party dependencies and `placement.rs` enforces it** — this is an in-repo
helper, not a parser crate.

## 4. `unlock`'s abort path

`unlock.rs:66-69` returns `Ok(())` — exit `0` — when the operator declines at the interactive
prompt. On its own that is defensible: a human answered "no".

**The hazard is one step further in.** `confirm_interactively` (`unlock.rs:109-123`) returns `false`
when stdin yields no `yes`, **including EOF**. So in a non-interactive context the prompt can never
succeed, resolves to "no", prints `aborted: lock not cleared`, and exits `0`. **A CI script running
`prikk unlock && proceed` continues with the lock still held, having been told everything is fine.**

Under §1's invariant it exits `1`, **interactive refusal included** — uniform, rather than
special-casing whether a human was watching.

**This is a behaviour change and must be named in the release notes**, per
`release-compatibility.md`'s own pre-1.0 policy for CLI surfaces. No separate ruling needed; that
policy already covers it.

## 5. The JSON-printer `panic!`

`output/verification.rs:117` panics if `verify_repository` omits a stage it declared. **That is a
broken invariant, which is a reason to return an error, not to abort** — and it is the last hard
`panic!` on a user-reachable path in the CLI.

## 6. What must not happen

- **No new dependency.** `prikk-cli` has zero, and `boundary-check` enforces it.
- **No fourth exit code** (§1).
- **No classification by message-string matching** (§2).
- **`doctor --repair-main-ref` keeps its recognized-and-refused shape.** `args.rs:85` documents it as
  *"Always refused -- no repair is implemented"*, and a prior handoff repaired its refusal message.
  Recognizing an input and refusing it with a reason is better than "unknown argument" for something
  a user reaches for in trouble. **Whatever unknown-argument rejection you build must not swallow
  it.**
- **No change to what any command does** — only to how it reports.

## 7. Controls

1. **Each of the three exit codes demonstrated**, on real commands: `0`, a `1` (dirty worktree or a
   verification finding), and a `2` (unknown flag) — exit codes shown explicitly, not inferred from
   success/failure.
2. **`prikk status --nonsense` refused**, with its exit code, before and after.
3. **Duplicate `--ref` refused** on `bundle export`, before and after — and **show which value won at
   base**, the way RFC 125's round 2 did. That converts last-wins from a reading into a fact.
4. **`unlock`'s abort exits `1`** — demonstrated both interactively-declined and via the EOF path
   (`prikk unlock < /dev/null`), since the second is the one that bites scripts.
5. **The JSON-printer returns an error rather than panicking**, shown.
6. **Your parser enumeration as a result** (§3.3): what you searched, which sites you found, which
   have no parser at all.
7. **A test per class**, so none of this regresses silently.

## 8. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit, **clippy as a single
invocation per target with the exit code captured explicitly**, plus `mdbook build` if any doc page
changes. Cross-target clippy judged from your own diff.

**No CI control** — that is the architect's at push time.

One commit on `main`, local, **no push, no tag**.

## 9. If this grows past one reviewable increment

**Stop and say so rather than delivering a diff too large to review.** The natural seam is §2+§5 (the
contract and the error surface) against §3+§4 (the parsers and `unlock`). Splitting on that line is
pre-authorized; splitting elsewhere needs a word first.
