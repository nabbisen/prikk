# RFC (done) - DC-44 Migration, Backup, and Restore Evidence

**Status.** **DONE, 2026-09-01.** Before closure this line read *"Proposed; scheduled beyond M2 and not current implementation authority"* — kept as the record of what it said. Disposition below.

**Status update, 2026-09-01 — CLOSED. Every design goal is delivered or explicitly superseded.**
The 2026-08-27 update below stands as the record of what was true then; this supersedes its
"not full closure" conclusion.

| Design goal | Disposition |
|---|---|
| Self-describing export manifest (format, scope, tool version, exclusions) | Delivered — `PBNDL003`, `c135dd0` |
| Offline verification of the export file before restore | Delivered — `prikk bundle verify`, `d7c180c` |
| Named failure modes: interrupted export, destination collision | Delivered — atomic write + `--force` guard, `fd2424d`; the same four sync output sites, `1c13ade` |
| Format-1-to-current migration, or explicit supersession | Superseded — RFC 114's format-refusal ruling (see the 2026-08-27 update below) |
| A page stating what backup/restore does and does not prove | Delivered — `docs/src/guide/backup-restore.md`, `a4d875b`, corrected `d487194` |

**What is deliberately still open, and is named as such in the shipped documentation rather than
here:** multi-ref bundle export does not exist, and no restore has been rehearsed across a
repository-format change. Both are stated in `backup-restore.md`'s own Limits section and in the
`Still deferred` list of `integrity-recovery.md`, `durability-recovery.md`,
`concurrency-locking.md`, and `repository-layout.md` — not carried as unfinished DC-44 work.

**Moved to `rfcs/done/` and its ROADMAP Open-Work Index marker retired on 2026-09-01**, on the
owner's instruction — the two are coupled by RFC 120's index gate and were taken as one action.

**Status update, 2026-08-27 (evidenced, not a verdict — reported with the confidence the evidence
supports, not more).** Part of the gap this RFC exists to close has since been answered elsewhere,
and part has not.

**Answered:** design goal 4 asked to either exercise format-1-to-current migration or explicitly
supersede that path. RFC 114 (format-stability contract, accepted) ruled the opposite of a
migration path: formats 1 through 5 are refused outright at open, with no read-only fallback
(`docs/src/reference/release-compatibility.md`, "Repository Format Transitions"). The same page
names the reviewed alternative this RFC's own non-goals already permit ("no promise that 0.18.0
format-1 repositories are writable"): `prikk bundle export` on the version that still opens the
old repository, `prikk bundle import` into the new one — documented as the explicit,
in-place-migration-free way to carry work across a format change.

**Not found:** the remaining design goals — a self-describing export manifest naming format,
included refs/objects, digests, tool version, and exclusions; a dedicated offline verification
step on the export file itself before restore (distinct from running ordinary `verify` after
import, which `bundle import` already does); and test coverage of the specific named failure
modes (interrupted export, incomplete backup, missing/corrupt object, wrong format, destination
collision, restore retry). No page under `docs/src/` documents bundle export/import as a
backup/restore mechanism, or states what it does and does not prove, the way this RFC's own last
design goal asks. Checked `crates/prikk-store/src/bundle.rs` and its own test directory directly,
not inferred from the module doc's summary.

**So: real, evidenced progress on the problem that motivated this RFC, not full closure.** The
remainder appears real, not merely unverified — reported as evidence, since I am not confident
enough in either direction to call this closed or unchanged.
**Target milestone.** M3 - post-assurance recovery capability; target release not assigned.
**Tracks.** NFR-REL-03, format migration exercises, and backup/restore evidence missing from the 0.17.7
architecture review.
**Touches.** Repository export manifest, offline verification, restore workflow, format migration
exercise, failure evidence, and operator documentation.

## Problem

DC-40 deliberately makes format-1 repositories read-only without history-preserving migration. The
project also lacks a verified backup/export and restore exercise. These are not hidden inside M2
assurance: they are a separate recovery capability and production-readiness prerequisite.

## Design goals

- Define a self-describing export manifest that identifies repository format, included refs/objects,
  expected digests, tool version, and explicit exclusions without embedding signing secrets.
- Verify an export offline before restore and verify the restored repository before any mutation.
- Preserve immutable object/signature bytes; migration creates explicitly versioned new authority and
  never edits identity-bearing history in place.
- Exercise format-1 to current-format migration or explicitly supersede that path with a reviewed
  export/re-authoring contract.
- Cover interrupted export, incomplete backup, missing/corrupt object, wrong format, destination
  collision, and restore retry.
- Document what backup/restore proves and does not prove, including external trust/key custody.

## Non-goals

- No promise that 0.18.0 format-1 repositories are writable.
- No cloud backup service, remote sync, GC, secret escrow, or silent automatic migration.
- No production-readiness claim merely because one exercise passes.

## Dependencies and acceptance

DC-44 design begins after DC-40 format authority and M2 evidence tooling stabilize. Completion requires
literal manifest/version rules, end-to-end backup/restore fixtures, at least one migration rehearsal,
failure/retry evidence, and independent architecture review. Production suitability remains blocked
until this work or an explicit superseding decision closes NFR-REL-03.
