# RFC 137 increments 2-3 — the book's `site-url`, and the Pages staging step

**RFC:** `rfcs/proposed/137-project-entrance-landing-page.md` §7 increments 2 and 3.
**Base:** `main` at `3126a24` (increment 1, accepted and pushed).

**Read §1 before planning the work: one of these two increments cannot be landed on its own, and the
RFC's own §7 was wrong about the other.** Both were established by building the thing and looking,
not by reading documentation.

---

## 1. Two corrections to RFC 137 §7, measured against mdBook 0.5.4

**1.1 `site-url` does not do what the RFC says it does — but it is still required.** §7 claims that
without it *"the book's generated 404 page loads its assets from the site root and renders
unstyled"*. The asset references are **relative in both cases**; the mechanism is a `<base>` tag:

| `book.toml` | `404.html` contains | Asset refs |
|---|---|---|
| no `site-url` | `<base href="/">` | `css/general-*.css` (relative) |
| `site-url = "/docs/"` | `<base href="/docs/">` | `css/general-*.css` (relative) |

So the **effect** the RFC describes is real — a 404 served at `/docs/guide/nope.html` resolves its CSS
against `<base href="/">` and gets `/css/general-*.css`, which does not exist — but the cause is the
`<base>` tag, not absolute hrefs. State the mechanism correctly in whatever comment you leave.

**1.2 The value is host-dependent, and `/docs/` would be wrong today.** `<base>` takes a path from the
**host** root, not the site root:

| Deployment | Correct `site-url` |
|---|---|
| `prikk-vcs.github.io/prikk/docs/` — **today** | `/prikk/docs/` |
| `prikk.org/docs/` — after the domain | `/docs/` |

**RFC 137 §7's claim that increments 1-4 are all "correct against the current deployment" does not
hold for this one.** Use `/prikk/docs/` now.

## 2. Increment 2 — `site-url` in `docs/book.toml`

Add to the existing `[output.html]` table:

```toml
site-url = "/prikk/docs/"
```

**Leave a comment naming the cutover**, because the value must change when the domain goes live and
nothing enforces it. Increment 5 (`homepage` → `https://prikk.org/`) is the companion step; say so in
the comment so whoever does that change meets this one.

**An alternative was considered and is not chosen.** mdBook honours the environment override
`MDBOOK_OUTPUT__HTML__SITE_URL` (verified: it produced `<base href="/prikk/docs/">`), so the workflow
could derive the value from `actions/configure-pages`' outputs and never need a cutover edit. It is
not chosen because `base_path` is `/` for an apex custom domain, so the obvious
`${{ steps.pages.outputs.base_path }}/docs/` yields `//docs/` after the cutover — a trap that would
surface exactly when nobody is looking for it. **If you prefer the env-var route, you must solve that
case explicitly and show it in the report; otherwise take the literal value.**

**Verify, do not assume:** build the book and grep `docs/book/404.html` for `<base`. It must read
`<base href="/prikk/docs/">`. Report the actual line.

## 3. Increment 3 — the staging step, and the constraint that governs it

### 3.1 What it does

`docs.yml` uploads `docs/book` as the whole site. It must instead upload a directory holding **the
landing page at the root** and **the built book under `docs/`**.

Stage outside the repository tree — `${{ runner.temp }}` — so nothing new appears under `docs/`, no
`.gitignore` entry is needed, and a local run cannot leave a third directory beside `src/` and
`book/`. Shape (adapt to the job's `working-directory: docs` default, which applies to `run:` steps):

```
<temp>/site/            <- docs/landing/ contents
<temp>/site/docs/       <- docs/book/ contents
```

then point `upload-pages-artifact` at `<temp>/site`.

`docs.yml`'s `paths:` filter is already `docs/**`, so `docs/landing/` is covered and **needs no filter
change** — confirmed, not assumed.

### 3.2 BINDING: increment 3's commit must not reach `origin/main` before increment 4's

**Landed alone, this increment breaks the published documentation site.** It moves the book from the
artifact root to `/docs/` while `docs/landing/` does not yet exist, so
`https://prikk-vcs.github.io/prikk/` would serve **nothing** — the project's live entrance, 404, for
however long increment 4 takes.

**Therefore:** commit increment 2 and increment 3 separately as usual and report them, but state in
the report that **increment 3 is not releasable until increment 4 exists.** The architect will hold
increment 3 unpushed and push it together with increment 4. Do not attempt to work around this by
adding a placeholder landing page — a thin placeholder would be publicly served as the project's front
page, which is worse for a visitor than the documentation index that is there now.

**If you see a way to make increment 3 safe standalone that does not put a placeholder in front of
visitors, report it rather than implementing it** — that is a sequencing decision, not an
implementation one.

### 3.3 What to verify locally, since CI cannot be run

1. **Build the staged tree by hand** with the same commands the step will use, and show the resulting
   layout: the landing directory's files at the root, `docs/index.html` present, `docs/guide/...`
   present.
2. Because `docs/landing/` does not exist yet, **use a throwaway directory to stand in for it** and
   say so in the report. Do not create `docs/landing/` in the commit.
3. Confirm the book's internal links still work from `/docs/` — mdBook's links are relative, so they
   should, but check one nested page's CSS reference rather than assuming.

## 4. Out of scope

The landing page itself and its `DECLARED_DOCUMENTS` entry (increment 4); `homepage` (increment 5);
anything about the domain, DNS or the certificate.

**Three findings from increment 1's review are carried to increment 4, not here** — do not fix them in
this round: `<pre>` inside a Markdown fence is double-counted; `<CODE>`/`<PRE>` are invisible because
tag matching is case-sensitive; and `clippy::indexing_slicing` was allowed module-wide where
`regions.first().expect(...)` would have needed no exemption.

## 5. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**`reference-check` matters more than usual here**: it scans every `.yml` in the repository, and this
increment edits a workflow. Run it and report the result explicitly rather than as part of a list.

Also run `mdbook build docs` and report its output. Cross-target clippy is not applicable unless your
own diff introduces `#[cfg(target_os)]` — check the diff, do not infer it from the change's shape.

## 6. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
and write the report to `.git-exclude/review-request/`. Include the actual `<base ...>` line from the
built 404 page, the staged-tree listing from §3.3, an explicit statement of §3.2's release constraint,
and anything that departed from this handoff.
