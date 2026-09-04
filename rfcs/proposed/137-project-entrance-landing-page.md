# RFC 137 — The project's entrance: a landing page, and how it stays true

**Status.** **PROPOSED, 2026-09-04.** Opened at the project owner's instruction after a design
discussion the same day that settled the whole option space. **This RFC records decisions already
made rather than re-opening them** (§3), and contributes the two things the discussion did not
settle: **how a landing page stays true** (§4) and **what it may say that the other two entrance
surfaces do not** (§5).

**Coupled to RFC 135 (first-run entrance and configuration), deliberately.** The landing page ends at
an install command; RFC 135 begins at what a new user meets once that command has run. They are two
halves of one arrival, and §8 states the seam.

**Author-review independence.** The architect wrote this RFC and is also its only reviewer — the
standing gap on every architect-authored design here. Compensated at implementation review.

**Tracks.** The published entrance. **No shipped-code behaviour change is proposed.**

---

## 1. The problem

**A visitor arriving at this project's web address is shown a table of contents.**
`docs/src/index.md` — 19 lines — opens with "Prikk Documentation", a paragraph of definition, a note
on the name's Norwegian etymology, and links. It is a good documentation index. It is not a front
door, and it is currently doing the job of one.

The owner raised this first about `README.md`, in a form that applies with more force to the web
entrance:

> Our "Quick Start" in `README.md` is not quick at all... They are generally unfamiliar with visitors.
> I doubt it makes visitor feel uneasy and brings their withdrawal.

`README.md` has since been slimmed 323 → 161 lines. The web entrance has not been touched, and it is
the surface a link from anywhere else in the world lands on.

## 2. Why this is its own RFC rather than a task

Three reasons, in order of weight:

1. **It creates a published surface with no gate.** Every other document this project publishes is
   Markdown, and 40 of them are mechanically checked against the live command registry. A landing page
   is HTML, and **no gate in this repository can read it** (§4). That is a design question, not a task.
2. **It changes what the project's URLs mean**, including one that is immutable (§6).
3. **It is a third statement of the same claims**, and this project has already ruled on duplication
   in a way that does not obviously extend to it (§5).

## 3. Decisions already made — recorded, not re-opened

Settled by the owner across the 2026-09-04 discussion
(`.git-exclude/reviewed/landing-page-hosting-shape-v1.md`,
`.git-exclude/reviewed/landing-page-owner-drafts-review-v1.md`):

| Decision | Value |
|---|---|
| Domain | `prikk.org` (owner acquiring) |
| Landing page URL | `/` |
| Documentation URL | `/docs/` — **subdirectory**, not `docs.prikk.org` |
| Repositories | **one**; one Pages site, one CNAME, one workflow |
| Landing page source | `docs/landing/` |
| mdBook root | `docs/` — **unchanged**; no rename |
| `build.build-dir` | unchanged — declined, with reasons recorded |
| Page content | the two 2026-09-04 drafts **merged**, not chosen between |
| Story image | used, **split into panels with its text as real HTML** |
| The 1.39 MB GIF | dropped |

**The single reason the subdirectory won**, restated because it is this RFC's own thesis: it keeps the
landing page inside the repository that gates everything else. A subdomain needs a second Pages site,
therefore a second repository, therefore a page with no CI, no `release-policy`, and no gate — the
surface most likely to carry a claim nobody re-checks.

## 4. How the landing page stays true — the design content of this RFC

### 4.1 The evidence that this is a real problem, not a theoretical one

Two landing-page drafts were written on 2026-09-04. Reviewed against the live command inventory, they
contained between them:

| Claim | Reality |
|---|---|
| "Give each session its own workspace" (draft-01 prose) | **No workspace concept exists.** 23 commands; none implements one |
| "Isolated Workspaces — Safe & Parallel" (draft-02 image) | Same. And `prikk --help` states in its own words: *"there is no `branch switch` yet, and no current-branch pointer"* |
| "Effortless Review — Clear Changes" (draft-02 image) | **No review command.** `merge-evidence`/`merge-plan` are read-only analysis surfaces |
| "Confident Merge" (draft-02 image) | Partial — `prikk merge` requires an explicit `--baseline-block ID` and seals only a proven-confluent merge |
| `https://github.com/nabbisen/prikk` ×3 (draft-01) | Two migrations stale; RFC 129 moved to `prikk-vcs/prikk` at `c69f5a9` |

**Three false claims and a stale URL, in two drafts written the same week, by two authors.** The
landing page is the surface where aspiration is most natural and least checked. That is the problem
this RFC exists to solve.

### 4.2 The gate cannot read HTML, and declaring the page anyway is worse than not declaring it

`code_regions` (`crates/prikk-cli/src/commands/tests.rs:128-166`) finds command mentions by scanning
for ``` fences and `` ` `` inline spans. Rule (A) then checks every `prikk <token>` in those regions
against the live `COMMANDS` registry.

**An HTML page has neither.** It writes `<code>prikk seal</code>`. Adding `docs/landing/index.html` to
`DECLARED_DOCUMENTS` today would make `code_regions` return zero regions, `command_tokens` return zero
tokens, and **rule (A) pass vacuously** — a gate that reads as coverage in the declaration list while
checking nothing, so the next reader sees it declared and stops looking.

`release-policy`'s `command_scan` does not help: it walks `.md`/`.yml`/`.yaml`/`.sh`, and `.html` is
not in that set.

### 4.3 Ruled: extend `code_regions` to recognise `<code>` and `<pre>`

**Accepted by the owner 2026-09-04**, over the two alternatives:

| Option | Why not |
|---|---|
| Generate the HTML from a declared `.md` | Needs a build step this project does not have; §19 of the owner's own direction argues against tooling for one page |
| Leave it ungated, check by hand at each release cut | §4.1 is what manual checking of this surface produces |

**It must land before the page does.** Same ordering argument DC-90 used for its own gate: a boundary
added afterwards documents what happened instead of constraining it.

**What this does and does not buy.** Rule (A) checks that every `prikk <command>` named on the page is
a real registry entry. It does **not** check prose claims — "workspace", "review", "parallel" name no
command and would pass. **The gate closes the command half of §4.1's table and none of the rest.**
Stating that plainly is part of the design: an unstated limit in a gate is how a vacuous assertion
gets believed.

The remaining half is covered by §5's rule and by the release-cut checklist, not by machinery.

## 5. Three entrance surfaces, and what each may say

After this change the project has three front doors:

| Surface | Reader | Arrives from |
|---|---|---|
| `docs/landing/index.html` at `/` | someone who has not decided to try prikk | a link, a search, crates.io `homepage` |
| `docs/src/index.md` at `/docs/` | someone who has decided, and wants to do something | the landing page, or a deep link |
| `README.md` | someone reading the code | GitHub, a fork, a clone |

**The owner's standing duplication ruling — *"Duplicate is allowed, because reader can access to docs
from each"* — permits overlap and does not require it.** It was made about documentation pages that
each need to stand alone. A landing page that restates `README.md` is not wrong by that ruling; it is
merely wasted, because its reader has not asked the question `README.md` answers.

**Proposed division, as a rule that can be applied rather than a preference:**

- **The landing page answers "what is this and why would I want it", and ends at one install
  command.** It states nothing a reader would need to verify, and makes no claim naming a capability
  (§4.1's failure mode). Its own direction document already says it: *"The landing page is not
  documentation."*
- **`/docs/` answers "how do I do the thing"** and remains the index it is.
- **`README.md` answers "what is this repository"** — crates, gates, current state — for a reader who
  is already inside it.

**The one claim the landing page must carry that the others need not:** what prikk is *not yet*. The
owner's guideline ruling placed "Current Status" and "Not a Good Fit Yet" as **secondary** in
`README.md`; on the landing page, a visitor deciding whether to try an early-implementation VCS needs
that in the first screen, not the footer. The architect's own draft carries it as a short
`.maturity` note under the hero. **This is a deliberate asymmetry between the three surfaces, not an
inconsistency.**

## 6. What the URL change costs — measured

Setting a custom domain makes GitHub redirect `prikk-vcs.github.io/prikk/<path>` to `<domain>/<path>`.

| Link | Count | Editable? |
|---|---:|---|
| book root | 6 in-repo, **plus `homepage` in every published crate version on crates.io** | in-repo yes; **published versions: never** |
| `reference/release-compatibility.html` | 2 | yes |
| `guide/ignore.html` | 1 | yes |

**Only three non-root deep links exist, all in files we control.** The one immutable link is the book
root, baked into `homepage = "https://prikk-vcs.github.io/prikk/"` (`Cargo.toml:33`) across eight
crates and every released version.

**The subdirectory layout improves that link rather than breaking it:** the redirect sends the book
root to `prikk.org` — the landing page. A visitor clicking "Homepage" on crates.io arrives at the
project's front door, which is what that field means. Under a subdomain they would arrive at a
documentation index instead.

**`homepage` should be updated to `https://prikk.org/` at the next publish.** Already-published
versions keep the old value and redirect correctly; nothing needs migrating.

## 7. Increments

Ordered. Each is independently reviewable; **1 gates 4.**

1. **Extend `code_regions` for `<code>`/`<pre>`** (§4.3). Must not change what it finds in the 40
   Markdown documents — that is the review's negative control.
2. **`book.toml` gains `site-url = "/docs/"`.** Without it the book's generated 404 page loads its
   assets from the site root and renders unstyled. One line, silent failure if skipped.
3. **`docs.yml` gains a staging step** — landing page at the artifact root, built book under `docs/`.
   Its `paths:` filter already covers `docs/**`, so `docs/landing/` needs no filter change.
4. **Build the page** at `docs/landing/`: the two drafts merged, the story image split into panels
   with HTML captions, the GIF dropped, the repository URL corrected, and the three false claims of
   §4.1 removed. Declare it in `DECLARED_DOCUMENTS`.
5. **`homepage` → `https://prikk.org/`** in `Cargo.toml`, at whatever release publishes next (§6).

**Not blocked on the domain.** 1-4 are all correct against the current `prikk-vcs.github.io/prikk/`
deployment; only 5 waits.

## 8. The seam with RFC 135

The landing page's last instruction is an install command. **RFC 135 owns everything after it** — what
a new user meets before anything works, including that none of the 23 commands generates or derives a
key.

**The consequence for this RFC:** the landing page must not promise a first-run experience RFC 135 has
not built. Its install section ends at *installed*, not at *working*. If RFC 135 later produces a
`prikk setup`-shaped entrance, the landing page gains one line and no more.

## 9. Scope

**In:** the entrance problem, the truth mechanism, the three-surface division, the measured URL cost,
and §7's five increments.

**Out:** visual design (settled in the drafts review, not re-litigated here); the domain, DNS and
certificate, which are the owner's; any change to `README.md`; any change to `docs/src/index.md`
beyond its unchanged role; and the workspace concept (`010-20260818-01`), which §4.1 records as
claimed-but-not-built and which this RFC does not schedule.
