# DC-77 Docs Mermaid Rendering — Handoff v1

**Cleared to start.** Accepted by the project owner 2026-08-08, at
`rfcs/accepted/DC-77-DOCS-MERMAID-RENDERING.md`. **Authored by** the architect.
**Size:** small. **Docs and CI only — no product crate is touched.**

## 1. What to do

1. `book.toml` — add `[preprocessor.mermaid]`.
2. `mdbook-mermaid install docs` — vendors `mermaid.min.js` and `mermaid-init.js` and wires
   `additional-js`. **Commit the assets.**
3. `.github/workflows/docs.yml` — install `mdbook-mermaid`, **version-pinned and `--locked`**, mirroring
   the existing `mdbook` line's discipline (`--vers "^0.5" --locked`).
4. `tools/release-policy/src/command_scan/procedure.rs` — **one exact entry** in the `install` arm.

## 2. The part that is actually under review

Steps 1–3 are mechanical. **Step 4 is a change to a security control**, and it is the reason this is an
increment rather than a config edit.

`boundary/publication.rs:41` scans everything under `.github`, `scripts`, and `release`.
`procedure.rs:148`'s `install` arm currently accepts **exactly one** argument vector — the existing
`mdbook` install — and returns `false` for anything else. That is why your new `docs.yml` line will fail
`boundary-check` until the arm has an exact entry for it.

**Write it exact.** Do **not** relax the arm to accept arbitrary crates, and do not touch `inert_head` or
any other arm. Two precedents:

- **DC-70's B1** was an unsound widening of this same scanner — `inert_head` extended to treat `tar`,
  `rustc`, and `gh` as safe with any arguments. Found at review and repaired.
- The **architect edited this file unreviewed** once
  (`.git-exclude/reviewed/prikk-architect-boundary-breach-record-v1.md`). It is a governed file for good
  reason.

## 3. Criterion 2 is the one I will check hardest

**Prove the entry is narrow, not broad.** Show that a *different* `cargo install …` in a scanned file is
still rejected — a negative control, in the shape this project now expects.

An entry that permits the command you want is not the same as an entry that permits **only** it, and the
difference is invisible in a passing `boundary-check`. I will run this control myself at review, exactly
as I did on DC-74's refusal tests, where four of five passed with the gate removed.

## 4. Criterion 4: assert against the built HTML

The two documents currently emit `language-mermaid` — I verified that in `docs/book/` before writing this.
Success is `class="mermaid"` in the built output for both
`reference/architecture.md` and `reference/data-model-lifecycle.md`. **Grep the built HTML; do not judge
by looking at the page.**

Also confirm the book builds and renders **offline** — the JS must be vendored, never CDN-fetched. That
matters more here than in most projects: offline verifiability is the product's central claim.

## 5. Hard limits

- `ALLOWED_THIRD_PARTY` **untouched.** `mdbook-mermaid` is a CI build tool, not a crate dependency — DC-51's
  gate does not apply and must not be edited.
- No product crate manifest changes. Nothing may reach a published crate or release binary.
- Do not edit the two documents' content, only their rendering.

## 6. Gates

`EXECUTION-ORDER.md` §6 rule 9, **verbatim** — including `--locked`, `--no-fetch`, and `cargo +1.85.0`.
Test counts before and after.
