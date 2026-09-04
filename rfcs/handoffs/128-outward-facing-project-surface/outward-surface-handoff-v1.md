# RFC 128 §3–§5 — the three items the outward-facing surface still lacks

**Authority:** `rfcs/proposed/128-outward-facing-project-surface.md` **§3, §4, §5**, with §6's
constraints binding. §2's `SECURITY.md` shipped in `0.28.0`.
**Base:** current `main`. **Under `003-landing-work-on-main.md`.**

**Three independent items.** They may land as one commit each; nothing sequences them. **§5 is the
largest and the audit called it the highest-leverage single page this project could add.**

---

## 1. `CONTRIBUTING.md` (§3)

**Confirmed absent** at root and in `.github/` — GitHub's contributor UI finds neither, and
`docs/src/contributing/development.md` is invisible to it.

**The owner has already ruled the duplication question** — *"Duplicate is allowed, because reader can
access to docs from each"* — so this may restate the existing guidance rather than be a link stub.

**What it must add beyond that page**, and this is the whole point of the file: **how work is reviewed
here.** An outside contributor cannot guess that this project runs an architect-review discipline
against a fixed gate set, and that **a drive-by pull request is not the expected shape of a
contribution.** Say that plainly and without apology; it is unusual and it is better learned before
someone spends a weekend.

**Do not invent process.** Describe what actually happens — handoffs, review rounds, the gate set in
`rfcs/EXECUTION-ORDER.md` §6 rule 9 — and if you find yourself writing a rule nobody follows, stop and
report it instead.

## 2. Crate metadata (§4)

**Eight crates publish to crates.io on every release and seven present as uncategorized, unkeyworded
libraries with no documentation link.** Verified at the current commit:

| Field | State now |
|---|---|
| `categories` / `keywords` / `homepage` | present on **`prikk-cli` only**; the other seven inherit none |
| `documentation` | **absent from all nine manifests** |
| `categories = ["algorithms"]` | wrong crates.io slug for a VCS |
| `keywords = ["vcs"]` | 1 of 5 slots used |
| `tools/release-policy` | omits `readme` although the file exists |

**The RFC's own suggestions**: `development-tools`, `command-line-utilities` for categories;
`version-control`, `dvcs`, `patch`, `merge` for the free keyword slots.

**Check the slugs against crates.io's own category list before using them** — a category string that
is not a real slug is silently ignored, which is the same failure as not setting it. **Report the
list you checked against.**

**`tools/release-policy` and `tools/benchmarks` are `publish = false`** — decide deliberately whether
they get metadata at all, and say which you chose and why. The `readme` omission is worth fixing
regardless, since it is a plain inaccuracy.

## 3. The Git→prikk mapping page (§5)

**`prikk`'s vocabulary collides with Git's**, and that is the reason this page exists: `commit` does
not publish, there is no `HEAD`, no staging area, no branch switching, and `seal` has no Git
counterpart at all. **A reader who maps the words onto Git's meanings is wrong about five things at
once and does not know it.**

**Minimum content**, per §5:

- **The command correspondence table.** The audit's own §4 matrix is the first one that has ever
  existed and can seed it.
- **The five conceptual deltas**: no staging; no `HEAD` or switching; `commit` versus `seal`; messages
  not yet stored (RFC 123); file-based distribution instead of remotes.

**§6's constraints are binding:**

- **State limits at the limit site.** A row saying a Git verb has no counterpart says so *where the
  verb is named*, not in a footnote.
- **Every command named on the page must exist.**

**Explicitly not RFC 113 and not an importer.** This page explains the model to a human and is useful
immediately, with no migration tooling in existence. Do not promise one.

## 4. The gate this page must join — and four pages that already escaped it

§6 requires the new page be added to **`DECLARED_DOCUMENTS`**
(`crates/prikk-cli/src/commands/tests.rs:35`), so rule (A)
(`rule_a_every_documented_command_names_a_real_registry_entry`, `:215`) checks mechanically that every
`prikk <command>` it names is real.

**While preparing this handoff I found four pages that mention commands in code context and are not
declared:**

```
docs/src/guide/ignore.md            `prikk commit`  `prikk worktree-status`
docs/src/guide/faq.md               `prikk branch`  `prikk init`
docs/src/guide/troubleshooting.md   `prikk doctor`  `prikk trust`
docs/src/reference/durability-recovery.md   `prikk bundle`
```

**`ignore.md` is ours** — added by TASK-17 four days ago, and the architect's review of that increment
checked `SUMMARY.md` and not this list. The other three predate it.

**Add all four alongside the new page**, and **report whether rule (A) then passes for each** — if a
page names a command that is not in the registry, that is a real finding and it belongs in the report,
not in a quiet fix. The list is 33 entries against 43 pages on disk, and the comment above it explains
the list is deliberately declared rather than globbed, so **adding entries is the intended maintenance,
not a workaround.**

## 5. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit — **not reproduced here**.

**`mdbook build` applies** — §3 and §5 add pages, and §5's must be listed in `docs/src/SUMMARY.md` or
it is invisible to the book's own navigation. Nothing gates `SUMMARY.md` against the filesystem.

**Run both cross-target clippy commands** whether or not this diff contains `#[cfg(target_os)]` — the
question is whether anything you add has consumers only behind one.

Local commits on `main`; **no push, no tag, no publish.** Report to `.git-exclude/review-request/`,
stating: the crates.io category slugs you verified against; your `publish = false` metadata decision;
rule (A)'s result for each of the five newly declared pages; and every place this handoff's claims
proved wrong.
