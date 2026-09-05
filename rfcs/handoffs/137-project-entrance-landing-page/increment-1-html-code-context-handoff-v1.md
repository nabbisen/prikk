# RFC 137 increment 1 — teach the currency gate to read HTML code context

**RFC:** `rfcs/accepted/137-project-entrance-landing-page.md` §4.3, §7 increment 1.
**Base:** `main` at `de192c5`.
**Gates this increment:** increment 4 (the landing page itself) must not land before this does.

**One sentence:** `code_regions` finds `prikk <command>` mentions by scanning Markdown fences and
backticks; the landing page will be HTML, which has neither, so a declared HTML page would pass rule
(A) **vacuously**. Teach it `<code>` and `<pre>`.

---

## 1. Why now rather than with the page

A gate added after the artifact documents what happened instead of constraining it — the ordering
argument DC-90 used for its own `unsafe` boundary. RFC 137 §4.1 is the concrete reason: two landing
page drafts written on 2026-09-04 carried three false capability claims and a stale repository URL
between them. The page must meet a gate that already works, not acquire one afterwards.

## 2. What exists today

`crates/prikk-cli/src/commands/tests.rs`:

- **`code_regions(text) -> Vec<&str>`** (`:128`) — collects ``` fenced blocks first, recording their
  byte ranges, then collects `` ` `` inline spans **skipping any that start inside a recorded fence**,
  so a fenced `prikk x` is never also scanned as an inline span.
- **`command_tokens(region) -> Vec<&str>`** (`:173`) — every distinct token immediately after
  `"prikk "`: a run of ASCII alphanumerics/hyphens starting with a letter, so `--version` never
  matches.
- **Rule (A)** (`:228`) reads each declared document's **raw** text and asserts every token names a
  real `COMMANDS` entry.
- **Rule (B)** (`:255`) uses `is_explained` (`:244`), which reads through **`document_text`** (`:215`)
  — the one place `README.md`'s `## Useful Commands` section is stripped, so a mention in that bare
  listing does not count as an *explanation*.

**That (A)/(B) asymmetry is the precedent this increment reuses. Read it before designing.**

## 3. The measurement that makes the uniform design safe

**Neither `README.md` nor any file under `docs/src/` contains `<code>` or `<pre>` — zero
occurrences.** Verified at `de192c5`:

```
grep -rhoE "<code|<pre" README.md docs/src/ | sort | uniq -c    # no output
```

**Therefore extending `code_regions` uniformly — one definition of code context for every declared
document, not a `.html`-only branch — cannot change what it finds in any of the 40 declared Markdown
documents today.** Re-derive this yourself before relying on it; if it no longer holds, stop and
report, because then the change has a Markdown-side effect this handoff did not price.

Prefer the uniform definition. A format-conditional rule means two definitions of "code context" that
will drift, and mdBook permits raw HTML in Markdown, so a `.html`-only branch would be wrong the first
time a `.md` page uses `<code>`.

## 4. What to implement

**4.1 Extend `code_regions` with two HTML region kinds**, following the existing fenced/inline
precedence exactly:

- `<pre>…</pre>` and `<code>…</code>`.
- **Opening tags carry attributes.** The landing page drafts use `<code id="c1">` and
  `<pre class="term">`. Match `<code` / `<pre` followed by `>` or by whitespace-then-attributes-then
  `>` — not the bare literal `<code>`.
- **`<pre><code>…</code></pre>` is the common form and must not be scanned twice.** Same shape as the
  existing fence-then-inline ordering: record `<pre>` ranges first, then collect `<code>` spans that
  do not start inside a recorded `<pre>` range. A duplicated region is not a correctness bug for rule
  (A) (the same token asserted twice), but it is for anything that ever counts occurrences, and the
  existing code already declines to do it.
- **Unterminated tags:** the fenced arm treats an unterminated fence as running to end-of-text and
  stops. Decide the HTML arm's behaviour deliberately and state it in the doc comment; matching the
  fenced arm is the obvious choice, and being deliberate about it is what matters.

**4.2 Do not change `command_tokens`.** The landing page writes `prikk seal` as literal text inside
the tags. **Check whether an HTML entity can appear inside a token** (`&amp;`, `&lt;`) — command names
are ASCII alphanumerics and hyphens, so the answer is expected to be no, but confirm it rather than
assume it, and record the answer.

**4.3 The landing page must be checked by (A) and must not count for (B).** The precedent is §2's
`README.md` handling.

A landing page names commands; it does not explain them. If it counted for rule (B), a command
mentioned **only** there would read as documented, and a real documentation gap would close on a
marketing sentence. Extend `document_text`'s `README.md` special case into something that also
returns nothing explanatory for the landing page — the mechanism is yours to choose; the property is
binding.

**Do not add the landing page to `DECLARED_DOCUMENTS` in this increment.** The file does not exist
yet, and `every_declared_document_exists` would fail. Increment 4 declares it.

## 5. Controls — do these, and report what each produced

The architect will run their own; these are the ones this change specifically needs.

1. **The negative control that matters most: rule (A) must find exactly what it finds today across the
   40 Markdown documents.** Capture the set of `(document, token)` pairs before and after the change
   and diff them. **Any difference is a finding**, not a nuisance — report it rather than absorbing
   it.
2. **A positive control proving the new arm bites.** Add a temporary fixture containing
   `<code>prikk notacommand</code>` and confirm rule (A) fails. Then `<pre><code>prikk notacommand</code></pre>`
   and confirm it fails once, not twice. Remove the fixture; say in the report that you did.
3. **Attribute forms.** `<code id="x">`, `<code class="a b">`, `<pre class="term">` all recognised;
   `<codex>` and `<precise>` **not** matched as opening tags — the prefix trap.
4. **Rule (B) unchanged.** The set of commands considered explained must be identical before and
   after. `DECLARED_UNDOCUMENTED` must not need a new entry; if it does, something regressed.

## 6. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**Report the `prikk` test count before and after.** This increment should add tests; a flat count
means the new arm is unproven.

Cross-target clippy is **not** required unless your own diff introduces `#[cfg(target_os)]` — and
check that per-diff rather than assuming, since this project has twice been caught by a dependency on
a pre-existing gate elsewhere.

## 7. Out of scope

`book.toml`'s `site-url` (increment 2), the `docs.yml` staging step (increment 3), the landing page
itself and its `DECLARED_DOCUMENTS` entry (increment 4), and the `homepage` field (increment 5).

**Rule (A) checks command names, not prose.** This increment does not, and cannot, catch "workspace"
or "review" — RFC 137 §4.3 states that limit deliberately. Do not widen scope to attempt it.

## 8. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
and write the report to `.git-exclude/review-request/`. Include §5's four control results, the §4.2
entity answer, and anything that departed from this handoff — the procedure doc's own note is that
departures reported have been among the most useful findings in this project.
