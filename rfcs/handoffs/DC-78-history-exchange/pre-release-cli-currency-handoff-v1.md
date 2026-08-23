# Pre-release CLI currency: implementation handoff

**Base:** current `main` (`bf13fbd`). **Under `003-landing-work-on-main.md`** — note **§4.1/§4.2 are new.**
**Origin:** `.git-exclude/reviewed/release-readiness-assessment-v1.md` §4.2 and §4.4.

**Why this is release-blocking, and not ordinary tidying.** The next release is the one that ships
`sync`. Right now **the binary tells users `sync` is not implemented**, the help text names an artifact
format that no longer exists, and **RFC 117's entire deliverable is absent from `--help`.** Each is
small; together they mean the release's headline feature misdescribes itself at three of the places a
new user actually looks. **These are much cheaper to fix before a tag than after one.**

---

## 1. Four targets, already located

Each was verified against the source. **They are a starting point, not the scope** — see §2.

### 1.1 `prikk status` says sync is not implemented — `crates/prikk-cli/src/main.rs:351-354`

```rust
println!(
    "status: multi-operation text diff minimization, plugins, and sync not \
     yet implemented"
);
```

`sync` shipped across RFC 115/116/117 and criterion 1 is **MET** (`c9c8576`).

**Do not simply delete the word.** Check whether *"multi-operation text diff minimization"* and
*"plugins"* are still unimplemented before rewriting the sentence around them — **the same claim that
was wrong about `sync` may be wrong about one of the others**, and correcting one clause while leaving a
second false one is the defect this whole sweep exists to remove.

### 1.2 Two stale `PEXCH001` references — `crates/prikk-cli/src/output/help.rs:107,115`

The exchange artifact has been **`PEXCH002`** since RFC 117 stage 3 (`faa4d39`) — the format gained a tag
section. Both lines describe it by its old magic.

### 1.3 `sync tags` and `sync adopt-tag` are missing from `--help` — `help.rs`

`sync.rs:51-59` dispatches **nine** subcommands:

```
summary  compare  have  build  accept  pending  seal  tags  adopt-tag
```

`help.rs` documents **seven**. `tags` and `adopt-tag` appear **nowhere** in the help output — so the
user-visible half of RFC 117 (tags travel, and the receiver adopts them under its own key) **cannot be
discovered by reading `--help`**.

**Take the usage shape from `sync.rs`'s own argument parsing, never from this document or from
inference.** `sync.rs:29` records that both take no `--tags-out`/`--tags <file>` pair; read the two
functions and describe what they actually accept.

### 1.4 `ROADMAP.md:401` — sync recorded as the largest unowned gap

> *"nothing in the tree exchanges history between repositories, so a distributed VCS cannot currently
> distribute. **This is the largest single gap** between prikk and dropping the "early implementation"
> badge, and it is unowned with no increment behind it."*

Recorded 2026-08-09; contradicted by ten merged increments. **It is the file a contributor reads to pick
up work**, so it is currently recruiting people to build something that exists.

**The adjacent `Transport` bullet is partly answered too** — RFC 116's accepted ruling is that prikk
stays off the network and `prikk-store` is bytes-in/bytes-out. **Adjudicate it; correct it only if the
ruling settles it outright**, and report rather than guess if it does not.

## 2. Method: fix these four, then check each one's siblings

This is **not** a sweep with a grep-defined scope — the four are already found. **But each belongs to a
class, and a class rarely has one member.** Before reporting, check for siblings:

| Target | Sibling check |
|---|---|
| 1.1 | **`main.rs:155` carries a near-identical "not yet implemented" list.** Read it. Are all its clauses still true? |
| 1.2 | Any other `PEXCH001` / stale magic (`PSYNCSU1`, `PSYNCHV1`, `PBNDL002`) anywhere in `prikk-cli` |
| 1.3 | Any **other** command whose `--help` subcommand set is narrower than its dispatch — the same method as 1.3, applied to `bundle`, `checkout`, `branch`, `trust`, `doctor` |
| 1.4 | Any other `ROADMAP.md` entry describing shipped work as unowned |

**Report what you checked and found clean**, not only what you changed. A sibling check with no findings
is a result.

## 3. Follow-on inside this increment: `README.md`'s sync list

The README false-claim pass added seven `sync` lines and **correctly declined** to add
`tags`/`adopt-tag`, reasoning that `help.rs` — the exhaustive surface — lacked them, so adding them in
one place only would be inconsistent.

**Once §1.3 lands, that reason dissolves.** Add the two lines to `README.md`'s `Useful Commands` so the
two surfaces agree. Keep the section's existing bare-command format, and keep them under the existing
`# sync: on `main` only, not in the released 0.22.1` comment.

## 4. Out of scope — do not touch

- **`crates/prikk-store/src/format.rs:18-21`.** Its comment (*"closure is the only field ever added to
  an existing payload after its type shipped"*) **is false** — `TagPayload` gained two fields after `Tag`
  shipped. **Its correct wording depends on a pending owner ruling** on whether `Tag` mints schema 2.
  **Leave it. It is mine, and it is blocked.**
- **Anything about `TagPayload`, `Tag` schema versions, or the RFC 114 identity vectors.** Same ruling.
- **`CHANGELOG.md`.** The release entry is its own increment and must come last.
- **`MILESTONES.md`.** Mine.
- **The two `println!`s in `prikk-store`** — long-standing, never adjudicated, not release-blocking.

## 5. What to report

1. **Each of the four**: what it said, what it says now, and the authority.
2. **Each sibling check** from §2 — including the ones that found nothing.
3. **§1.1 specifically**: your verdict on *"multi-operation text diff minimization"* and *"plugins"*,
   with how you determined it. **If either is still unimplemented, say so and keep it.**
4. **§1.3 specifically**: the usage shapes, and **which lines of `sync.rs` you read to get them.**
5. The `README.md` follow-on (§3).
6. Your verdict on `ROADMAP.md`'s `Transport` bullet (§1.4).
7. The **full gate set against the exact commit, after the last edit.**
8. Test counts. **A behaviour change to `prikk status`'s output may move a CLI test** — if a test asserts
   that string, update it and **say which**, rather than letting it fail or quietly deleting it.
9. Anything here that was wrong. **My last three handoffs each contained a miscount or a mis-stated
   scope, and the README one carried an error serious enough that the review had to reverse part of it.
   Assume this one is wrong somewhere and check.** In particular I have asserted `sync.rs` dispatches
   nine subcommands and `help.rs` documents seven — **count them yourself.**

**Stop and escalate, do not guess**, if: a claim's replacement depends on the pending `Tag` ruling; a
sibling check turns up something whose correct wording is not settled by any authority; or you find a
**fifth** class of stale claim large enough to be its own increment rather than folded into this one.
