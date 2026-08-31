# RFC 124 — No ignore mechanism exists at any layer

**Status.** **Proposed.** Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-1a.md` §3, Top-10 #4). Confirmed: `worktree_status.rs:185,216-227` skips
exactly one path, `.prikk`, and nothing else.

**Tracks.** The worktree scan layer only. Nothing here touches identity, patches, or trust.

---

## 1. The problem

`commit` scans the whole worktree and authors an operation for every file it finds. There is no
ignore file, no exclude flag, no built-in pattern. On any real project — `target/`, `node_modules/`,
`dist/`, editor swap files — the first `commit` sweeps all of it into signed, permanent history.

**And this history cannot be cleaned up.** The no-GC model is deliberate and correct
(`layout.rs`, `compact.rs:1-26` — containers compact, objects never disappear), which means a
mistaken first commit of a build directory is a permanent property of that repository. The
combination of *scan everything* and *delete nothing* is what raises this above a convenience gap.

## 2. Why it is not already disclosed

The README's "Not a Good Fit Yet" section names the limits a new user needs. **This one is absent** —
the audit is right that it is the biggest practical-use gap that the project does not tell users
about. The message discard (RFC 123) at least fails visibly; this one fails by succeeding.

**Documenting it is not the fix, but it is the part that can ship first**, and it should.

## 3. Design questions this RFC must answer

**3.1 Where does the rule bind?** Recommendation: **the worktree scan layer only** — the same place
`.prikk` is skipped. An ignore rule must never reach identity, replay, verification, or materialization:
a patch that already exists must apply regardless of what the receiver's ignore file says, or two
repositories with different ignore files would disagree about the same history. This is the single
most important boundary in this RFC and it should be stated as a constraint before any syntax is chosen.

**3.2 What syntax?** The audit says "a validated-subset syntax is fine". Options, cheapest first:

- **Literal path prefixes only** — one repo-relative path per line, matched as a directory or file
  prefix. No globbing, no negation, no precedence rules. Covers `target/`, `node_modules/`, `.venv/`.
- **A bounded glob subset** — `*` within a component, trailing `/` for directory-only, `#` comments.
  No `**`, no `!` negation, no per-directory override files.
- **gitignore compatibility** — familiar, and a large specification with precedence, negation,
  per-directory files, and edge cases that this project would then own forever.

**Recommendation: literal path prefixes, and say so as a stated limit rather than as a first
increment toward gitignore.** This project's house style is to ship a narrow thing that refuses
clearly rather than a broad thing that is subtly wrong — and an ignore syntax that *nearly* matches
gitignore is worse than one that obviously does not.

**3.3 Where does the file live, and is it itself committed?** `.prikkignore` at the repository root
is the obvious answer; whether it is itself tracked is a real question (it is content, so it would be
by default) with a real consequence (an ignore file that is itself in history cannot be varied per
checkout without a commit).

**3.4 What happens to already-tracked paths that a new rule would exclude?** Recommendation: **the
rule applies to discovery, never to removal.** A path already in history stays; the ignore file
cannot cause a `DeleteNode`. Otherwise adding a line to a text file silently authors deletions.

**3.5 Does an unreadable or malformed ignore file fail open or closed?** Everything else in this
product fails closed. An unparseable `.prikkignore` should refuse the commit, not silently ignore
nothing — but note that failing closed on a *convenience* feature is a different trade than failing
closed on durability, and it deserves an explicit choice rather than an inherited default.

## 4. Scope

**In:** the ignore file, its parser, its binding at the scan layer, the README/"Not a Good Fit Yet"
disclosure (shippable immediately and independently), and tests that a receiving repository's ignore
file cannot change how a received patch applies.

**Out:** `.gitattributes`-shaped behaviour (filters, eol, diff drivers); per-directory ignore files;
negation; a global or per-user ignore file (there is no config file — `main.rs:344` defers it);
`prikk check-ignore`-shaped tooling.

## 5. Ordering note

**The disclosure is not blocked on the design.** Adding this limit to README "Not a Good Fit Yet" is
a documentation increment that can land before any of §3 is decided, and it should — the gap is
harmful mainly because it is undisclosed.
