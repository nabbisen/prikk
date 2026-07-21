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
| Mixed release-policy tooling ownership and custom schema evaluator | Tooling debt | DC-45 | M2 |
| Declared Rust 1.85 minimum does not pass the locked product workspace | Compatibility debt | DC-46 | M2 |
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

DC-36 and DC-37 designs were accepted on 2026-07-15. DC-37 implementation was accepted and committed,
and DC-36 immutable object publication implementation was subsequently accepted. DC-38 ref publication
recovery implementation was accepted and committed after repair re-review on 2026-07-15. DC-35's
repository-governed multi-signer and break-glass amendment was accepted after architect design re-review
v3 on 2026-07-15. Architect repair re-review v3 accepted its policy implementation on 2026-07-16 after
byte/object, canonical-governance, tag-shape, and attempt-growth repairs. No signer is admitted; bootstrap
remains a separate prerequisite. DC-45 design acceptance is required before bootstrap, but completing
the Rust tooling migration is not. Until DC-45 cutover, bootstrap uses the accepted Python gate under
the separately reviewed DC-35 governance transaction. Architect design repair re-review v1 accepted
DC-45 on 2026-07-16; this acceptance does not itself authorize bootstrap. DC-39 implementation waits
for its own design review while using accepted DC-34 authority.
DC-38 and DC-40 designs, including the DC-40 companion state-root/format FDD, were accepted on
2026-07-14. DC-40 implementation remains pending behind the remaining M1 sequencing and its own gate.

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
4. DC-45 Release Policy Tooling Consolidation.
5. DC-46 Workspace Rust 1.85 Compatibility.
6. DC-47 Stable Clippy Gate Alignment.
7. DC-48 Legacy Clippy Production Retirement (planned after DC-47 post-commit acceptance).

DC-45 is the first M2 tooling increment. Its design must be accepted before signer bootstrap, and its
Rust command cutover must be accepted before the 0.19.0 release candidate; migration completion is not
an M1 or bootstrap prerequisite. Its design was accepted after architect repair re-review v1 on
2026-07-16. Profile hardening and the observation adapter are committed, and architect implementation
repair re-review v1 accepted the exact-byte oracle semantics on 2026-07-17. Project-owner acceptance is
withheld pending a compact tracked representation that avoids the candidate's 237 per-case vector
files. Architect footprint QA conditionally approved three strict suite packs, and architect design
amendment re-review v1 accepted the pack, location, closure, and archive contract on 2026-07-17. Compact
implementation is complete without staging and awaits implementation re-review. Owner acceptance,
isolated commit, and source-archive evidence precede Rust implementation. Compact implementation
review v1 found one blocking dot-segment grammar defect. Architect repair re-review v1 accepted its
narrow repair on 2026-07-17; explicit project-owner acceptance of the exact 13-file inventory remains
the next gate before the isolated freeze commit. Architect design repair re-review v1 accepted the
explicit retirement schedule on 2026-07-17, satisfying the lifecycle-design condition for that separate
owner decision. Five Python oracle authoring/verification files remain through the first Rust-gated
0.19.0 release. The first later release-candidate increment is blocked until an architect accepts the
later-commit stability rerun; the following release-candidate increment is blocked until the exhaustive
five-file decommissioning review removes each file or records an individual owner-approved, event-bound
exception. Rust must replace the complete accepted manifest verifier and self-test matrix. The other
eight frozen evidence/contract files remain until a later equivalence-backed replacement/consolidation
review or an explicit final-retirement review closes migration and rollback needs. These blockers
remain durably tracked if DC-45 moves to `done/` before their completion. The project owner committed
the exact 13-file oracle with the reviewed design/status update as stage-1 freeze commit `47aec9c` on
2026-07-17. Deterministic archive, checkout/extracted verification, direct-dependency/identity, and
seven-product-package exclusion evidence was accepted after architect post-commit evidence review v1
on 2026-07-17. Stage-2 Rust implementation was accepted after architect repair re-review v11 and
committed as `6a65a35` on 2026-07-21. Its deterministic archive, isolated checkout/extraction,
Python/Rust engine, differential, boundary, reference, identity, and seven-product-package exclusion
evidence was accepted after architect post-commit evidence review v1 on 2026-07-21. Preparation of an
isolated authoritative-command cutover candidate and disposable rollback rehearsal is now authorized.
Preparation found that the accepted stale-reference gate hardcodes Python live authority and cannot
validate an inventory/documentation-only Rust switch without a Rust-source transition repair. Focused
architect QA v1 accepted a separate exact two-state transition repair before cutover implementation
resumes. Architect implementation review v1 accepted the Python-primary repair, and it was committed
as `2bfb7cc` on 2026-07-21. Post-commit preservation evidence was accepted after architect review v1
on 2026-07-21. The exact four-file inventory/live-reference cutover was committed as `6a8e365`;
deterministic archive, clean checkout/extraction, full gate, and committed-identity rollback evidence
was accepted after final architect ruling v1 on 2026-07-21. The Rust command is governance-
authoritative. Python and the frozen oracle remain required through the first Rust-gated 0.19.0 release
and an accepted later-commit stability rerun.
DC-41 waits for the corrected contracts so its evidence does not bless superseded behavior. DC-42 may
perform read-only measurements during M1, but semantic optimization or broad source moves wait until M1
stabilizes. DC-43 policy design may proceed without credentials; implementation waits for security
review and must consume the stable post-DC-45-cutover gate rather than extend the Python engine.
DC-46 design now selects restoration of the declared Rust 1.85 locked-workspace contract through three
bounded source rewrites, focused trust regressions, and pinned locked CI gates. Architect design
rereview v1 accepted it on 2026-07-21. Architect command-grammar amendment QA v1 then authorized five
exact ordinary-Cargo vectors and existing scanner tests after the prepared candidate exposed a DC-45
classifier conflict. Architect implementation review v1 accepted the complete candidate on 2026-07-21;
it was committed as `0d221af`, and architect post-commit evidence review v1 accepted its clean
checkout/archive evidence. DC-46 and the Rust 1.85 compatibility blocker are complete; DC-45 does not
silently absorb this resolved product-workspace mismatch.
DC-47 is the accepted pre-0.19.0 release-candidate correction for the remaining Clippy command
divergence: DC-35 public release guidance selects `--all-features`, while current stable CI and the
DC-45 governed classifier select the no-all-features vector. It proposes preserving the stronger
release gate and adding one exact non-authority classifier production. Architect design review v1
accepted the bounded design on 2026-07-21. Architect legacy-vector test-contract QA v1 resolved the
retained-vector contradiction and authorized bounded implementation. Architect implementation review
v1 accepted the candidate on 2026-07-21; the owner commit and post-commit review remain required. After
those pass, DC-48 must separately retire both unconsumed legacy Clippy productions before the 0.19.0
release candidate.

**Completion condition:** reproducible crash/fuzz/hash/platform evidence is available, performance and
source-structure gates are enforced or carry reviewed exceptions, and release artifacts have reviewed
security reporting, dependency policy, SBOM, digest, and provenance controls. The release-policy tool
is consolidated behind the reviewed Rust command with the public schema, product publication graph,
and differential oracle evidence preserved.

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
