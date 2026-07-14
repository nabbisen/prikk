# Prikk Corrective Milestones

This file schedules the corrective program opened after the independent architecture review of the
released 0.17.7 tree. `ROADMAP.md` remains the concise project backlog, individual RFCs own design, and
`rfcs/IMPLEMENTATION-STATUS.md` remains the current implementation snapshot.

## Baseline and release posture

The reviewed 0.17.7 tree remains suitable for architecture experimentation and corrective development.
It is not approved for production use, repository-format stabilization, or a public-preview readiness
claim. Successful existing unit and integration gates do not close reproduced crash-state defects.

Feature increments and unrelated documentation releases are frozen through M1. Design records,
corrective implementation commits, and review packages may proceed during the freeze. The next release
is prepared only after all M1 completion conditions are met; accepted RFCs are not individually treated
as release readiness.

Milestones below are dependency-ordered, not calendar promises. Target versions identify the intended
release boundary and may change only through an update to this file, `ROADMAP.md`, and affected RFCs.

## Finding ownership

| Review subject | Severity | Owning RFC | Milestone |
|---|---|---|---|
| Ref publication split-brain crash state | Blocking / critical | DC-34 authority, DC-38 implementation | M0, M1 |
| Block state root is a Patch-id scaffold | Blocking / high | DC-40 | M1 |
| Required directory durability errors suppressed | Blocking / high | DC-37 | M1 |
| Existing object path accepted without byte validation | Blocking / high | DC-36 | M1 |
| Signature-preimage authority unresolved | Blocking / high | DC-34 authority, DC-39 implementation | M0, M1 |
| Signature envelope canonicalization incomplete | Non-blocking | DC-39 | M1 |
| RefUpdate timestamp always zero | Non-blocking | DC-34 ruling, DC-39 implementation/docs | M0, M1 |
| Merge status docs contradict released CLI | Non-blocking | DC-35 | M1 |
| Crash/fuzz/platform and hash evidence incomplete | Assurance blocker | DC-41 | M2 |
| Full-tree commit scan versus NFR-PERF-01 | Requirement gap | DC-42: implement or obtain explicit requirements amendment | M2 |
| Active-Patch warning 800 / hard bound 1000 from NFR-PERF-02 | Requirement gap | DC-42: implement or obtain explicit requirements amendment | M2 |
| Source/test structure gates absent | Maintainability risk | DC-42 | M2 |
| Vulnerability reporting, SBOM, provenance absent | Distribution risk | DC-43 | M2 |
| Backup/restore verification and migration exercises absent | Recovery capability gap | DC-44 | M3 |

## M0 - Architecture ratification

**Release target:** none.

**RFC:** DC-34 Publication and Identity Authority.

**Status:** Complete; architect re-review accepted DC-34 on 2026-07-14 and it is tracked in
`rfcs/accepted/`.

M0 selects the ref publication commit point, valid interrupted states, retry/doctor authority, the
literal version-1 signature preimage, and the RefUpdate no-clock sentinel. DC-38 through DC-40 may not
begin identity-bearing implementation before DC-34 is accepted by architect review.

**Completion condition:** Satisfied. DC-34 was reviewed, repaired, re-reviewed, and moved to
`rfcs/accepted/` with roadmap/index/status links updated.

## M1 - Corrective storage and identity baseline

**Release target:** 0.18.0.

**RFCs:**

1. DC-35 Release Compatibility and Status Correction.
2. DC-36 Existing-Object Publication Integrity.
3. DC-37 Required Filesystem Durability.
4. DC-38 Ref Publication Crash Recovery.
5. DC-39 Signature and Envelope Authority.
6. DC-40 State Merkle Root and Format Transition.

DC-35 through DC-37 may receive design review after the M0 draft is stable. DC-38 implementation waits
for accepted DC-34 and implemented DC-37 semantics. DC-39 implementation waits for accepted DC-34.
DC-38 and DC-40 designs, including the DC-40 companion state-root/format FDD, were accepted on
2026-07-14. DC-38 implementation still waits for implemented DC-37 semantics. DC-36 is otherwise
independent and is the preferred first implementation repair.

**Release condition:** all five blocking findings are closed by accepted implementation review; the
reproduced ref failure no longer succeeds; the state-root and signature vectors are pinned; format-1/
format-2 behavior is explicit; release/status documentation is current; the full relevant gate set and
corrective failpoint matrix have observed passing evidence; and an adversarial 0.18.0 release-candidate
review accepts the combined state. No production or public-preview claim follows automatically.

## M2 - Assurance and distribution baseline

**Release target:** 0.19.0, subject to M1 release and design review.

**RFCs:**

1. DC-41 Integrity Evidence Campaign.
2. DC-42 Performance and Maintainability Gates.
3. DC-43 Release Security and Distribution Controls.

DC-41 waits for the corrected contracts so its evidence does not bless superseded behavior. DC-42 may
perform read-only measurements during M1, but semantic optimization or broad source moves wait until M1
stabilizes. DC-43 policy design may proceed without credentials; workflow implementation waits for
security review.

**Completion condition:** reproducible crash/fuzz/hash/platform evidence is available, performance and
source-structure gates are enforced or carry reviewed exceptions, and release artifacts have reviewed
security reporting, dependency policy, SBOM, digest, and provenance controls.

After M2, request a new independent architecture review. Public-preview readiness, repository-format
stability, and production suitability remain separate decisions and are not milestone completion
side effects.

## M3 - Migration and recoverable backup

**Release target:** not assigned; scheduled after M2.

**RFC:** DC-44 Migration, Backup, and Restore Evidence.

M3 owns NFR-REL-03 and the migration/restore exercises intentionally excluded from the 0.18.0 format
transition. It defines verifiable export/restore and either exercises format migration or records an
explicit superseding recovery contract. This work is not implied by DC-41's broad evidence campaign.

**Completion condition:** reviewed manifest/version authority, offline backup verification, restore and
retry fixtures, at least one migration rehearsal, and independent architecture acceptance. Production
suitability remains no-go before M3 or a superseding reviewed decision; public-preview consideration
after M2 remains a separate narrower ruling.

## Deferred during the corrective program

- Merge execution, branch lifecycle expansion, remotes/sync, rollback publication, plugins/audit, and
  key lifecycle features remain frozen.
- TASK-14 through TASK-16 documentation themes remain queued. TASK-13 is the narrow exception because
  compatibility and release rules are required for the corrective format transition.
- Any newly discovered correctness or identity defect interrupts this sequence and receives its own
  RFC or an explicit amendment to the owning proposed RFC before implementation.
