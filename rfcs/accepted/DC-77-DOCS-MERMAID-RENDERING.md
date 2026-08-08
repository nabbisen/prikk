# RFC (accepted) - DC-77 Docs Mermaid Rendering

**Status.** **ACCEPTED by the project owner 2026-08-08.** Small increment.
**Independence.** Author-reviewed, the standing ceiling; compensated at implementation review.
**Arises from.** `reference/architecture.md` and `reference/data-model-lifecycle.md` (added `5403246`)
carry Mermaid diagrams that currently render as **code blocks** — verified in the built HTML, which emits
`language-mermaid` rather than `class="mermaid"`. The docs are correct but the diagrams are not pictures.
**Target.** 0.20.0. Docs only; no product crate is touched.

## 1. Scope

Make Mermaid render in the mdBook output:

- `book.toml` — a `[preprocessor.mermaid]` stanza.
- `.github/workflows/docs.yml` — install `mdbook-mermaid`, **version-pinned and `--locked`**, matching
  the existing `mdbook` entry's discipline.
- Vendored `mermaid.min.js` and `mermaid-init.js`, as `mdbook-mermaid install` produces them.
- `tools/release-policy/src/command_scan/procedure.rs` — **one exact allowlist entry** for the new
  install command.

## 2. The only security-relevant part, and the reason this is an increment rather than a config edit

`boundary/publication.rs:41` scans every file under `.github`, `scripts`, and `release` through the
command scanner. `command_scan/procedure.rs:148`'s `install` arm accepts **exactly one** argument vector
today. A second `cargo install` in `docs.yml` is rejected until that arm gains an exact entry.

**Widening this allowlist is a change to a security control.** Two pieces of history make that concrete:

- **DC-70's B1** was an unsound widening of this same scanner — `inert_head` extended to cover `tar`,
  `rustc`, and `gh`, treating them as safe with any arguments. It was repaired.
- The **architect edited this exact file unreviewed** once, recorded at
  `.git-exclude/reviewed/prikk-architect-boundary-breach-record-v1.md`.

So the entry must be **exact**, not a relaxation of the `install` arm to accept arbitrary crates.

## 3. Acceptance criteria

1. `boundary-check` passes with the new `docs.yml` command.
2. **The entry is proved narrow, not broad — a negative control.** Show that a *different*
   `cargo install …` command in a scanned file is still rejected. **An entry that happens to permit the
   command we want is not the same as an entry that permits only it**, and this criterion is the whole
   point of the increment.
3. The install command is **version-pinned and `--locked`**, so the docs build is reproducible.
4. `mdbook build docs` emits `class="mermaid"` — not `language-mermaid` — for the diagrams in
   `reference/architecture.md` and `reference/data-model-lifecycle.md`. Assert against the built HTML,
   not by inspection.
5. Vendored assets are committed, and the book builds **offline** — no CDN fetch at build or view time.
6. `ALLOWED_THIRD_PARTY` **untouched**; no product crate manifest changes; no published crate or release
   binary is affected.
7. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, verbatim.

## 4. Non-goals

- Any change to the other `procedure.rs` arms, or to `inert_head`. **One entry, one justification.**
- Any product dependency change.
- Rewriting the two documents' content — only their rendering.
