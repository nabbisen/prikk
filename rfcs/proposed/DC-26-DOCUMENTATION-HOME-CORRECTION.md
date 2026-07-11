# RFC (draft) - DC-26 Documentation Home Correction

**Status.** Proposed — entered `rfcs/proposed/` after maintainer review; awaiting design review.
**Target release.** 0.16.1 — recommended as the *first* increment after 0.16.0, before the TASK-06..16
reference series is built on the current pattern.
**Tracks.** Corrects the documentation-home decision made in DC-24.
**Touches.** Location and authority of current-state reference docs; `docs/src/reference/`; the fate of
`rfcs/fdds/`; the graduation targets of TASK-06..16.

## Context

DC-24 created two current-state references — data model and trust/threat model — and placed the
**authoritative** copy under `rfcs/fdds/` (`FDD-00-DATA-MODEL.md`, `FDD-04-TRUST-THREAT-MODEL.md`), with
short pointer pages in the published mdBook (`docs/src/reference/`). The TASK-06..16 backlog was written
to follow the same pattern (~10 more reference subjects, each an `rfcs/fdds/FDD-xx` authority + a thin
book page).

Review and use have shown this home is inverted. This RFC corrects it once, before the pattern is
replicated.

## Problem

1. **Substance lives where readers don't look.** The published book is the reader-facing surface, but
   the authoritative content sits *outside* it in `rfcs/`; the book carries only a stub that links out.
   Architecture and project concept are exactly what evaluators need — they belong in the book.
2. **The split manufactured a failure class.** Because the authority is outside `docs/src/`, the book
   cannot link to it internally; DC-24's F-1 finding (published-site 404s) was a direct consequence, and
   the "fix" (absolute GitHub URLs) makes the book non-self-contained and prone to link rot on any
   repo/branch rename.
3. **It created the split-brain it meant to avoid.** The Core Caveats block is duplicated across four
   files, and a bespoke "no-drift" check exists solely because the content was split across two homes.
4. **Category error.** `rfcs/` carries `proposed/ → accepted/ → done/` lifecycle semantics. A
   current-state reference has no lifecycle state, which is why FDD-00/FDD-04 had to sit in a new `fdds/`
   folder *outside* that scheme. That awkwardness (design-review note N-1) was the structure signalling
   it did not belong there.

The root confusion: two different kinds of content wore one label.

- **Design-process / gating material** — *why* decisions were made; design docs that gate unbuilt work.
  Audience: contributors/architects. Correct home: `rfcs/`.
- **Current-state reference** — *what* the system is today. Audience: users/evaluators. This is
  documentation. Correct home: `docs/src/`.

FDD-00 and FDD-04 as written are the second kind, mislabeled as the first.

## Proposed Structure

1. **`docs/src/reference/` is the authoritative, book-rendered home** for current-state architecture and
   project-concept references. Each page is **self-contained**: full current-state content, inline
   caveats, and its claim-to-source anchor table, all rendered in the book.
2. **Links point from the book *into* `rfcs/`, not the reverse.** A reference page may link to specific
   `done/` RFCs for the *rationale/history* ("why this decision"), but the book never depends on an
   external file for *what the system is*. Internal book links stay relative and unbreakable; outbound
   RFC links are supplementary, not load-bearing.
3. **Caveats live once.** The Core Caveats block exists in one authoritative place per topic (the book
   reference page). Other surfaces link to it. The four-file duplication and the no-drift check are
   retired.
4. **`rfcs/fdds/` is reserved for genuine gating FDDs only** — design documents that gate *unbuilt*
   work (e.g. a future plugin ABI or sync protocol). It is **not** a home for current-state references.
   If no gating FDDs exist yet, `rfcs/fdds/` is removed until one is written.
5. **Provenance and anchor tables move into the reference page** (as an appendix or footer) linking to
   the relevant RFCs/code. They serve evaluator trust and stay with the content they annotate.

## Migration (DC-24 output)

- Fold the bodies of `rfcs/fdds/FDD-00-DATA-MODEL.md` and `FDD-04-TRUST-THREAT-MODEL.md` into
  `docs/src/reference/data-model.md` and `docs/src/reference/trust-threat-model.md`, which already carry
  the caveats. The pages become the authority (full content + anchor tables), not stubs.
- Remove the absolute-GitHub-URL workaround; cross-links to RFCs become supplementary "history" links.
- Delete `rfcs/fdds/FDD-00`/`FDD-04` (or leave a one-line pointer into the book during a deprecation
  window). Remove `rfcs/fdds/` if it then holds nothing.
- Update `rfcs/README.md`, `README.md`, and `rfcs/IMPLEMENTATION-STATUS.md` references accordingly.
- Retire the caveat no-drift gate; keep a single-source caveats block.

## Migration (TASK-06..16 graduation targets)

Every task currently says "graduate into `rfcs/fdds/FDD-xx` + a thin mdBook page." New target for all of
them:

| Task | Old graduation home | New graduation home |
|---|---|---|
| TASK-06 durability & crash-recovery | `rfcs/fdds/FDD-02` + mdBook | `docs/src/reference/durability-recovery.md` (authoritative, book-rendered) |
| TASK-07 verify & doctor | FDD-02 + mdBook | `docs/src/reference/integrity-recovery.md` |
| TASK-08 patch-algebra concepts | `rfcs/fdds/FDD-01` + mdBook | `docs/src/reference/patch-algebra.md` |
| TASK-09 key mgmt & signing setup | mdBook `guide/` | `docs/src/guide/security-setup.md` (unchanged — already book-home) |
| TASK-10 repository layout & authority | fold into FDD-00 | `docs/src/reference/data-model.md` (§layout) or `reference/repository-layout.md` |
| TASK-11 path & worktree safety | mdBook `reference/` | `docs/src/reference/path-safety.md` (unchanged intent) |
| TASK-12 concurrency & locking | FDD-02 + mdBook | `docs/src/reference/concurrency-locking.md` |
| TASK-13 release/versioning policy | mdBook `reference/` | `docs/src/reference/release-compatibility.md` (unchanged) |
| TASK-14 non-goals / deferred | mdBook `reference/` | `docs/src/reference/non-goals.md` (unchanged) |
| TASK-15 roles & user classes | mdBook orientation | `docs/src/` orientation page (unchanged) |
| TASK-16 error taxonomy | mdBook `reference/` | `docs/src/reference/errors.md` (unchanged) |

Net effect: the four tasks that pointed at `rfcs/fdds/FDD-0x` (06, 07, 08, 10, 12) move their authority
into `docs/src/reference/`; the rest were already book-homed and only lose the "thin pointer to an
external FDD" framing. Rationale/provenance still link back to `done/` RFCs and code.

## Non-goals

- No change to any code, schema, trust, or CLI behavior — documentation location only.
- No change to the RFC lifecycle policy (RFC-000) itself.
- Does not forbid future gating FDDs in `rfcs/fdds/`; it only removes *current-state references* from
  there.

## Timing (recommended)

**Do not reopen 0.16.0.** It is accepted and honest as-is; the current structure is suboptimal, not
wrong, and reopening a just-accepted release invites scope creep. Ship 0.16.0, then take **DC-26 as an
early 0.16.1 increment — before TASK-06..16**. The migration is two files today; after the reference
series lands it is a twelve-file move. Fixing the pattern now is the cheap moment.

(If the maintainer would rather not ship the interim `rfcs/fdds/` structure at all, the alternative is to
fold this into 0.16.0 before tag — a small two-file change plus a re-review. Recommended only if the tag
is not time-sensitive; otherwise prefer the 0.16.1 path.)

## Open questions

1. Delete `rfcs/fdds/FDD-00/04` outright, or leave one-line pointers into the book for a deprecation
   window (in case external links already reference them)?
2. Keep claim-to-source anchor tables visible in the reader-facing page, or move them to a collapsed
   appendix so the main page stays short? (Recommend: visible but at the page foot — evaluator trust
   depends on them.)
3. Does `rfcs/fdds/` stay as a reserved (empty) home for future gating FDDs, or get removed until needed?
   (Recommend: remove until a real gating FDD exists.)

## Review / acceptance

This is a structural documentation decision that amends done DC-24. It is now in `rfcs/proposed/` and
awaits design review, then follows the normal design → implementation → release-note flow. On
acceptance (move to `rfcs/accepted/`), apply the TASK-06..16 graduation-target edits in
`.git-exclude/tasks/002-update-management/` and the ROADMAP *0.16.1+ Documentation Reference Backlog*
"Home" column in the same pass.
