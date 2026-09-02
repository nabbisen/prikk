# No ignore mechanism at any layer — implementation handoff

**Authority:** `rfcs/proposed/124-worktree-ignore-mechanism.md`.
**Base:** current `main` (`c1335ad`). **Under `003-landing-work-on-main.md`.**

**Two commits, in this order: the disclosure, then the mechanism.** RFC 124 §5 is explicit that the
disclosure is not blocked on the design, and it should not wait behind it — the gap is harmful
mainly because it is undisclosed.

---

## 1. The gap, and why it is worse here than elsewhere

`commit` scans the whole worktree and authors an operation for every file it finds. **Only `.prikk`
is skipped** — `worktree_status.rs:261`, `first.as_os_str().to_str() == Some(".prikk")`. On any real
project, `target/`, `node_modules/`, `dist/`, editor swap files all go into signed history on the
first commit.

**And that history cannot be cleaned up.** The no-GC model is deliberate and correct — containers
compact, objects never disappear. **Scan everything plus delete nothing is what raises this above a
convenience gap**: a mistaken first commit of a build directory is a permanent property of that
repository.

## 2. Commit one — the disclosure, which can ship today

`README.md`'s **"Not a Good Fit Yet"** list (line 83) names the limits a new user needs and **does
not name this one.** The audit's finding was precisely that this is the biggest practical-use gap
the project does not tell users about.

Add it. One bullet, in that list's own register. **This commit needs nothing from §3–§5** and should
land first, on its own.

**Check whether `docs/src/reference/non-goals.md` needs the same line** and report what you found —
if the two lists disagree about what is deferred, that is worth knowing whether or not you change
it.

## 3. Commit two — and a finding that is not in RFC 124

**There are two independent worktree walks, and RFC 124 assumes there is one.**

| Command | Walk |
|---|---|
| `commit` | `worktree_patch/node_authoring/worktree_files.rs:41` → `fsutil::list_directory(layout.worktree_mutation_root(), dir)` — the **anchored** fsutil path |
| `worktree-status` | `worktree_status.rs:210` → plain `fs::read_dir` |

**Each has its own `.prikk` skip.** So an ignore rule has two places to bind, and a rule that binds
in only one makes the two commands disagree about what is in the worktree — **the exact defect shape
RFC 122 just fixed for the baseline**, recurring one layer up.

**Requirement: one derivation of "is this path ignored?", consulted by both.** If that means
extracting a shared helper, extract it. **A second implementation that agrees today is the defect,
one release later** — that sentence is from RFC 122's own handoff and it applies unchanged here.

**Report which walk you bound it in and how you proved the other agrees.** Grepping is not proof; a
test that commits and then runs `worktree-status` against the same ignored path is.

## 4. The design questions, with recommendations you may overturn

RFC 124 §3 states these; the recommendations are mine and are not rulings.

1. **Where it binds — the scan layer only.** **This is the constraint, not a preference.** An ignore
   rule must never reach identity, replay, verification, or materialization: a patch that already
   exists must apply regardless of what the receiver's ignore file says, or two repositories with
   different ignore files would disagree about the same history. **State in your report how you
   ensured a received patch is unaffected**, and test it.
2. **Syntax — literal repo-relative path prefixes.** No globbing, no negation, no per-directory
   files. It covers `target/`, `node_modules/`, `.venv/`, and it can be described completely in two
   sentences. **Say it is a stated limit, not a first step toward gitignore** — an ignore syntax
   that *nearly* matches gitignore is worse than one that obviously does not.
3. **Where the file lives, and whether it is itself tracked.** `.prikkignore` at the root is the
   obvious answer; whether it is tracked is a real question with a real consequence (a tracked
   ignore file cannot be varied per checkout without a commit). Decide and say why.
4. **Already-tracked paths — the rule applies to discovery, never to removal.** A path already in
   history stays; adding a line must never author a `DeleteNode`. **Test this explicitly**; it is the
   one failure mode here that destroys data.
5. **A malformed ignore file fails closed.** Everything else in this product does. Note that failing
   closed on a *convenience* feature is a different trade than on durability, so make it a decision
   rather than an inherited default — but the default is right.

## 5. What must not happen

- **No `.gitattributes`-shaped behaviour** — filters, eol, diff drivers. Not this, not later.
- **No per-directory ignore files, no negation, no global or per-user ignore file.** There is no
  config file; `main.rs:344` defers one deliberately.
- **No `prikk check-ignore`-shaped tooling.**
- **No new dependency.** No glob crate — the syntax in §4.2 is chosen partly so none is needed.

## 6. Controls

1. **The disclosure quoted before and after**, plus your `non-goals.md` finding as a result.
2. **A file matching an ignore rule is absent from `commit`'s operations** — the real binary, on a
   real repository.
3. **The same path is absent from `worktree-status`'s untracked list** — §3's agreement, demonstrated
   rather than argued.
4. **A received patch touching an ignored path still applies** — §4.1's constraint, tested. This is
   the control that proves the rule stayed at the scan layer.
5. **An already-tracked path that a new rule would cover is not deleted** — §4.4.
6. **A malformed `.prikkignore` refuses** — §4.5, with the exit code shown.
7. **Your enumeration**: every place the worktree is walked, and whether each now consults the shared
   rule. **This handoff names two; treat that as a floor** — my site lists have been short three
   times this month and understated by threefold once.

## 7. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against each commit — **the set now includes
`RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`** — with clippy
as a single invocation per target and the exit code captured explicitly, plus `mdbook build` if any
doc page changes. Cross-target clippy judged from your own diff.

**No CI control** — that is the architect's at push time.

Two commits on `main`, local, **no push, no tag**. **If the mechanism turns out larger than one
reviewable increment, land the disclosure and stop** — that is pre-authorized and is the whole reason
it is commit one.
