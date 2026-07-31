# Prikk Corrective Milestones

This file schedules the corrective program opened after the independent architecture review of the
released 0.17.7 tree. `ROADMAP.md` remains the concise project backlog, individual RFCs own design, and
`rfcs/IMPLEMENTATION-STATUS.md` remains the current implementation snapshot.

## Baseline and release posture

The reviewed 0.17.7 tree remains suitable for architecture experimentation and corrective development.
It is not approved for production use, repository-format stabilization, or a public-preview readiness
claim. Successful existing unit and integration gates do not close reproduced crash-state defects.

The corrective implementation freeze ended when DC-35 through DC-40 implementation and evidence were
accepted. Development priority and release readiness now use separate lanes: product/assurance work is
selected by project value, while release work remains dormant until the project owner explicitly
activates preparation for a named version. A completed implementation, target version, or listed next
gate does not itself activate signer bootstrap, a hold, RC preparation, tagging, or publication.

When release preparation is activated, all gates for that target remain binding and release assets must
be current before publication. Accepted RFCs are not individually treated as release readiness.

### Durable release-lane transition

Release activation is an atomic reviewed planning/status commit, not a conversational instruction. That
commit must update all three durable authorities with the same values:

- `ROADMAP.md`, under **Release Candidate Increment**: lane state `active` and exact target version;
- this file, under **Baseline and release posture**: lane state `active` and exact target version;
- `rfcs/IMPLEMENTATION-STATUS.md`: `Current release activation: active` and
  `Current activated release target: <exact version>`.

The transition must land before any fingerprint is requested, bootstrap candidate is prepared, or other
release-lane work begins. Discussion, implementation completion, target-version prose, review
recommendations, and untracked messages are non-authoritative. While no bootstrap transaction or hold
has begun, parking or retargeting uses the same reviewed three-authority transition. Once bootstrap
begins, DC-35's governance, containment, hold, and lift rules govern closure; a planning edit cannot
erase that state.

Release conditions attach to unshipped accepted increments, not only to a version label. The first
release that contains an accepted but unshipped increment inherits every release condition and
lifecycle/status correction attached to that increment. Therefore, activating 0.19.0 while 0.18.0
remains unpublished would carry forward all M1 gates in addition to applicable M2 gates. Retargeting
also updates this file, `ROADMAP.md`, and every affected RFC target/status statement in the same reviewed
change.

If the three authorities disagree, the release lane is parked. No release-lane work may begin until a
reviewed commit restores agreement.

**Current release lane:** `parked`.

**Current activated release target:** none.

Milestones below are dependency-ordered, not calendar promises. Target versions identify the intended
release boundary and may change only through an update to this file, `ROADMAP.md`, and affected RFCs.

## Two milestone schemes — resolve gate labels here first

**This file's `M0`–`M3` are the corrective scheme. Requirement gate labels in
`specs/prikk-non-functional-requirements-v1.1.md` are NOT.** They belong to the original product scheme in
`specs/prikk-roadmap-milestones-v1.1.md`, which reuses the labels `M0`–`M3` with different meanings and
continues to `M7`.

Recorded 2026-07-29 by DC-42 design review v2 (finding B4). The collision had already caused a careful
architect review to misread a requirement gate and conclude that overdue work was not yet due. Every one
of the 38 NFR IDs carries such a label, so the error is available in both directions — it can make overdue
work look scheduled, or scheduled work look overdue.

| Label | Product scheme (what NFR gates mean) | Delivered? | Corrective scheme (this file) |
|---|---|---|---|
| M0 | Design Lock and Safe Scaffolding | yes | Architecture ratification |
| M1 | Core Storage and Identity | capability yes — **but NFR-PERF-01 unmet** | Corrective storage and identity baseline |
| M2 | Minimal Patch Engine | yes | Assurance and distribution baseline |
| M3 | Block DAG and Checkout | **partially** — checkout and block DAG shipped, but **multi-patch active blocks are not implemented**, leaving NFR-PERF-02 unreachable and NFR-PERF-03 vacuous | Migration and recoverable backup |
| M4 | WASM Plugin and Audit | no | — |
| M5 | Sync and Quarantine | no | — |
| M6 | Alpha Hardening | no | — |
| M7 | Public Preview Readiness | no | — |

The corrective scheme is a remediation track laid over the product scheme after the independent
architecture review; it does not replace it. A requirement gated at product M1 is **overdue today**
regardless of where the corrective track has reached.

When citing a gate, name the scheme: "product M3" or "corrective M2", never a bare "M3".

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
| Public portability claim exceeds Linux-only mutation support | Release-claim mismatch | DC-37 boundary plus tracked portability/requirements correction | M1 |
| Crash/fuzz/platform and hash evidence incomplete | Assurance blocker | DC-41 | M2 |
| Full-tree commit scan versus NFR-PERF-01 | **Missed product gate**, carried; measured by DC-59 (4.22 ms at 10 files to 516 ms at 10,000, one file changed) | DC-59 evidence **complete** (`a9c2fe0`). DC-56 implemented the changed-path index (`8748f00`) and it works. DC-64 implemented an incremental baseline cache and eliminated the O(operations replayed) full-lineage-replay cost (the dominant violator, ~370 ms of ~520 ms at 10,000 files) — **but did not fully flatten Axis A**: `load`/`persist`/`from_replay`, each a binding condition of the architect's trust-ladder ruling, remain O(live node count). Warm-cycle cost still grows ~7.9x for 10x repository size (DC-59 Axis C), down from ~9x cold. **NFR-PERF-01 remains missed**, now on a lower, non-flat curve. See the DC-64 design document §9 | Product **M1**; carried into corrective M2 |
| **Baseline reconstruction is O(repository size) on the commit hot path** — `replay_derived_state` + `live_nodes` projection + `working_state` clone cost 375 ms of a 519 ms commit at 10,000 files, and scale 9.15x for 10x repository size | **The actual NFR-PERF-01 violator**, found 2026-07-31 by the DC-56 implementation's scope finding and independently re-measured by the architect | **DC-64 implemented** (`rfcs/handoffs/DC-64-baseline-reconstruction-cost/incremental-baseline-cache-design-v1.md`): the O(operations replayed) full-lineage-replay term (97.6% of the phase, ~40 µs/op) is eliminated on the warm path (~2.6 ms at 10,000 files). What remains is O(live node count), not O(operations) — a structurally different, smaller, but still repository-size-proportional cost, required by the ruling's own binding conditions (persisted-state validation and complete `seen_ids` retention) | Product **M1** |
| **Editing the same text file in two separate sealed commits fails** — `plan_edit_text` reads `base.blob_id` as a stored `Blob`, but `EditText` never writes one (`write_content_blob` has exactly two call sites, create and `ReplaceBinary`), so any second edit errors `baseline content Blob … is missing` | **Severe / core workflow.** Found 2026-07-31 by the DC-64 implementation, **independently reproduced by the architect on `6064da6`** — long-standing, not a regression. The suite never edits one text file across two sealed commits, so 561+80 tests, a crash matrix, a fuzz campaign and DC-41 all passed over it | **DC-65** (`rfcs/accepted/DC-65-TEXT-EDIT-BASELINE-CONTENT.md`), **accepted 2026-07-31 and placed at the top of the development lane**, ahead of all performance work | Product **M1** |
| DC-64's residual commit cost is O(live node count) — cache `load` ~58 ms, `persist` ~29 ms, `from_replay` ~5.4 ms at 10,000 files; warm cycles grow ~7.9x for 10x repository size | **Not implementation slack.** Each is a direct consequence of the trust-ladder ruling's binding conditions 1 and 3. DC-64 eliminated the O(operations replayed) term it was authorized to remove (370.6 ms to ~2.6 ms) | Unowned. Reducing it means changing the persisted representation and therefore the trust argument — a design question, not a tuning pass | Product **M1**, with NFR-PERF-01 |
| Multi-patch active blocks not implemented — active WAL capped at one record | **Capability gap; blocks two NFRs** | Needs a queuing increment, or reviewed amendment of both NFRs. **Owner decision pending** | Product **M3** |
| Active-Patch warning 800 / hard bound 1000 from NFR-PERF-02 | **Missed product gate** — blocked on the capability above, not merely unimplemented | DC-57 **HELD 2026-07-29**: thresholds are unreachable while an active block holds one patch | Product **M3**; carried into corrective M2 |
| Merge scope bounded by active block size (NFR-PERF-03) | **Vacuously satisfied** — same root cause | Unowned; resolves with the capability above | Product **M3** |
| Source/test structure gates absent | Maintainability risk | DC-58 (was DC-42) — **complete**: batches 1 (`e1d0213`) and 2 (`54a3037`) accepted, N1 report reframing `6f53da3` accepted 2026-07-31 | Corrective M2 |
| `branch close` fails **open** on a corrupt repository — its WAL guard uses `.is_ok()`, so a non-empty active WAL with missing or malformed ownership metadata permits closure where every sibling publisher (`commit`) propagates the same `Integrity` error and stops | **Low, but a convention break.** Needs an already-corrupt repository; deletes nothing. Recorded 2026-07-31 from the DC-61 implementation review (N1) | **Repaired `2394f1b`** — the guard now matches the outcome (`Ok` refuses, `LockConflict` proceeds, any other error propagates) instead of testing `.is_ok()`, with a test constructing the missing-metadata case. DC-61 complete | Corrective M2 |
| Branch closure is CLI-reachable but reopening is not — `prikk branch close` exists, no `reopen` verb; recovery requires calling `RefStore::publish` from Rust | **Usability asymmetry, deliberate.** Criterion 6 required reopening to *succeed*, not to have a verb, and it is tested. Recorded 2026-07-31 from the DC-61 review (N2) | Unowned; candidate for whichever increment next touches the branch surface | Unscheduled |
| Vulnerability reporting, SBOM, provenance absent | Distribution risk | DC-43 | M2 |
| Mixed release-policy tooling ownership and custom schema evaluator | Tooling debt | DC-45 | M2 |
| Declared Rust 1.85 minimum does not pass the locked product workspace | Compatibility debt | DC-46 | M2 |
| Backup/restore verification and migration exercises absent | Recovery capability gap | DC-44 | M3 |
| Cross-platform evidence absent for portable logic | Assurance gap | DC-49 (descoped from DC-41) | M2, blocked on the M1 portability correction |
| First-party SHA-256 maintenance ROI unanswered | Deferred decision | DC-50 — answered at `4005efb`: **replace**; DC-55 implements | M2 |
| No mechanical gate on product `[dependencies]` placement | Supply-chain risk | DC-51 (DC-41 finding B4) | M2 |
| DC-45 retirement obligations tracked only in prose | Process debt | DC-52 | M2 |
| Repository-wide AUTHOR trust unverified | Capability gap | DC-53 | Post-M2, unscheduled |
| Case-insensitive ref-name collisions are not rejected — `validate_local_branch_ref` has no such rule, so `heads/Main` and `heads/main` coexist as distinct refs | **NFR-SEC-03 unmet** ("case-insensitive collisions are rejected by repo policy"). Surfaced 2026-07-30 by DC-63's fix design review, which had wrongly attributed the rule to the branch validator | Unowned. Must cover **branches and tags together** — fixing it inside DC-63's tag validator would leave the requirement half-met and conceal the branch half | Product **M1/M3** |
| `publish` rejects every `tags/` ref name — `validate_publication` calls `validate_local_branch_ref` unconditionally, which reserves the namespace | **Blocks §6.6 entirely.** No tag can be published through the ordinary machinery | DC-63 §2 — kind-aware validation inside `validate_publication` | Product **M1** |
| `verify` requires every ref target to be a `Block` — `ensure_block_exists` at `scan.rs:65` and `:221` | **Blocks §6.6 entirely.** §6.6 requires a tag ref to target a tag object, which `read_typed` rejects as a type mismatch | DC-63 §3 — resolve one extra hop for `RefKind::Tag`, both call sites | Product **M1** |
| DC-62's memory table published absolute `VmHWM` with no baseline row, so the content-proportional component was present but not visible — 1,000→10,000 files read as 2.58x growth where above the process floor it is 9.9x | **Presentation, not data.** Mattered because DC-56's criterion 5 evidences its memory improvement against this table, and without a baseline it could not distinguish eliminating the full-tree read from merely reducing it | **Resolved 2026-07-30**: DC-62's N1 repair at `07b1fc8` measures a floor (a real `commit` against a 1-file repository, 6,144 KB) and publishes an "Above floor" column — 9.92x growth where absolute VmHWM shows 2.58x. Repaired under DC-62, not DC-56 as the architect had wrongly specified | Corrective M2 |
| Two `ObjectType::Block`-hardcoded ref-target reads remain in `refs/evidence.rs` (`:31`, `:64`) after DC-63 made verification kind-aware | **Latent, not live.** `:31` reads the ref named in `ActiveRefMetadata`, written only by active-WAL append paths and only for branches; `:64` is reachable only via `finish_interrupted_publication`, whose sole production callers are seal. Tags cannot enter either path | Revisit if tags gain an active-session path, an interrupted-publication recovery path, or if `finish_interrupted_publication` gains a non-seal caller. Recorded 2026-07-30 from the DC-63 v2 review (N1) | Corrective M2 |
| §6.6 tagging has no command surface — `TagPayload`, `ObjectType::Tag` (code `0x05`), and `RefKind::Tag` all exist and are identity-pinned, but nothing creates a tag | Capability gap; recorded at `rfcs/IMPLEMENTATION-STATUS.md:484` as deferred | DC-63 | Product **M1** object model complete; surface outstanding |
| `TagPayload.created_at` documented as an "authoritative creation timestamp" while DC-34 ruled there is no clock authority and zeroed the analogous `RefUpdate` field | Documentation/design contradiction, surfaced 2026-07-30 by DC-63 | **Resolved 2026-07-30**: DC-63 writes the no-clock sentinel and corrects the doc comment, comment-only so the pinned `Tag` ObjectId does not move | Product **M1** |
| Commit loads the entire worktree into memory — `WorktreeFile { bytes: Vec<u8> }` per file, O(total worktree bytes) regardless of change size | **Fixed by DC-56 (`8748f00`), but it was not the memory problem.** `WorktreeFileMeta` now holds no bytes. Measured 2026-07-31: worktree content is ~2.5 MB of a ~13 MB above-floor footprint at 10,000 files; the resident cost is replayed node state. **Peak `VmHWM` rose ~1.1 MB** (19,464 → 20,652 KB) because DC-56's index is itself resident — accepted as a known cost | Structure removed by DC-56. **DC-64 implemented an incremental cache but could not reduce this term**: the persisted cache holding the full live node set is exactly what condition 1 (complete `seen_ids`, never truncated) requires, so `load`/`persist` do not shrink the resident live-node-set cost below holding `NodeLifecycleState` itself. Criterion 7 was amended by the ruling to reflect this is not eliminable within DC-64's scope | Product **M1**, alongside NFR-PERF-01 |
| `branch list` is more permissive than `verify` on ref-pointer directory entries — `list_ref_pointers` skips non-regular and non-`.ref` entries and does not check the filename is `sha256(ref_name)`, where `verify`'s `read_pointers` hard-errors on all three | **Low** — needs filesystem write access, `verify` still catches it, no identity or commit outcome depends on listing. But `list` can display a branch `verify` would reject | Recorded 2026-07-30 from the DC-60 slice review (N1). Natural close is DC-61, which already touches `list_ref_pointers` | Corrective M2 |
| Lifecycle-cache trust ladder built but unwired — 848 test-only lines await blob-kind verification, provenance-vs-baseline staleness, and replay reconstruction/compare | **Capability gap, previously untracked** — recorded 2026-07-30 from the DC-58 batch 2 review. Its governing RFC (DC-09 Phase 4.4-2b.1) is in `rfcs/archive/`, so the capability has no live owner | **Ruled 2026-07-31, `.git-exclude/reviewed/prikk-dc64-trust-ladder-ruling-v1.md`: DC-64 did not need it.** The ladder's rung-4 certification (`ComparedLifecycleCache`) guards `node_id` reuse/restoration-equivalence, both consumed only by `patch_algebra` from the merge path — the commit path makes neither decision, so DC-64 built an independent, narrower, commit-path-scoped cache instead. Still unowned; still needs an RFC or an explicit decision to drop the scaffolding, but no longer a candidate to resolve alongside DC-64 | Unscheduled |
| **A text file edited twice across two separate sealed commits fails commit** with `integrity error: baseline content Blob ... is missing` — `node_authoring.rs`'s `plan_edit_text` calls `read_file_blob_bytes(object_store, base.blob_id)`, but `EditText` never writes its derived content as a stored `Blob` object (only `CreateFile`/`ReplaceBinary` call `write_content_blob`); the first edit's `base.blob_id` is the genesis blob (real, succeeds), but any *second* edit's `base.blob_id` is a previous `EditText`'s computed-but-never-persisted identity | **Severe, previously undiscovered, blocks a basic workflow.** No existing test edited the same text file in two separate commits before DC-64's Axis C benchmark development surfaced it manually; reproduces identically on pre-DC-64 `6064da6`, so it is unrelated to DC-64 and predates it. `checkout`/materialization evidently reconstructs current text via replay rather than a direct blob read, so this is specific to `plan_edit_text`'s shortcut | Unowned; needs its own RFC or fix-and-review cycle. Reported 2026-07-31, evidence and reproduction in the DC-64 submission package | Product **M1/M2**, correctness |

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
remains a separate prerequisite. DC-45 design acceptance and the later Rust authority cutover are
complete. Bootstrap therefore uses the accepted Rust release-policy gate under the separately reviewed
DC-35 governance transaction. Neither DC-45 acceptance nor cutover authorizes bootstrap. DC-39
architect review v1
required authority over the public canonical serializer and strict Ed25519 signature shape. The
design repair adds those rules, the invalid-predecessor `add_signature` invariant, and deterministic
diagnostic multiplicity/order while retaining the DC-34 preimage vector, canonical envelope tuple,
format-1 diagnostics, writer inventory, and companion RefUpdate no-clock erratum. Architect design
re-review v1 accepted the repaired design on 2026-07-22. Implementation repair re-review v2 accepted
the bounded candidate, committed as `8f565f2`; post-commit evidence review v1 accepted independent
no-hardlink checkout and deterministic-archive evidence on 2026-07-22. DC-39 implementation is
complete. This closure does not authorize release activity.
DC-38 and DC-40 designs, including the DC-40 companion state-root/format FDD, were accepted on
2026-07-14. DC-40 architect implementation review v1 required four repairs covering strict format-2
read admission, anchored mutation authority, exact legacy cleanup authority, and the end-to-end
format-1 command matrix. Architect repair re-review v1 accepted the repaired candidate, committed as
`70c3902`; post-commit evidence review v1 accepted independent checkout/archive identity and focused
plus full regression evidence on 2026-07-23. DC-40 implementation delivery is complete. Release
activation is parked: no signer bootstrap, hold, or RC has started. If the project owner explicitly
activates 0.18.0 preparation, the first release-lane action is the separately reviewed initial DC-35
release-signer bootstrap governance transaction.

**Conditional M1 first-shipping-release activation order (currently labeled 0.18.0):**

1. Prepare the initial DC-35 signer-bootstrap transaction as an isolated public governance change with
   the required non-secret proof, two distinct accountable approvals, and branch-governance evidence.
2. After that transaction becomes public, keep release publication blocked for at least 72 hours.
3. During the hold, re-run the literal older-review stale-pointer/ahead-log crash reproduction against
   the accepted DC-38 state machine and record the exact result as the B1 release-condition check.
4. During the hold, align README and other tracked public portability claims with DC-37's accepted
   0.18.0 support matrix: mutation is experimental on qualifying Linux local filesystems; macOS and
   Windows remain read-only/diagnostic unless the required primitives and crash evidence are reviewed
   before RC. Record in durable tracked authority that the excluded NFR-PORT-01 source is historical
   input, DC-37 is current 0.18.0 authority, and broader cross-platform mutation remains a deferred
   design target rather than current support or a silently waived goal.
5. After the minimum interval and required containment/classification checks, obtain the explicit
   architect/security hold-lift ruling required by DC-35.
6. Only then prepare the combined 0.18.0 RC, refresh all release assets before publication, run the full
   relevant gate and corrective failpoint matrix, and request adversarial RC review.

This sequence is dormant until the reviewed tracked activation transition. It carries forward unchanged
if a later target first ships the unshipped M1 increments. The bootstrap transaction, hold-lift ruling,
and RC are separate authority decisions. Evidence work and documentation correction during the hold do
not shorten it. The cosmetic unknown/malformed-marker diagnostic remains optional and does not block the
ordered sequence unless separately selected.

**Release condition:** all five blocking findings are closed by accepted implementation review; the
reproduced ref failure no longer succeeds; the state-root and signature vectors are pinned; format-1/
format-2 behavior is explicit; release/status documentation is current; the full relevant gate set and
corrective failpoint matrix have observed passing evidence; and an adversarial 0.18.0 release-candidate
review accepts the combined state. No production or public-preview claim follows automatically.

## M2 - Assurance and distribution baseline

**Development status:** active at DC-41 design review against the accepted corrective baseline.

**Eventual release target:** 0.19.0, subject to explicit release activation and all applicable release
gates.

**RFCs:**

1. DC-41 Integrity Evidence Campaign.
2. DC-59 Commit Benchmark Harness (**implemented `a9c2fe0`**), DC-56 Commit Full-Tree Scan Compliance,
   DC-57 Active-Patch Thresholds (**held** — see the capability gap above), DC-58 Source-Structure Audit
   (batch 1 at `e1d0213`). DC-56/57/58 supersede DC-42, archived 2026-07-29; DC-59 was split from DC-56 at
   design review and produces NFR-PERF-01's named evidence artifact.
3. DC-43 Release Security and Distribution Controls.
4. DC-45 Release Policy Tooling Consolidation.
5. DC-46 Workspace Rust 1.85 Compatibility.
6. DC-47 Stable Clippy Gate Alignment (complete at `ea95e92`; post-commit evidence accepted).
7. DC-48 Legacy Clippy Production Retirement (complete at `383e503`; post-commit evidence accepted).
8. DC-49 Portable-Logic Platform Matrix (blocked on the M1 portability-claim correction).
9. DC-50 First-Party SHA-256 ROI Decision (closed at `4005efb` with a **replace** decision; produced no
   code and authorized exactly one successor, DC-55).
10. DC-51 Product Dependency Placement Gate.
11. DC-52 Python and Oracle Decommissioning.
12. DC-55 First-Party SHA-256 Replacement (the implementation DC-50 authorized; identity-bearing;
    accepted 2026-07-28, **implementation complete at `753ebab`**, implementation re-review accepted
    2026-07-29).

DC-49 through DC-52 were added on 2026-07-28 to give an owner to obligations that previously existed only
in architect review prose. DC-55 was added the same day for the same reason — DC-50's replace decision
would otherwise have been an authorization living only in a decision record. Of these five, DC-50 is
accepted and closed, DC-51 is accepted and implemented at `d3e939b`, and **DC-55 is accepted and
implemented at `753ebab`**; DC-49 and DC-52 remain proposed and each requires individual design
acceptance.
DC-49 is the platform matrix descoped from DC-41 and is the only development increment blocked on a
release-lane event. DC-53 (repository-wide AUTHOR trust verification) is recorded as a post-M2 capability
gap and is deliberately not part of M2. The recommended sequence across all open work is in
`rfcs/EXECUTION-ORDER.md`.

DC-45 through DC-48 are preparatory M2 tooling and compatibility increments that landed before M1
release. They are not the remaining post-0.18.0 execution sequence. DC-45's design was accepted after
architect repair re-review v1 on
2026-07-16. Profile hardening and the observation adapter were committed, and architect implementation
repair re-review v1 accepted the exact-byte oracle semantics on 2026-07-17. Project-owner acceptance was
initially withheld pending a compact tracked representation that avoided the candidate's 237 per-case
vector files. Architect footprint QA conditionally approved three strict suite packs, and architect design
amendment re-review v1 accepted the pack, location, closure, and archive contract on 2026-07-17. Compact
implementation was then prepared without staging for implementation re-review. Owner acceptance,
isolated commit, and source-archive evidence were required to precede Rust implementation. Compact
implementation review v1 found one blocking dot-segment grammar defect. Architect repair re-review v1 accepted its
narrow repair on 2026-07-17. Architect design repair re-review v1 accepted the
explicit retirement schedule on 2026-07-17, satisfying the lifecycle-design condition for that separate
owner decision. Five Python oracle authoring/verification files remain through the first Rust-gated
0.19.0 release. The first later release-candidate increment is blocked until an architect accepts the
later-commit stability rerun; the following release-candidate increment is blocked until the exhaustive
five-file decommissioning review removes each file or records an individual owner-approved, event-bound
exception. The accepted Rust implementation was required to replace the complete manifest verifier and
self-test matrix, and later did so. The other eight frozen evidence/contract files remain until a later
equivalence-backed replacement/consolidation
review or an explicit final-retirement review closes migration and rollback needs. These blockers
remain durably tracked if DC-45 moves to `done/` before their completion. The project owner committed
the exact 13-file oracle with the reviewed design/status update as stage-1 freeze commit `47aec9c` on
2026-07-17. Deterministic archive, checkout/extracted verification, direct-dependency/identity, and
seven-product-package exclusion evidence was accepted after architect post-commit evidence review v1
on 2026-07-17. Stage-2 Rust implementation was accepted after architect repair re-review v11 and
committed as `6a65a35` on 2026-07-21. Its deterministic archive, isolated checkout/extraction,
Python/Rust engine, differential, boundary, reference, identity, and seven-product-package exclusion
evidence was accepted after architect post-commit evidence review v1 on 2026-07-21. Preparation of an
isolated authoritative-command cutover candidate and disposable rollback rehearsal was then authorized.
Preparation found that the accepted stale-reference gate hardcodes Python live authority and cannot
validate an inventory/documentation-only Rust switch without a Rust-source transition repair. Focused
architect QA v1 accepted a separate exact two-state transition repair before cutover implementation
resumed. Architect implementation review v1 accepted the Python-primary repair, and it was committed
as `2bfb7cc` on 2026-07-21. Post-commit preservation evidence was accepted after architect review v1
on 2026-07-21. The exact four-file inventory/live-reference cutover was committed as `6a8e365`;
deterministic archive, clean checkout/extraction, full gate, and committed-identity rollback evidence
was accepted after final architect ruling v1 on 2026-07-21. The Rust command is governance-
authoritative. Python and the frozen oracle remain required through the first Rust-gated 0.19.0 release
and an accepted later-commit stability rerun.
The remaining M2 development order is DC-41, then DC-59/DC-56/DC-57/DC-58, then DC-43. This is
program sequencing, not implementation authority: each remains proposed until its individual design
review is accepted. DC-41's former DC-36-through-DC-40 implementation dependency is satisfied; it
may start now against the accepted committed baseline. Release-specific reproduction and gate evidence
must be rerun when an RC is explicitly selected; development evidence does not silently become RC
evidence. DC-56 follows that evidence baseline; read-only measurement before then does not authorize
optimization or broad source moves. DC-43 follows them, requires security/architect design review, and
must consume the stable Rust release-policy gate rather than extend the retained Python oracle. DC-43
completion remains required before any public-preview reconsideration. Separately authorized read-only
DC-56 measurement or credential-free DC-43 policy drafting may be prepared earlier, but proposed RFCs
never authorize implementation.
DC-46 design selected restoration of the declared Rust 1.85 locked-workspace contract through three
bounded source rewrites, focused trust regressions, and pinned locked CI gates. Architect design
rereview v1 accepted it on 2026-07-21. Architect command-grammar amendment QA v1 then authorized five
exact ordinary-Cargo vectors and existing scanner tests after the prepared candidate exposed a DC-45
classifier conflict. Architect implementation review v1 accepted the complete candidate on 2026-07-21;
it was committed as `0d221af`, and architect post-commit evidence review v1 accepted its clean
checkout/archive evidence. DC-46 and the Rust 1.85 compatibility blocker are complete; DC-45 does not
silently absorb this resolved product-workspace mismatch.
DC-47 was the accepted pre-0.19.0 release-candidate correction for the then-remaining Clippy command
divergence: DC-35 public release guidance selects `--all-features`, while current stable CI and the
DC-45 governed classifier selected the no-all-features vector. Its design preserved the stronger
release gate and added one exact non-authority classifier production. Architect design review v1
accepted the bounded design on 2026-07-21. Architect legacy-vector test-contract QA v1 resolved the
retained-vector contradiction and authorized bounded implementation. Architect implementation review
v1 accepted the candidate on 2026-07-21, committed as `ea95e92`. Architect post-commit evidence review
v1 accepted its clean checkout/archive evidence on 2026-07-21, completing DC-47. DC-48 then separately
retired both unconsumed legacy Clippy productions required before the 0.19.0 release candidate; its design
was accepted after architect review v1 on 2026-07-22 with exact bare/prefixed A/B rejection evidence
required. Architect implementation review v1 accepted the bounded candidate, committed as `383e503`.
Architect post-commit evidence review v1 accepted its clean checkout/archive evidence on 2026-07-22,
completing DC-48 and the legacy-Clippy-production blocker.

**Current DC-45 through DC-48 disposition:** Rust release-policy authority, Rust 1.85 compatibility,
stable all-features Clippy alignment, and legacy Clippy production retirement are complete. Only DC-45's
event-bound obligations remain: retain Python and the frozen oracle through the first Rust-gated 0.19.0
release, obtain accepted later-commit stability evidence, and complete the separately reviewed Python/
oracle retirement or consolidation events. These obligations do not alter the DC-41 through DC-43
program order.

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

## Deferred until selected

- Merge execution, branch lifecycle expansion, remotes/sync, rollback publication, plugins/audit, and
  key lifecycle features are no longer frozen solely by release status. They remain unselected and
  require design-first prioritization.
- TASK-14 through TASK-16 documentation themes remain queued. TASK-13 is the narrow exception because
  compatibility and release rules are required for the corrective format transition.
- Any newly discovered correctness or identity defect interrupts this sequence and receives its own
  RFC or an explicit amendment to the owning proposed RFC before implementation.
