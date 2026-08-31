# RFC 121 — The CLI's boundary contract: what a script may rely on

**Status.** **Proposed.** Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-{1b,3}.md`; review at
`.git-exclude/reviewed/external-audit-20260831-review-v1.md` §1.3, §3.4, §4). Every finding here was
reproduced independently before this file was written.

**Tracks.** The boundary between the CLI and its caller — pipes, exit codes, flags, help. Nothing in
this RFC touches the store, the object format, signing, or trust.

**Ruling required (§6).** The exit-code contract's shape.

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

**The architect's recommendation is 3, then 2 if experience justifies it** — the usage/failure split
is the one a caller cannot reconstruct from output, while findings-vs-integrity is already available
in `verify`'s JSON. Committing to fewer codes now is cheaper to widen later than a four-code contract
is to correct. **This is the owner's call: it is an interface promise, not an implementation detail.**

## 7. Non-goals

This RFC does not make the CLI scriptable in the general sense. `verify --format json` remains the
only structured output surface; extending that pattern to other commands is separate work with its
own schema-stability question.
