# RFC 121 — The CLI's boundary contract: what a script may rely on

**Status.** **Proposed.** Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-{1b,3}.md`; review at
`.git-exclude/reviewed/external-audit-20260831-review-v1.md` §1.3, §3.4, §4). Every finding here was
reproduced independently before this file was written.

**Tracks.** The boundary between the CLI and its caller — pipes, exit codes, flags, help. Nothing in
this RFC touches the store, the object format, signing, or trust.

**RULED BY THE ARCHITECT 2026-09-01 (§6): option 3 — `0` / `1` / `2`.** §6 originally called this
the owner's call; **that escalation was wrong and §6a says why.** What remains the owner's is
untouched and already settled elsewhere.

---

## 1. The problem, in one sentence

**Every promise this CLI makes to a script is either unstated or wrong**, and one of them panics.

The store beneath it is contract-driven to an unusual degree — nine durability guarantees as trait
methods, a machine-checked unsafe boundary, an admitted-schema table consulted at every read and
write. The CLI on top of it has no equivalent, and the audit found the consequences at four separate
points that all turn out to be the same missing thing.

## 2. Evidence

### 2.1 A closed pipe panics — reproduced

```
$ prikk verify | head -3
thread 'main' panicked at library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Reproduced at `3a8d730` with stderr captured separately. Rust's default `SIGPIPE` disposition is
`SIG_IGN`, so `println!` returns `EPIPE` and the standard-library macro panics. **This fires for
`head`, for quitting `less` early, and for any script that stops reading.** It is the most likely
defect in this RFC to be met by a real user, and it presents as a crash in a tool whose entire pitch
is trustworthiness.

### 2.2 Exit codes carry no information

`0` and `1` are the whole vocabulary (`main.rs:70-78`). A flag typo, a dirty worktree, and a corrupt
repository are indistinguishable to a caller. Worse in one direction: `unlock` **declining** to act
exits `0`.

### 2.3 Unknown arguments are silently swallowed

```
$ prikk status --nonsense
prikk repository: …
exit=0
```

Reproduced. `status` ignores every argument it is given. `init` takes the first positional as the
path and discards the rest.

### 2.4 Repeated flags silently take the last value

`parse_export_args` (`bundle.rs:217-248`) assigns `--ref` and `--output` into a single `Option`,
last write wins. `prikk bundle export --ref heads/main --ref heads/other -o x` exports
`heads/other`, with no diagnostic. **This one is not hypothetical harm: `docs/src/guide/backup-restore.md`'s
"A bundle is one ref" section is exactly where a reader learns they have several branches, and the
next thing such a reader types is two `--ref` flags.** They would get a backup of one branch and no
indication which. The same shape recurs across the hand-rolled parsers.

### 2.5 Four flags exist and `--help` does not mention them

`verify --format json`, `verify --stop-on-first-error`, `unlock --force` (an alias of `--yes`,
`unlock.rs:27`), and `doctor --repair-main-ref`.

**`--repair-main-ref` is not an oversight and must not be removed.** `args.rs:85` documents it as
*"Always refused -- no repair is implemented"*, and
`rfcs/handoffs/DC-78-history-exchange/doctor-repair-main-ref-message-handoff-v1.md` already repaired
its refusal message after a prior review. Recognizing an input and refusing it with a reason is
better than an "unknown argument" error for an operation a user will reach for in trouble. The defect
is only that `--help` is silent about it.

### 2.6 One hard `panic!` remains in the CLI

`output/verification.rs:117`, on a missing verification stage in the JSON printer. It fires only if
`verify_repository` omits a stage it declared — a broken invariant, which is a reason to return an
error, not to abort.

## 3. What these five have in common

They are not five bugs. They are one absence: **the CLI has never stated what it guarantees to a
caller**, so each site made a local choice, and the local choices disagree. Fixing them one at a time
would leave the next flag and the next command free to disagree again.

## 4. Scope

**In:** EPIPE handling; a documented exit-code contract; per-command `--help` derived from the
existing command registry; reject-unknown-argument and refuse-duplicate-flag as shared parser
behaviour; the four undocumented flags; the JSON-printer `panic!` → error.

**Out:** any change to what a command *does*; JSON output for commands that lack it; a config file
(`main.rs:344` defers this deliberately); adopting a third-party argument parser — the zero-dependency
CLI is a standing property of this project, and a shared in-repo helper is the shape that preserves it.

## 5. Constraints this RFC must respect

- **No new third-party dependency in the shipped binary.** `prikk-cli` has zero today and
  `placement.rs` enforces it. EPIPE handling must reuse `rustix` (already a `prikk-store` dependency)
  or std, not a new crate.
- **`--repair-main-ref` keeps its recognized-and-refused shape** (§2.5).
- **Per-command help derives from the command registry**, never a second hand-written table — RFC 118
  ("derive, never transcribe") governs, and the RFC 118 §8 doc-coverage gate already binds `COMMANDS`
  to the documentation.

## 6. The ruling this RFC needs

**What is the exit-code contract?** Three shapes, and the choice is a public commitment because
scripts will encode it:

1. **`0` ok / `1` everything else** — status quo. Honest, useless to callers, and free.
2. **`0` ok / `1` findings / `2` usage error / `3` integrity failure.** The audit's suggestion.
   Distinguishes the three things a caller actually branches on. Costs one classification decision
   at every error site.
3. **`0` ok / `1` any failure / `2` usage error.** The minimal split that fixes the worst confusion
   (a typo reading as corruption) without asking every error site to self-classify.

### 6a. The ruling, and why it is the architect's to make

**Ruled: option 3.**

```
0  the operation succeeded and did what was asked
1  operational failure — verification findings, integrity failure, refusal,
   a dirty worktree, or any refusal to carry out the request
2  usage error — unknown argument, missing required flag, malformed flag value,
   duplicate flag; detected before any repository work begins
```

**Why this and not option 2 (a separate integrity code).** The split a caller genuinely cannot make
for itself is **usage error versus operational failure** — reconstructing it from output means
parsing English error strings. **Findings versus integrity failure is already available, structured
and three-valued, in `verify --format json`.** Encoding a lossy subset of that verdict into an
integer would create a second source of truth for the same question, and the two would drift. **This
project has an RFC about exactly that: RFC 118, "derive, never transcribe."** Option 2 asks every
error site in the workspace to self-classify into a grading that a schema'd output already carries
better.

Widening later is additive and cheap; narrowing is a break. Three codes now, and a fourth only if
evidence demands one.

**Why the architect rules this, correcting §6's own framing.** §6 called it the owner's because
it is "an interface promise". By that reasoning every error-message string and every `note:` line
would be the owner's too, and they are not. More concretely:
**`docs/src/reference/release-compatibility.md:53` already names "command names, arguments, exit
behavior, and human-readable output" as a compatibility surface, and already states the pre-1.0
policy for changing it** — *"a minor release may intentionally change documented Cargo or CLI
surfaces when release notes identify the change."* **The owner has already ruled the policy question.
What was left was the shape of a contract the policy anticipates, and that is design work.**

**What remains the owner's, and is not being taken here:** whether this contract is ever promoted
into a *stability commitment* — a promise not to change it — which is a release-compatibility
decision, not a design one.

### 6c. One exit path still outside the vocabulary — recorded 2026-09-02, unscheduled

Found by the architect while reviewing the round-2 increment that fixed the other two.

`stdout.rs`'s write-failure arm now reports and exits `1` for a genuine stdout failure, as the
contract requires. **But it reports with `eprintln!`, which panics if stderr is also unwritable:**

```
$ prikk verify >/dev/full 2>/dev/full
exit=101
```

**Reachable only when both streams are unwritable**, where there is by definition nothing useful to
report — so this is recorded rather than scheduled. It is noted because §6a states `0`/`1`/`2` as
*the whole vocabulary*, and a promise with a reachable exception is worth writing down rather than
discovering later.

**The fix, whenever it is taken, is one line**: write to stderr with `writeln!(io::stderr(), …)` and
ignore its result, then exit `1` — so the exit code stays inside the contract even when the message
cannot be delivered. **It belongs with whoever next touches `stdout.rs`**, not with the argument
hygiene work, which does not go near it.

### 6d. Two commands acquire credentials before validating arguments — recorded 2026-09-02, unscheduled

**§1's own wording is the standard this misses**: a usage error is *"detected before any repository
work begins."* Building a signer from the environment is work.

`main.rs::run_seal` (`:169`) and `main.rs::run_merge` (`:180`) both call
`maintainer_signer_from_env()?` **before** parsing their arguments. So an operator who mistypes an
argument *and* has no maintainer key configured is told the wrong thing:

```
$ prikk seal  --ref heads/main --ref heads/main      # no PRIKK_MAINTAINER_* set
error: maintainer signing is required: set PRIKK_MAINTAINER_KEY_ID (no signing key configured)   exit 1
$ prikk merge --into heads/main --into heads/x --from heads/y
error: maintainer signing is required: set PRIKK_MAINTAINER_KEY_ID (no signing key configured)   exit 1
```

With the key configured, both correctly report `duplicate --ref flag` / `duplicate --into flag` at
exit `2`. **The exit code is right for the error that happened; the wrong error happened.**

**`run_commit` and `run_rollback_draft` get the ordering right** — they parse first, so
`prikk commit -m a -m b` reports `duplicate -m/--message flag` at exit `2` even with no author key
configured. So this is two sites against two, not a house style.

**Found in two stages, which is worth recording as much as the defect.** The round-3 increment found
`seal` and named it rather than working around it silently in the test that had to set the env vars
to reach the refusal it was testing. The architect's review then found `merge` has the identical
shape. **A site list is a floor even when it appears in a report rather than a handoff.**

**Correctly out of scope for round 3** ("v1 §3 and nothing else"). The fix is a reorder of two
functions. **It belongs with §6c** — both are small residuals of the contract work, and both are for
whoever next has `main.rs` open.

### 6b. Two consequences that fall out of the ruling

**The invariant, stated once so each site derives from it rather than deciding for itself:
no command may exit `0` while refusing to do what it was asked.**

**`prikk unlock`'s abort path must become non-zero — and the audit's stated reason was not the real
one.** The audit and this RFC's first draft both recorded it as *"`unlock` declining exits 0"*. What
`unlock.rs:66-69` actually does is return `Ok(())` when **the operator** declines at the interactive
prompt, which on its own is defensible: a human answered "no" and got exactly that.

**The real hazard is one step further in, and it is worse.** `confirm_interactively`
(`unlock.rs:109-123`) returns `false` when `stdin` yields no `yes` — including EOF. **In a
non-interactive context the prompt can never succeed, so it resolves to "no", prints
`aborted: lock not cleared`, and exits `0`.** A CI script running `prikk unlock && proceed` therefore
proceeds with the lock still held, having been told everything is fine. Under the invariant above it
exits `1`, interactive refusal included — uniform, rather than special-casing whether a human was
watching.

**This is a behaviour change and must be named as one** in the release notes, per
`release-compatibility.md`'s own pre-1.0 policy. It does not need a separate ruling; that policy
already covers it.

## 7. Non-goals

This RFC does not make the CLI scriptable in the general sense. `verify --format json` remains the
only structured output surface; extending that pattern to other commands is separate work with its
own schema-stability question.
