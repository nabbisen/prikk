# DC-44 Migration, Backup, and Restore Evidence - Design Brief

**This is a design brief, not an implementation handoff.** DC-44's detailed design does not exist yet, so
there is nothing honest to instruct an implementer to build. This document specifies what the **design
stage** must produce, so that when DC-44 is scheduled the design work starts from a defined target rather
than a blank page. An implementation handoff follows once the design is accepted.

**Authored by** the architect (function-designer role).
**Stage gate:** DC-44 is proposed and scheduled for **M3**, after M2. Design work may not begin until the
owner schedules it; `MILESTONES.md` places it after M2 and it depends on DC-40 format authority (done) and
M2 evidence tooling stabilising (in progress).
**Assigned to:** architect for design; developers for implementation after design acceptance.

## Why this needs a design stage at all

DC-44 owns NFR-REL-03 and is a stated production-readiness prerequisite. It is also the increment that
decides what happens to **existing format-1 repositories**, which DC-40 deliberately made read-only
without history-preserving migration. That is a user-facing commitment, not an internal detail, and
getting it wrong is expensive to reverse.

## Decisions the design must make and record

These are genuinely open. Do not treat any as pre-decided.

1. **Export manifest contract.** Self-describing: repository format version, included refs and objects,
   expected digests, tool version, explicit exclusions. Must embed **no signing secrets**. Needs literal
   byte-level rules and version authority, following the DC-40 companion-FDD precedent — a manifest whose
   format is described only in prose will drift.
2. **Verification points.** The RFC requires verifying an export *offline before restore*, and verifying
   the restored repository *before any mutation*. Decide what each verification actually checks, and what
   a partial pass means.
3. **Migration versus supersession.** Either exercise format-1 → format-2 migration, **or** explicitly
   supersede that path with a reviewed export/re-authoring contract. This is the central decision. The
   current documented answer is re-authoring in a fresh repository; if that stands, say so as a decision
   rather than by default, and state the consequence for existing users.
4. **Immutability rule.** Migration creates explicitly versioned new authority and never edits
   identity-bearing history in place. Confirm how that is enforced, not just intended.
5. **Failure matrix.** Interrupted export, incomplete backup, missing or corrupt object, wrong format,
   destination collision, restore retry. Each needs a defined outcome before implementation.
6. **Claim boundary.** What backup/restore proves and — importantly — does not prove, including external
   trust and key custody, which are outside the repository.

## Constraints inherited from accepted work

- **Format-1 stays read-only.** DC-40's boundary is accepted; DC-44 does not reopen it.
- **Persisted bytes are immutable.** DC-39 and DC-40 froze signature preimage and state-root grammar. Any
  export/restore path that would alter a persisted byte is out of scope by construction.
- **Linux-only mutation.** DC-37 means restore is a mutation operation and therefore Linux-only today.
  The design must state this rather than implying cross-platform restore.
- **No production claim** follows from one passing exercise.

## What the design stage must deliver before an implementation handoff exists

- A companion design document with the literal manifest and version rules (DC-40 FDD precedent).
- The migration-versus-supersession decision, recorded with its user consequence.
- The failure matrix with a defined outcome per row.
- Fixture design for end-to-end backup/restore and at least one migration rehearsal.
- The claim boundary, written for operators rather than for reviewers.

## Non-goals (from the RFC, restated so the design does not drift)

No promise that format-1 repositories become writable. No cloud backup service, remote sync, GC, secret
escrow, or silent automatic migration. No production-readiness claim from a single passing exercise.

## Standing boundaries

No release-lane action. No change to object identity, canonical encoding, signature preimage, or the
state-root grammar. Production suitability remains blocked until this work — or an explicit superseding
reviewed decision — closes NFR-REL-03.
