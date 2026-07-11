# RFC (accepted) - DC-26 Documentation Home Correction

**Status.** Accepted for implementation after architect design review.
**Target release.** 0.16.1 — recommended as the first documentation/reference increment after 0.16.0,
before the TASK-06..16 reference series is built on the current pattern.
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
4. **Awkward fit with RFC lifecycle.** `rfcs/` carries `proposed/ → accepted/ → done/` lifecycle
   semantics, while current-state references are not lifecycle RFC records. DC-24 handled this honestly
   by listing FDD-00/FDD-04 under a distinct "Current FDD References" section, but the structure still
   signalled that reader-facing current-state references were living beside design-process material
   rather than in the published reference book.

The root confusion: two different kinds of content wore one label.

- **Design-process / gating material** — *why* decisions were made; design docs that gate unbuilt work.
  Audience: contributors/architects. Correct home: `rfcs/`.
- **Current-state reference** — *what* the system is today. Audience: users/evaluators. This is
  documentation. Correct home: `docs/src/`.

FDD-00 and FDD-04 as written are the second kind. DC-26 moves that content to its reader-facing home
without claiming DC-24 was dishonest; the correction is about audience proximity, self-contained book
mechanics, and eliminating avoidable duplication.

## Proposed Structure

1. **`docs/src/reference/` is the authoritative, book-rendered home** for current-state architecture and
   project-concept references. Each page is **self-contained**: full current-state content, inline
   caveats, and its claim-to-source anchor table, all rendered in the book.
2. **Links point from the book *into* `rfcs/`, not the reverse.** A reference page may link to specific
   `done/` RFCs for the *rationale/history* ("why this decision"), but the book never depends on an
   external file for *what the system is*. Internal book links stay relative and unbreakable. Outbound
   RFC links from the published book must use absolute repository URLs, such as
   `https://github.com/nabbisen/prikk/blob/main/rfcs/...`, so they resolve from GitHub Pages. They are
   supplementary, not load-bearing, but they must still avoid the DC-24 F-1 broken-link class.
3. **Caveats live once.** The Core Caveats block exists in one authoritative place per topic (the book
   reference page). Other surfaces link to it rather than copying it. The four-file duplication and the
   no-drift check are retired.
4. **`rfcs/fdds/` is reserved for genuine gating FDDs only** — design documents that gate *unbuilt*
   work (e.g. a future plugin ABI or sync protocol). It is **not** a home for current-state references.
   If no gating FDDs exist yet, `rfcs/fdds/` is removed until one is written.
5. **Provenance and anchor tables move into the reference page** (as an appendix or footer) linking to
   the relevant RFCs/code. They serve evaluator trust and stay with the content they annotate.
6. **Security-claim changes keep review discipline.** Moving the trust/threat model into `docs/src/`
   does not make security claims ordinary guide text. Changes that alter trust, threat, verification,
   signature, key-management, durability, platform-support, or production-readiness claims require
   architect review or an accepted RFC/DC. The DC-24 grounding discipline travels with the page.

## Migration (DC-24 output)

- Fold the bodies of `rfcs/fdds/FDD-00-DATA-MODEL.md` and `FDD-04-TRUST-THREAT-MODEL.md` into
  `docs/src/reference/data-model.md` and `docs/src/reference/trust-threat-model.md`, which already carry
  the caveats. The pages become the authority (full content + anchor tables), not stubs.
- Replace load-bearing absolute GitHub links to FDD-00/FDD-04 with self-contained book content. Keep
  any supplementary book-to-RFC rationale/history links as absolute repository URLs so they resolve
  from the deployed book.
- Replace `rfcs/fdds/FDD-00`/`FDD-04` with one-line pointers into the book for the 0.16.1 release, so
  links shipped in 0.16.0 continue to land on useful pages. Delete those pointer files in 0.17.0 unless
  a later review extends the deprecation window.
- Remove `rfcs/fdds/` when it becomes empty. Recreate it only when a genuine gating FDD exists.
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

Net effect: the five tasks that pointed at `rfcs/fdds/FDD-0x` (06, 07, 08, 10, 12) move their authority
into `docs/src/reference/`; the rest were already book-homed and only lose the "thin pointer to an
external FDD" framing. Rationale/provenance still link back to `done/` RFCs and code.

## Non-goals

- No change to any code, schema, trust, or CLI behavior — documentation location only.
- No change to the RFC lifecycle policy (RFC-000) itself.
- Does not forbid future gating FDDs in `rfcs/fdds/`; it only removes *current-state references* from
  there.

## Timing (recommended)

0.16.0 has shipped with the DC-24 structure. That release is honest as-is; the current structure is
suboptimal, not wrong. DC-26 should be taken as an early 0.16.1 increment — before TASK-06..16. The
migration is two reference subjects today; after the reference series lands it becomes a larger
multi-page move. Fixing the pattern now is the cheap moment.

## Resolved Design Review Decisions

- FDD-00/FDD-04 are replaced with one-line pointers into the book for one release, then removed in
  0.17.0 unless explicitly extended.
- Claim-to-source anchor tables remain visible at the foot of each reference page. Evaluator trust
  depends on seeing claims tied to code/RFCs.
- `rfcs/fdds/` is removed when empty and recreated only when a genuine gating FDD exists.
- Book-to-RFC rationale/history links use absolute repository URLs, not relative links that can break
  in the deployed book.
- The trust/threat reference remains security-sensitive: security-claim changes require architect
  review or accepted RFC/DC coverage.

## Review / acceptance

This is a structural documentation decision that amends done DC-24. It follows the normal design →
implementation → release-note flow. During implementation, apply the TASK-06..16 graduation-target
edits in `.git-exclude/tasks/002-update-management/` and the ROADMAP *0.16.1+ Documentation Reference
Backlog* "Home" column in the same pass.
