# `.prikkignore` — the documentation it shipped without

**Authority:** `ROADMAP.md`'s Post-0.16.1 Documentation Reference Backlog, **TASK-17**, prioritized by
the project owner on 2026-09-03. RFC 124 is closed (`rfcs/done/124-worktree-ignore-mechanism.md`);
this handoff adds no behaviour to it.
**Base:** current `main` (`472dcab`). **Under `003-landing-work-on-main.md`.**

**Why this is prioritized over RFC 126 §5 and AUD-04.** `.prikkignore` shipped in `0.29.0` and is the
newest thing a user will reach for. Right now the only prose describing it is a module doc in
`crates/prikk-store/src/ignore.rs`, which no user reads. **A feature nobody can find is not delivered.**

---

## 1. Scope

**One new guide page, one `SUMMARY.md` entry, two cross-links, and one help line per affected
command.** No behaviour change to the mechanism itself. If writing the page makes you want to change
the mechanism, **stop and report that instead** — it is a closed RFC and a shipped release.

**`docs/src/guide/ignore.md`**, listed in `docs/src/SUMMARY.md` under `# Guide`. Place it adjacent to
`Worktree Status` and `Worktree Patch Authoring` — those are the two commands it binds — and say in
your report why you put it where you did; nothing gates `SUMMARY.md` against the filesystem, so an
unlisted page is simply invisible.

## 2. The one authority for content

**`crates/prikk-store/src/ignore.rs`'s module doc is the source. Derive from it; do not restate it
from this handoff** — I am summarising it below to tell you what the page must cover, and my summary
is the thing most likely to be wrong.

The page must state, in the user's terms:

1. **Where the file lives and what a rule is.** `.prikkignore` at the repository root. One rule per
   line, each a **literal repo-relative path prefix**. `target` matches `target` and everything under
   it — and **never `target2` or `targetfoo`**, because matching is by whole path component.
2. **What the syntax deliberately does not have.** No globbing, no negation, no comments, no
   per-directory files. **Say plainly that `*.log` does not work.** RFC 124's reasoning is worth one
   sentence: a syntax that *nearly* matched gitignore's semantics would be worse than one that
   obviously does not attempt to.
3. **Where it binds: discovery only.** `commit`'s worktree walk and `worktree-status`'s untracked
   scan. Nothing else.
4. **Where it does not bind, and why.** Applying, replaying, verifying, and materializing history
   ignore it entirely. **The reason is the interesting part and belongs on the page**: otherwise two
   repositories with different ignore files would disagree about the same signed history.
5. **A rule can never hide an already-tracked path**, or one under an already-tracked path — so
   adding a line cannot make `commit` see an existing file as deleted and author a `DeleteNode`.
6. **A malformed file is refused, not treated as empty.** An absent file is not malformed: no
   `.prikkignore` means no rules, and every repository created before `0.29.0` behaves exactly as it
   did.
7. **It is an ordinary tracked file, not configuration.** It is committed, signed, and travels through
   `bundle` and `sync` — so a received history carries the sender's rules, which shape the receiver's
   *future* commits and nothing about the patches already in hand.

**Worth one line because it is the practical reason the feature works at all:** an ignored directory
is skipped without being descended into. That is not only speed — `commit`'s walk fails closed on
symlinks and unsupported entry kinds, and a real `node_modules/` is full of exactly those, so without
directory-level pruning, ignoring it would not actually let such a project commit.

## 3. Honest limits — this project's standing documentation discipline

Every reference and guide page here carries its limits rather than implying completeness. **Two are
mandatory:**

- **No globbing or negation** (§2.2 above), stated as a current limit, not as a roadmap promise.
- **A file swept into history by mistake cannot be removed later.** `.prikkignore` prevents future
  capture; it does nothing about what is already committed. **`README.md:67` already words this;
  read that bullet and stay consistent with it rather than inventing a second phrasing.**

**Do not promise gitignore compatibility, a config file, or per-directory rules** — RFC 124 refused
all three, and `main.rs:344` defers a config file deliberately.

## 4. Cross-links

Add a link to the new page from **`docs/src/guide/worktree-status.md`** and from
**`docs/src/guide/patches/worktree-patch.md`** — the two commands whose behaviour the file changes. A
reader who has landed on either page and is wondering why a file was or was not scanned should not
have to search for the answer.

## 5. The help surface

`commit` and `worktree-status` say nothing about `.prikkignore` today. **Add one line to each**, in
the single `COMMANDS` table's `help_lines` (`crates/prikk-cli/src/commands.rs`) — that table feeds both
`prikk --help`'s inline rendering and `prikk <command> --help`, so one edit reaches both, and adding a
second table is what RFC 118 forbids.

**One line each, naming the file and what it does — not the syntax.** The guide page is where syntax
belongs; help text that grows a tutorial is how it stops being read.

`crates/prikk-cli/tests/rfc121_command_help.rs` exercises this surface. **Check whether it pins
content that your line changes**, and if it does, update it deliberately rather than discovering it in
the gate run.

## 6. Verify what I have told you

**Two claims in this handoff are mine and should be checked before you build on them:**

1. **That `commit`'s and `worktree-status`'s walks are the only two binding sites.** Confirm from
   `ignore.rs` itself and its call sites, not from this file.
2. **That nothing gates `SUMMARY.md` against the pages on disk.** I looked and found no such check; if
   one exists and I missed it, say so.

## 7. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run from there against your final commit — **not
reproduced here**: `reference-check` treats a policy-command line outside its registered sites as an
`unregistered-reference`.

**`mdbook build` applies to this increment** — it is the one that has been skipped as "no docs page
changed" for several increments running, and this one changes `docs/src/`. `docs-pr.yml` builds the
book on `docs/**`, and `docs.yml` deploys on push to `main`, so a broken page reaches the published
site.

Local commits on `main`; **no push, no tag, no publish.** Report to `.git-exclude/review-request/`,
and state:

1. Where you placed the page in `SUMMARY.md` and why.
2. The result of both checks in §6.
3. Whether `rfc121_command_help.rs` needed updating.
4. Anything in §2's summary that `ignore.rs` contradicts. **That list is my paraphrase of a module
   doc, and this project's handoffs have a consistent record of getting such details wrong.**
