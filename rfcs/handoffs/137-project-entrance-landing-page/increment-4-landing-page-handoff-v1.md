# RFC 137 increment 4 — build the landing page

**RFC:** `rfcs/accepted/137-project-entrance-landing-page.md` §7 increment 4.
**Base:** `main` at `ea687c9` (increment 2, pushed). **Increment 3's commit `d3124e4` is held,
unpushed, and is re-made as part of this round — see §1.**

**Design inputs, all local under `.git-exclude/` (untracked, present in this working tree):**

| Path | What it is |
|---|---|
| `tasks/architect/landing-page-20260904/prikk-landing-page-direction-20260904.md` | the owner's direction document — **binding**, especially §10 (tone), §16 (mobile), §17 (animation), §18 (accessibility), §19 (technical restraint), §20 (not documentation) |
| `tasks/architect/landing-page-20260904/index.html` | the architect's prototype |
| `tasks/architect/landing-page-20260904/draft-01/` | the owner's draft (`index.html` + `landing.css`) |
| `tasks/architect/landing-page-20260904/draft-02/prikk-attraction-story.png` | the story image, 1536x1024 |
| `reviewed/landing-page-owner-drafts-review-v1.md` | the review that ruled the merge; **§6 is the specification** |

---

## 1. Two commits in this round, and one of them is a replacement

**1.1 Correct increment 3, in a new commit.** `d3124e4` is held unpushed because it fails without a
landing page.

**Do not amend it and do not reset it out — correct it with an ordinary follow-up commit in this
round.** The architect's own handoff commit now sits above `d3124e4` on local `main`, so amending or
resetting would mean rewriting across someone else's commit; a follow-up commit needs no history
surgery from anyone. The consequence is that the pushed range will contain one commit
(`d3124e4`) that would fail the Docs workflow if it stood alone. **That is accepted deliberately**:
the range lands atomically, nothing is ever deployed from that commit, and the reason is recorded here
rather than left for someone to rediscover with `git bisect`.

Two corrections, from `.git-exclude/reviewed/site-url-and-staging-review-v1.md`:

- **Required (§3.2 there): drop the job-level `env: SITE_DIR: ${{ runner.temp }}/site`.** The `runner`
  context is not available in `jobs.<job_id>.env`, so it evaluates empty and `mkdir -p "/site"` fails
  with `Permission denied`. Use `$RUNNER_TEMP` inside the `run:` body — always present, no expression
  context needed — and publish the path with `echo "SITE_DIR=$RUNNER_TEMP/site" >> "$GITHUB_ENV"` so
  `path: ${{ env.SITE_DIR }}` still resolves. Any equivalent that removes the `runner`-context
  dependency is acceptable.
- **Correction (§1.1 there): `docs/book.toml`'s new comment says the value sets the `<base>` mdBook
  writes into "every generated page".** Measured: **only `404.html` carries one**; index and nested
  pages use ordinary relative paths, which is why the book works under any prefix at all. Fix the
  wording.

**1.2 The page itself.** Order the two commits so the workflow change and the page land together;
they are pushed together regardless.

## 2. The page — the merge, stated as a specification

`.git-exclude/reviewed/landing-page-owner-drafts-review-v1.md` §6 rules this. Restated so it is not
re-derived:

**From draft-01 (the owner's), because it is warmer:**

1. The **eyebrow rhythm** — a quiet `<p class="eyebrow">` label above each section heading.
2. **Numbered principles** `01` / `02` / `03` for Patch / Evidence / Growth.
3. The **two-column hero** — copy left, visual right.
4. The **closing bookend** — a second, softer call to action mirroring the hero.
5. `@media (prefers-reduced-motion: no-preference)` as the **opt-in wrapper** for any animation. This
   is the correct direction of the test; do not invert it to a `reduce` override.

**From the architect's prototype, because it is true:**

6. The **install block** — both commands, with copy buttons.
7. `https://github.com/prikk-vcs/prikk` — **draft-01's `nabbisen/prikk` is two migrations stale.**
8. The **real captured terminal output**, not invented output.
9. The **maturity note in the first screen** (§4.2 below).

**Dropped:** draft-01's 1.39 MB `prikk-patches-grow-and-shine.gif`. The story image says the same
thing statically at a fraction of the weight and satisfies §17's own test that the page look good with
all animation paused.

**Commands and output that may appear, because they were captured from the built binary:**

- `prikk --version` -> `prikk 0.31.0` (draft-01 had this right).
- The `prikk commit --from-worktree` transcript in the architect's prototype.
- Install: `curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh`
  and `cargo install prikk`.

**Do not invent output.** If you want to show a command not in the prototype, run it and paste what it
prints.

## 3. Links must be relative

The page is served at `/` today under `prikk-vcs.github.io/prikk/` and later at `prikk.org/`. **Every
link into the book must be relative** — `docs/guide/install.html`, never `/docs/guide/install.html`
and never an absolute `https://prikk-vcs.github.io/...` URL. A root-absolute path breaks under the
repository-path deployment; a hard-coded host breaks at the domain cutover. Relative works under both,
with no change at cutover.

External links (GitHub, crates.io) are absolute, obviously.

## 4. Claims — the part with the least room for judgement

### 4.1 Three claims must not appear, in any wording

Verified against the live `prikk --help` inventory at `ea687c9`:

| Claim, as it appeared in a draft | Why it must go |
|---|---|
| "Give each session its own workspace" / "Isolated Workspaces — Safe & Parallel" | **No workspace concept exists.** 23 commands, none implements one. Worse, `prikk --help` says in its own words: *"there is no `branch switch` yet, and no current-branch pointer"* — so the one thing a reader would try next is the one thing prikk cannot do |
| "Effortless Review — Clear Changes" | **No review command.** `merge-evidence`/`merge-plan` are read-only analysis surfaces, and the reference page calls them exactly that |
| "A next-generation VCS" | The register §10 of the direction document rules out |

**"Confident Merge" is partial, not false** — `prikk merge` exists but requires an explicit
`--baseline-block ID` and seals only a proven-confluent merge. If it appears, it must not read as a
general merge. **"Validation & Evidence — Trust Built-in" is true** and may stay.

### 4.2 Maturity goes in the first screen — a deliberate asymmetry, not an oversight

RFC 137 §5 rules this, and it runs **against** the owner's own guideline ruling that "Current Status"
and "Not a Good Fit Yet" are *secondary* in `README.md`. That ruling stands for `README.md`. On the
landing page a visitor is deciding whether to try an early-implementation VCS **before** they read
anything else, so it belongs above the fold. The architect's prototype carries it as a short
`.maturity` note under the hero; keep that shape or better.

### 4.3 The page is not documentation

Direction §20. It answers what prikk is, why it exists, what it feels like, and how to try it. Depth
goes to `docs/`.

## 5. The story image — split, with its text as HTML

**Measured: 1536x1024.** ImageMagick is available in this environment (`magick`). The layout, verified
by cropping and looking rather than read off the picture:

| Band (absolute y) | Content | Destination |
|---|---|---|
| ~0-210 | logo + headline | **drop** — the page has its own header |
| ~245-330 | stage label chips: Start / Patches / Integrated / Growing / Shining | **HTML text** |
| ~300-660 | the five illustrations | **five cropped images** |
| ~680-740 | the five captions | **HTML text** |
| ~800-910 | the four-item feature strip | **HTML text**, after §4.1 removes two of the four |

**Crop the illustrations only.** Choose each panel's top edge just below *its own* chip (the chips sit
at different heights) and verify every crop by looking at it. Panels 4 and 5 have foliage that rises
high — do not clip it. **Drop the inter-panel arrows**; layout expresses the sequence.

Report the exact crop boxes you used.

**Then:** lay the five out as a row on desktop and a **stack on mobile**, each with its chip label and
caption as real text. Direction §16 is explicit that the metaphor must work *"without requiring
horizontal diagrams"*.

**Why not the whole image:** it is 1,549,911 bytes; its text is unselectable, untranslatable and fails
WCAG 1.4.5 (Images of Text), going illegible on a phone before the illustration does; and a false
claim baked into a raster needs the image regenerated rather than a text edit.

Every panel needs a real `alt`. Decorative-only panels may use `alt=""`, but the sequence carries
meaning — prefer describing it.

## 6. Assets must live under `docs/landing/`

The staging step copies `docs/landing/.` to the site root and the built book to `docs/`. **Nothing
outside `docs/landing/` reaches the artifact.** So the logo and the five panels must be *inside*
`docs/landing/`, not referenced from `assets/logo/` at the repository root.

`assets/logo/prikk-mark-256.png` (256x256, tracked) is the mark. Copy what you need in; do not link
across.

Keep the page's own CSS in one file or inline — direction §19: no framework, minimal JavaScript, and
the copy buttons are the only script this page needs.

## 7. The gate, and three findings carried here from increment 1

**Declare the page** in `DECLARED_DOCUMENTS` (`crates/prikk-cli/src/commands/tests.rs`). Increment 1
made rule (A) able to read `<code>`/`<pre>`, and `document_text` already excludes anything under
`docs/landing/` from rule (B). Adding the entry is what finally makes increment 1 live.

Three findings from `.git-exclude/reviewed/html-code-context-review-v1.md` §4 are carried here because
this is when they first become reachable:

**7.1 `<pre>` inside a Markdown fence is double-counted.** Measured: a fence containing `<pre>...</pre>`
yields **2** regions, where a fence containing `<code>...</code>` yields 1. The `<pre>` arm pushes into
`fenced_ranges` without first checking whether it *starts inside* one; the `<code>` and backtick arms
both check. Harmless to rules (A)/(B) as written, but it contradicts the module's own precedence rule
and will surprise the next person who asserts on region counts. **Fix it — it is a three-line symmetry
fix — and add the regression test.**

**7.2 Tag matching is case-sensitive.** `<CODE>`, `<PRE>` and `<Code>` yield **zero** regions. HTML tag
names are case-insensitive by specification, so a page using `<PRE>` would pass the gate while being
checked not at all — the vacuous-pass mode this whole increment chain exists to prevent, one level
down. **Decide and report**: either make matching case-insensitive on the tag name, or add a check
that the declared landing page contains no uppercase `<CODE`/`<PRE` opening tag. **Do not leave it
undecided** — and if you leave it as-is, say why in the report rather than silently.

**7.3 `clippy::indexing_slicing` is allowed module-wide for five test assertions.** That lint is
`deny` at workspace level (`Cargo.toml:70`). Removing the allow names exactly five `regions[0]` sites,
all in increment 1's new tests, each already guarded by a preceding `assert_eq!(regions.len(), 1, ...)`
— none in the implementation. `expect_used` is already allowed in this module, so
`regions.first().expect(...)` needs no exemption at all. **Narrow it: remove the module-wide allow.**

## 8. Verify, do not assume

1. **Open the page in a browser and narrow the window**, at least to 360px. The install rows and the
   panel row both have to survive it. A previous round of this page shipped a horizontal-scroll fix
   that did not work, found only by looking.
2. **Every command named on the page** is a real registry entry — the gate now proves this, so run it.
3. **Every relative link resolves** against the staged tree from §1, not just on disk.
4. **Keyboard focus is visible** on every interactive element — the copy buttons and every link.
   draft-01's stylesheet has **zero** `:focus` rules; direction §18 requires visible focus.
5. **The page renders correctly with animation disabled** (§17's own test) and in both light and dark,
   if you offer a dark palette.
6. **Total page weight**, reported as a number, including the five panels.

## 9. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, against the exact final commit:
`cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked --
-D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**Report the `prikk` test count before and after.** §7.1 and §7.3 both change test code; a flat count
means §7.1's regression test is missing.

Cross-target clippy only if your own diff introduces `#[cfg(target_os)]` — check the diff.

## 10. Out of scope

`homepage` -> `https://prikk.org/` and the `site-url` cutover (increment 5, together); the domain, DNS
and certificate; any change to `README.md` or `docs/src/index.md`; and the workspace concept
(`.git-exclude/tasks/architect/010-20260818-01-...md`), which §4.1 records as claimed-but-not-built and
which nothing here schedules.

## 11. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
and write the report to `.git-exclude/review-request/`. Include §5's crop boxes, §7.2's decision and
its reasoning, §8's six verification results with the weight as a number, and every departure.

**Both commits stay local.** The architect pushes the workflow change and the page together.
