# RFC (proposed) - DC-44 Migration, Backup, and Restore Evidence

**Status.** Proposed; scheduled beyond M2 and not current implementation authority.
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
