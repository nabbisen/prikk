//! Repository verification routines.
//!
//! Verification is read-only. It checks object identity, object-type placement, envelope decoding,
//! sealed block references, joint ref publication state, active WAL replay checksums, and retained
//! active-publication cleanup state. Mutation belongs to narrow doctor or signer-backed seal paths.
//!
//! **A check's own code being present does not establish that a defect actually reaches it.** Earlier
//! gates in this module's pipeline can intercept a malformed input before a specific, later check ever
//! sees it -- so a check existing, and even a fixture that constructs the shape that check is meant to
//! reject, are not proof the check is exercised. Two independent instances (DC-95 Stage 1 rounds 10 and
//! 11): `ref_publication::require_retained_evidence` reclassifies several `refs/verify.rs` codes before
//! they're returned, so a raw pointer/log-shape fixture can silently land on a different code than the
//! one under test; `crate::format::validate_read_schema`, called from `Wal::replay()` itself, already
//! rejects a malformed-shape signature under `RepositoryFormat::CurrentV2` before
//! `rollback_verify::verify_rollback_draft_wal_records` is ever reached, so the same defect is reachable
//! only under `RepositoryFormat::LegacyV1`. Building a fixture for a specific check in this module means
//! tracing its actual call path from `verify_repository`, not just constructing input shaped to match
//! the check's own condition.
//!
//! **Do not report a result derived from a step that may not have run.** The general form of a rule
//! found three times, from three directions, across DC-95 Stage 1 and Stage 2 (`stage-2-level-1-sweep-
//! ruling-v1.md` §4): a check's presence in the source is not proof a defect reaches it (above); an
//! empty accumulator is not "none found" if its producer did not run to completion; a partial count is
//! not "this many verified" if the stage computing it stopped partway through. All three are the same
//! error seen from different angles -- inferring a result from the absence of evidence when the
//! evidence-gathering step itself may not have happened. `require_retained_evidence`'s own `trust_is_
//! valid` (see [`VerificationStage::PublicationReclassification`] below) is the concrete instance the
//! second form was caught in: `trust_verifier.issues.is_empty()` alone reads `true` when the `Objects`
//! stage failed before checking a single Block or RefState, which is not the same fact as "every
//! relevant object was checked and found trustworthy." It is `issues.is_empty() && objects_evaluated`
//! for exactly this reason. The third form is why `RepositoryVerification`'s per-stage counts
//! (`checked_objects` and its siblings) are `Option<usize>`, `None` rather than a partial number, when
//! their producing stage did not evaluate to completion -- a partial count is a completeness claim in
//! miniature, and `verify_objects`'s own topological pass over the whole object store means a partial
//! per-type count says nothing about whether the objects it did see are individually sound.
//!
//! # DC-95 Stage 2, Level 1: scope containment
//!
//! `verify_repository`'s pipeline is twelve stages (see [`VerificationStage`]), each a `?`-propagating
//! call in the pre-Stage-2 source. Stage 1 (above) proved which checks inside those stages are load-
//! bearing; Stage 2 Level 1 changed what happens when one fails. **Before:** the first hard error
//! anywhere aborted `verify_repository` entirely -- every later stage silently never ran, and
//! `print_verify_report` never printed anything but the one error string. **After:** each stage's own
//! `?` is caught at the boundary and recorded as a [`StageOutcome`] rather than propagated; the
//! pipeline continues to the remaining stages regardless. A `Failed` stage's own error becomes its
//! `StageOutcome`'s message; a stage that could not run because a real dependency ([`StageStatus::
//! NotEvaluated`], naming that dependency) is blocking on the same footing -- an incomplete
//! verification is not a passing one, so `RepositoryVerification::has_stage_failure` covers both, plus
//! a third state, [`StageStatus::Halted`], for `--stop-on-first-error`: a stage that was never attempted
//! because an *unrelated* earlier stage's failure already stopped the walk. `Halted` is kept distinct
//! from `NotEvaluated` because `blocked_by` is a dependency-graph claim -- reporting `NotEvaluated {
//! blocked_by: Objects }` for `LifecycleCache`, which does not depend on `Objects` at all, would assert
//! an edge that does not exist (implementation review v1 §4). The distinction only matters under
//! `--stop-on-first-error`; in the default full-accumulation walk, every `NotEvaluated` names a real
//! dependency and `Halted` never appears.
//!
//! **The Stage 1 classification table below is unchanged by this**: which checks are load-bearing,
//! downstream-redundant, excluded, or unreachable is a fact about the checks themselves, not about how
//! their failures propagate out of `verify_repository`. What changed is the shape a reader (or
//! `doctor_repository`, or `prikk verify`'s own exit-code chain) sees a failing check through: a
//! `StageOutcome` against the owning stage, not a bare `Result::Err` from the whole function. Checks
//! were not rewritten, moved, or deleted to get here -- only the twelve top-level boundaries around
//! them.
//!
//! # DC-95 Stage 1: end-to-end coverage, by cluster
//!
//! Every check `verify_repository` performs, classified by whether disabling it lets a defective
//! repository pass through `verify_repository` as `Ok` with no trace -- **Load-bearing** (the check is
//! the last line of defence; some are load-bearing only via a non-blocking-sibling mechanism, named
//! where that applies); **Downstream-redundant** (something else independently catches the same defect,
//! blocking, under a different code); **Excluded** (non-blocking finding, out of mandatory scope);
//! **Unreachable** (provably impossible to construct -- kept, untested, ruled on explicitly, not merely
//! unattempted). Full reasoning for each row lives in the test file cited, not duplicated here; this
//! table is the current-state index, not the round-by-round record (that's
//! `DC-95-VERIFY-COVERAGE-AND-FINDING-ACCUMULATION.md`'s own handoff trail).
//!
//! **Scope limit**: this enumerates checks `verify` *has*, not checks it *should have* -- a gap of a
//! different kind this method cannot surface. One known instance: `refs/received/` is never read by
//! `verify_repository` at all (RFC 101 §5.2's independently-derived transition trace); registered in
//! `FINDINGS.md`, not a row here, since there is no existing check to classify.
//!
//! ## `verify/objects.rs` + `block_state.rs` (`verify/tests.rs`)
//!
//! | Check | Classification |
//! |---|---|
//! | Block parent-block existence | Downstream-redundant (`validate_v2_lineage`) |
//! | Block patch existence | Downstream-redundant (lifecycle-replay layer's own read) |
//! | Block snapshot-blob existence | Load-bearing |
//! | Block format-2 shape validation (8 arms) | Load-bearing, all 8 |
//! | Topological cycle detection | Unreachable (needs a SHA-256 fixed point; unit-level substitute in `block_state/tests.rs`) |
//! | Envelope type mismatch / object id mismatch | Load-bearing, both |
//! | `validate_read_schema` strict-signature-shape | Load-bearing, via non-blocking-sibling mechanism |
//! | Publication-trust failure (Block/RefState) | Demonstrated via trusted/untrusted contrast |
//! | Directory/file shape structural errors | Downstream-redundant, both sub-arms |
//!
//! ## `refs/verify.rs` + `refs/verify/scan.rs` (`verify/tests/ref_cluster.rs`)
//!
//! | Check | Classification |
//! |---|---|
//! | Incomplete log tail without pointer lead | Load-bearing |
//! | `LEGACY-LOG-LEADS` (format-1) | Downstream-redundant (format-2 `DIVERGENCE` sibling equally blocks) |
//! | Catch-all "unexplained pointer/log divergence" | Load-bearing |
//! | `LEGACY-TIMESTAMP` (format-1) | Excluded, non-blocking |
//! | `created_at == 0` (format-2) | Load-bearing |
//! | `CANDIDATE-DEBRIS` | Non-blocking |
//! | Duplicate pointer identity / duplicate ref-log identity | Unreachable, both (needs a genuine SHA-256 collision) |
//! | Non-canonical ref pointer path | Load-bearing |
//! | RefState name mismatches pointer | Downstream-redundant (`classify_ref_state`'s own coherence arm) |
//! | `ensure_ref_target_valid` (dangling Branch/Tag target) | Load-bearing |
//! | Ref-log chain/sequence divergence | Load-bearing |
//! | `verify_update` RefState/RefUpdate coherence | Load-bearing |
//! | RefState unsigned | Downstream-redundant (`PublicationTrustVerifier`) |
//! | `ensure_ref_path_shape` (`by-id/`, `logs/`) | Downstream-redundant, provably, both |
//! | Signature-envelope issues, `RefLog` source | Excluded (see `signature_envelope_issues` caveat below) |
//!
//! ## `verify/ref_publication.rs`
//!
//! `mark_unproved` reclassification and `ACTIVE-CLEANUP-PENDING` were both already end-to-end covered
//! before DC-95 started; not part of Stage 1's gap-closing scope.
//!
//! ## `wal.rs` / `verify_wal_persistence` / `rollback_verify.rs` (`verify/tests/wal_cluster.rs`)
//!
//! | Check | Classification |
//! |---|---|
//! | `Wal::replay()` checksum mismatch | Load-bearing |
//! | `verify_wal_persistence` type mismatch | Load-bearing |
//! | Rollback WAL envelope type | Unreachable (`is_rollback_draft_envelope` already guarantees Patch type before this check runs) |
//! | Rollback WAL decode (op_seq contiguity) | Load-bearing |
//! | Rollback WAL apply-support (`DeleteNode(symlink)`) | Load-bearing |
//! | Rollback WAL empty-ops | Unreachable (`decode_patch_operations` already errors before returning empty) |
//! | Rollback AUTHOR signature: missing | Load-bearing |
//! | Rollback AUTHOR signature: wrong algorithm | Unreachable (`SignatureAlgorithm` has exactly one variant) |
//! | Rollback AUTHOR signature: legacy marker key id | Load-bearing |
//! | Rollback AUTHOR signature: wrong length | Load-bearing, via non-blocking-sibling mechanism; format-1 only |
//! | Signature-envelope issues, `ActiveWal` source | Excluded (see caveat below) |
//!
//! ## Active-WAL metadata status + WAL ordering (DC-66, `verify/tests.rs`)
//!
//! | Check | Classification |
//! |---|---|
//! | `MissingForEmptyWal` / `ValidForEmptyWal` | Excluded, non-blocking, both |
//! | `InvalidForNonEmptyWal` | Load-bearing |
//! | Active-WAL ordering violations | Load-bearing |
//!
//! ## `verify/trust.rs` / `trust.rs` (`verify/tests/trust.rs`)
//!
//! | Check | Classification |
//! |---|---|
//! | `PRIKK-TRUST-POLICY-INVALID` (missing/malformed policy) | Load-bearing |
//! | `PRIKK-TRUST-PUBLICATION-UNTRUSTED` | Load-bearing |
//!
//! ## `commit_index.rs` (DC-56) / `lifecycle_cache/incremental.rs` (DC-64) (`crates/prikk-cli/tests/dc64_baseline_cache.rs`)
//!
//! | Check | Classification |
//! |---|---|
//! | Commit-index content divergence | Load-bearing |
//! | Lifecycle-cache content-disagrees divergence | Load-bearing |
//! | Lifecycle-cache "could not be independently verified" | Load-bearing (horizon-anchored replay vs. `block_state.rs`'s non-horizon-anchored one -- the one genuine asymmetry between two otherwise-equivalent replay paths) |
//!
//! ## Standing caveat: `signature_envelope_issues`
//!
//! `signature_envelope_issues` (one `Vec` on [`RepositoryVerification`], populated from every
//! `SignatureEnvelopeSource`) backs no `has_*` blocking predicate, for any source -- an open question,
//! not a settled design: should the `MALFORMED` variant be wired into a blocking predicate? Every
//! "Excluded" row above that names this caveat, plus every "Load-bearing, via non-blocking-sibling
//! mechanism" row, depends on this staying `false`. If it's ever answered the other way, those rows
//! reopen.

use std::path::PathBuf;

mod objects;
mod ref_publication;
mod trust;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectId, ObjectType};

use crate::active::{ActiveRefMetadata, read_active_ref_metadata};
use crate::block_state::{BlockStateOutcome, BlockStateStatus};
use crate::commit_index::{CommitIndexDivergence, verify_divergence};
use crate::layout::{RepositoryFormat, RepositoryLayout};
use crate::lifecycle_cache::incremental::{
    LifecycleCacheDivergence, verify_divergence as verify_lifecycle_cache_divergence,
};
use crate::object_store::FileObjectStore;
use crate::refs::verify_refs;
use crate::rollback_verify::{verify_rollback_draft_wal_records, verify_rollback_patch_envelope};
use crate::signature_diagnostics::{
    SignatureEnvelopeIssue, SignatureEnvelopeSource, classify_signature_envelope,
};
use crate::trust::PublicationTrustIssue;
use crate::wal::Wal;

use objects::verify_objects;
pub use objects::{ObjectItemOutcome, ObjectItemStatus};
use trust::PublicationTrustVerifier;

/// Verification summary for a single persisted object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectVerification {
    /// The object ID parsed from the object filename.
    pub object_id: ObjectId,
    /// The object type implied by the directory being scanned.
    pub object_type: ObjectType,
    /// The object file path that was checked.
    pub path: PathBuf,
    /// Rollback-marked Patch references verified for this object when it is a Block.
    pub rollback_patch_count: usize,
    /// For a Block or RefState, the adopted MAINTAINER key id whose signature was trusted (DC-78
    /// §D3). `None` for other object types, or when publication trust could not be established.
    pub sealed_by_key_id: Option<String>,
}

/// Which adopted MAINTAINER key sealed a given Block (DC-78 §D3). Reporting only: the sealer's key
/// id already lives, non-strippably, inside the block's own signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSealVerification {
    /// The sealed Block's object id.
    pub block_id: ObjectId,
    /// The MAINTAINER key id whose trusted signature matched this Block.
    pub sealed_by_key_id: String,
}

/// One of the twelve top-level scopes `verify_repository`'s pipeline is organized into (DC-95 Stage 2
/// Level 1: scope containment). Named in pipeline order; the order itself is load-bearing for
/// `NotEvaluated` naming (`StageStatus::NotEvaluated`'s `blocked_by` is always an earlier stage).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerificationStage {
    /// Persisted-object scan: identity, placement, envelope decoding, sealed block references,
    /// publication trust for Block/RefState (shares `PublicationTrustVerifier` with
    /// `RefUpdateSchemaTrust`).
    Objects,
    /// Joint ref pointer/log verification: structural shape, publication-state classification.
    Refs,
    /// Per-`RefUpdate`-envelope format-read-schema validation and publication trust (shares
    /// `PublicationTrustVerifier` with `Objects`).
    RefUpdateSchemaTrust,
    /// Active-WAL replay: framing, checksums, envelope decoding.
    WalReplay,
    /// Active-WAL patch persistence cross-check against the object store.
    WalPersistence,
    /// Active-WAL rollback-draft classification and structural validation.
    RollbackDrafts,
    /// Per-active-WAL-record format-read-schema validation and signature-envelope classification.
    WalRecordSchema,
    /// Active-WAL ref-ownership metadata classification.
    ActiveWalMetadata,
    /// Retained-evidence reclassification of interrupted-publication ref issues.
    PublicationReclassification,
    /// DC-56 commit-index cache divergence check.
    CommitIndex,
    /// DC-64 incremental lifecycle-state cache divergence check.
    LifecycleCache,
    /// DC-66 active-WAL queue-ordering check.
    WalOrdering,
}

impl VerificationStage {
    /// Stable, lowercase-hyphenated scope name for diagnostics and CLI output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Objects => "objects",
            Self::Refs => "refs",
            Self::RefUpdateSchemaTrust => "ref-update-schema-trust",
            Self::WalReplay => "wal-replay",
            Self::WalPersistence => "wal-persistence",
            Self::RollbackDrafts => "rollback-drafts",
            Self::WalRecordSchema => "wal-record-schema",
            Self::ActiveWalMetadata => "active-wal-metadata",
            Self::PublicationReclassification => "publication-reclassification",
            Self::CommitIndex => "commit-index",
            Self::LifecycleCache => "lifecycle-cache",
            Self::WalOrdering => "wal-ordering",
        }
    }
}

impl std::fmt::Display for VerificationStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Outcome of attempting to evaluate one verification stage (DC-95 Stage 2 Level 1). **No stage may be
/// silently absent from a report.** A stage's own check raising an error is recorded as a blocking
/// finding against its scope rather than aborting the rest of verification (`Failed`); a stage that
/// could not run because a real dependency did not evaluate is itself blocking, not silently skipped
/// (`NotEvaluated`); a stage that could have run on its own terms but was preempted by an operator-
/// requested early stop is also blocking, but for a different reason it must not be confused with
/// (`Halted`) — a repository whose verification is incomplete is not verified, regardless of which of
/// the three non-`Evaluated` states explains the gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageStatus {
    /// The stage ran to completion; its findings and counts are authoritative.
    Evaluated,
    /// The stage's own check raised an error.
    Failed {
        /// The error the stage raised.
        message: String,
    },
    /// The stage could not run because a *real* dependency did not evaluate — `blocked_by` names a
    /// stage this one's own logic actually reads output from. This is a dependency-graph claim, and
    /// must remain true of the graph even when `--stop-on-first-error` is in effect; see `Halted` for
    /// the case where a stage merely followed an unrelated earlier stop.
    NotEvaluated {
        /// The earlier stage whose own non-evaluation is why this one could not run.
        blocked_by: VerificationStage,
    },
    /// The stage was never attempted because an earlier, *unrelated* stage's failure already stopped
    /// the walk under `--stop-on-first-error` (DC-95 Stage 2 Level 1 implementation review v1 §4) —
    /// `after` names the stage whose failure triggered the stop, not a dependency of this stage. Kept
    /// distinct from `NotEvaluated` because `blocked_by` is a dependency-graph claim: reporting
    /// `NotEvaluated { blocked_by: Objects }` for a stage that does not actually depend on `Objects`
    /// (e.g. `LifecycleCache`) would assert an edge that does not exist.
    Halted {
        /// The stage whose failure caused the walk to stop before this stage was reached.
        after: VerificationStage,
    },
}

impl StageStatus {
    /// Return true for any status other than a clean, completed evaluation. `NotEvaluated` and
    /// `Halted` are both blocking on the same footing as `Failed` — an incomplete verification is not
    /// a passing one, whichever of the three explains the gap.
    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        !matches!(self, Self::Evaluated)
    }
}

/// One stage's resolved outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageOutcome {
    /// Which of the twelve stages this outcome is for.
    pub stage: VerificationStage,
    /// How that stage resolved.
    pub status: StageStatus,
}

/// Repository verification summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryVerification {
    /// True when format-1 scaffold roots cannot be verified as clean-state commitments. A precondition
    /// fact about the repository's format, not sourced from any stage — always known.
    pub legacy_state_roots_unverifiable: bool,
    /// Outcome of each of the twelve verification stages (DC-95 Stage 2 Level 1), in pipeline order.
    /// Always exactly twelve entries — no stage may be silently absent.
    pub stage_outcomes: Vec<StageOutcome>,
    /// Phase A: one outcome per persisted object file scanned, in scan order (DC-95 Stage 2 Level 2).
    /// Empty when the `Objects` stage itself did not evaluate (a structural directory-shape error) —
    /// nothing was attempted, distinct from a non-empty set where every entry happens to be `Failed`.
    pub object_outcomes: Vec<ObjectItemOutcome>,
    /// Phase B: one outcome per `CurrentV2` Block whose Phase A check succeeded, in the
    /// state-dependency order `verify_blocks_topological` resolved them — not scan order (DC-92
    /// §4.2). Empty when the `Objects` stage did not evaluate, or when no `CurrentV2` Block passed
    /// Phase A at all.
    pub block_state_outcomes: Vec<BlockStateOutcome>,
    /// Number of persisted object files whose own Phase A checks ran to completion (DC-95 Stage 2
    /// Level 2). `None` only when the `Objects` stage itself did not evaluate (a structural
    /// directory-shape error) — under item containment this is no longer the same claim as "every
    /// object in the store is individually sound": some entries in `object_outcomes` may themselves
    /// be `Failed` while this count still reflects how many succeeded. Never a partial claim about
    /// state-root soundness, which `block_state_outcomes` is the only source of truth for (Level 2
    /// handoff §7 Q3 — `checked_blocks` below keeps its pre-Level-2 meaning unchanged).
    pub checked_objects: Option<usize>,
    /// Number of active WAL records replayed successfully. `None` when the WAL-replay stage did not
    /// evaluate to completion.
    pub checked_wal_records: Option<usize>,
    /// Number of persisted Block objects whose references (parent, patch, snapshot existence, merge
    /// baseline) were checked successfully — a Phase A claim only, never a claim about state-root
    /// soundness (see `block_state_outcomes`). `None` only when the `Objects` stage itself did not
    /// evaluate. This field's meaning is unchanged by Level 2 (handoff §7 Q3) — only *when* it is
    /// `None` changed, from "the whole stage failed" to "the whole stage did not evaluate at all."
    pub checked_blocks: Option<usize>,
    /// Number of persisted Block objects classified as rollback blocks, among those whose Phase A
    /// check succeeded. `None` only when the `Objects` stage itself did not evaluate.
    pub checked_rollback_blocks: Option<usize>,
    /// Number of sealed rollback-marked Patch objects referenced by Blocks whose Phase A check
    /// succeeded. `None` only when the `Objects` stage itself did not evaluate.
    pub checked_sealed_rollback_patches: Option<usize>,
    /// Number of active WAL patch records that already exist as persisted patch objects. `None` when
    /// the WAL-persistence stage did not evaluate to completion.
    pub persisted_wal_patches: Option<usize>,
    /// Number of ref pointer files whose own Phase-A-equivalent read succeeded (DC-95 Stage 2
    /// Level 2). `None` only when the `Refs` stage itself did not evaluate.
    pub checked_refs: Option<usize>,
    /// Number of inline ref-log records read successfully. `None` only when the `Refs` stage
    /// itself did not evaluate.
    pub checked_ref_log_records: Option<usize>,
    /// Interrupted ref-publication and candidate-debris conditions found by joint verification. Stays
    /// a plain `Vec` under stage containment: entries already pushed by a stage that later failed
    /// remain real findings; only the count/emptiness-as-proof reasoning needed a stage-aware guard
    /// (see `require_retained_evidence`'s own `trust_is_valid` computation).
    pub ref_publication_issues: Vec<crate::refs::RefPublicationIssue>,
    /// One outcome per ref pointer file scanned, in scan order (DC-95 Stage 2 Level 2). Empty when
    /// the `Refs` stage itself did not evaluate.
    pub pointer_outcomes: Vec<crate::refs::RefFileOutcome>,
    /// One outcome per ref log file scanned, in scan order. Empty when the `Refs` stage itself did
    /// not evaluate.
    pub log_outcomes: Vec<crate::refs::RefFileOutcome>,
    /// One outcome per ref name reached via a successfully-read pointer or log. Empty when the
    /// `Refs` stage itself did not evaluate.
    pub ref_item_outcomes: Vec<crate::refs::RefItemOutcome>,
    /// Warning-level format-1 signature-envelope compatibility findings in deterministic order.
    pub signature_envelope_issues: Vec<SignatureEnvelopeIssue>,
    /// Number of active WAL records classified and decoded as rollback drafts. `None` when the
    /// rollback-drafts stage did not evaluate to completion.
    pub checked_rollback_draft_records: Option<usize>,
    /// Number of publication envelopes checked against repository-local trust. `None` unless *both*
    /// the objects stage and the ref-update schema/trust stage evaluated to completion — this count is
    /// contributed to by both, sharing one `PublicationTrustVerifier` instance across them.
    pub checked_publication_trust_records: Option<usize>,
    /// Publication-trust issues found while structural verification succeeded. Stays a plain `Vec` —
    /// entries genuinely found before an interrupting failure remain real findings.
    pub publication_trust_issues: Vec<PublicationTrustIssue>,
    /// Recognized non-authoritative object publication temps left for explicit maintenance.
    pub object_temp_paths: Vec<PathBuf>,
    /// Number of trailing bytes in the active WAL that look like an incomplete final record. `None`
    /// when the WAL-replay stage did not evaluate to completion.
    pub trailing_partial_wal_bytes: Option<usize>,
    /// Active-WAL ref metadata status relative to the replayed WAL. `None` when the active-WAL-metadata
    /// stage did not evaluate to completion.
    pub active_wal_metadata_status: Option<ActiveWalMetadataStatus>,
    /// DC-56 commit-index entries whose recorded content hash disagrees with the worktree's actual
    /// current content despite a matching stat — a stale-but-trusted cache entry, reported per the
    /// cache-validity specification §6 rather than silently trusted by a future commit.
    pub commit_index_divergences: Vec<CommitIndexDivergence>,
    /// DC-64 incremental lifecycle-state cache entries whose contents disagree with an independent
    /// full replay of the block they claim to represent — reported per the design document §6
    /// rather than silently trusted by a future commit.
    pub lifecycle_cache_divergences: Vec<LifecycleCacheDivergence>,
    /// DC-66: active WAL queue-ordering violations — a record whose sequence does not strictly
    /// increase over its predecessor. Adversarial-only under normal operation (`Wal::append_patch`
    /// always assigns the next sequence), but a queue of N gives ordering a meaning ("patches seal in
    /// append order") worth verifying explicitly rather than assuming from decode success alone.
    pub active_wal_ordering_issues: Vec<ActiveWalOrderingIssue>,
    /// DC-75: `Merge` blocks whose recorded `merge_baseline_block_id` is not, in fact, a common
    /// ancestor of both parents — independently re-derived, not trusted, per
    /// `baseline-recording-answer-v1.md` §3 ("record it, then check it, unconditionally"). A recorded
    /// baseline that legitimate merge execution ever produced always passes this; a false claim (data
    /// corruption or tampering) does not.
    pub merge_baseline_divergences: Vec<MergeBaselineDivergence>,
    /// Which adopted MAINTAINER key sealed each checked Block (DC-78 §D3), in on-disk scan order.
    /// Reporting only — surfaces provenance that was already intrinsic to each block's own
    /// signature, so an auditor can ask "which parts of this history did I seal" and get an answer.
    pub block_seals: Vec<BlockSealVerification>,
}

/// A `Merge` block (DC-75) whose recorded `merge_baseline_block_id` is not a common ancestor of its
/// two parents. Precision note: this checks *validity* (is the claim even a common ancestor), not
/// *nearest-ness* (is it the single nearest one) — a merge legitimately sealed against an older-than-
/// necessary common ancestor is unusual but not what this finding is for; a baseline that is not a
/// common ancestor at all can only arise from a forged or corrupted field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeBaselineDivergence {
    /// The `Merge` block whose recorded baseline failed re-derivation.
    pub block_id: ObjectId,
    /// The recorded (claimed) baseline.
    pub recorded_baseline: ObjectId,
    /// The block's mainline parent.
    pub mainline_parent_id: ObjectId,
    /// The block's secondary parent.
    pub secondary_parent_id: ObjectId,
}

/// One active-WAL record whose sequence did not strictly increase over the previous record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWalOrderingIssue {
    /// Zero-based position of the offending record within the replayed WAL.
    pub index: usize,
    /// Sequence of the previous record.
    pub previous_seq: u64,
    /// Sequence of the offending record (not greater than `previous_seq`).
    pub seq: u64,
}

impl RepositoryVerification {
    /// Return true when any of the twelve verification stages did not evaluate cleanly — either its
    /// own check raised an error (`Failed`) or a dependency's non-evaluation prevented it from running
    /// at all (`NotEvaluated`). A repository whose verification did not run to completion is not
    /// verified, regardless of what the stages that did run found. Checked first, ahead of every
    /// finding-specific predicate below: those predicates' own backing data can itself be incomplete
    /// precisely because a stage failed, so this is the more fundamental question.
    ///
    /// **Does not, by itself, cover item-level defects (DC-95 Stage 2 Level 2)** — the `Objects`
    /// stage evaluates cleanly (`Evaluated`) even when one of its items individually failed, since
    /// item containment means a bad object no longer aborts the whole stage. See
    /// [`Self::has_item_failure`] for that question, and [`Self::has_blocking_defect`] for the
    /// combined check almost every caller actually wants.
    #[must_use]
    pub fn has_stage_failure(&self) -> bool {
        self.stage_outcomes
            .iter()
            .any(|outcome| outcome.status.is_blocking())
    }

    /// Return true when any individual item did not evaluate cleanly (DC-95 Stage 2 Level 2) — a
    /// Phase A object whose own check failed, a Phase B `CurrentV2` Block whose state-root check
    /// failed or could not be attempted because its own state-derivation parent did not evaluate,
    /// or a ref (its pointer file, log file, or classification) that failed. Item containment means
    /// these no longer make [`Self::has_stage_failure`] true: the owning stage itself completed, so
    /// this is a genuinely separate question, not a more detailed view of the same one. The backing
    /// `Vec`s are empty (not merely all-`Evaluated`) when their owning stage itself did not
    /// evaluate — this method reads that case as `false`, same as every other item-backed predicate
    /// in this type; `has_stage_failure` is what is already true for it.
    #[must_use]
    pub fn has_item_failure(&self) -> bool {
        self.object_outcomes
            .iter()
            .any(|outcome| matches!(outcome.status, ObjectItemStatus::Failed { .. }))
            || self
                .block_state_outcomes
                .iter()
                .any(|outcome| !matches!(outcome.status, BlockStateStatus::Verified))
            || self
                .pointer_outcomes
                .iter()
                .any(|outcome| matches!(outcome.status, crate::refs::RefFileStatus::Failed { .. }))
            || self
                .log_outcomes
                .iter()
                .any(|outcome| matches!(outcome.status, crate::refs::RefFileStatus::Failed { .. }))
            || self
                .ref_item_outcomes
                .iter()
                .any(|outcome| matches!(outcome.status, crate::refs::RefItemStatus::Failed { .. }))
    }

    /// Return true when this repository's verification found any blocking reason to refuse it --
    /// stage-level (`has_stage_failure`) or item-level (`has_item_failure`). A convenience predicate
    /// for a caller that only wants "is this repository verified at all" and does not care which
    /// half of that question failed.
    ///
    /// **Not currently called by this crate's own production code.** `doctor_repository`'s refusal
    /// gate is preserved by its own per-stage and per-item `DoctorIssue::error` loops feeding
    /// `is_healthy()`, not by calling this directly; `prikk verify`'s exit-code chain
    /// (`main.rs`) calls `has_stage_failure()` and `has_item_failure()` as two separate arms
    /// precisely so it can report *which* kind of failure occurred, rather than one generic
    /// message -- collapsing them here would lose that. Kept as public API for an external caller
    /// that only wants the yes/no answer.
    #[must_use]
    pub fn has_blocking_defect(&self) -> bool {
        self.has_stage_failure() || self.has_item_failure()
    }

    /// Return true when legacy scaffold roots prevent state-commitment verification.
    #[must_use]
    pub const fn has_unverifiable_state_roots(&self) -> bool {
        self.legacy_state_roots_unverifiable
    }

    /// Return true if the active WAL contained an incomplete trailing record. `None` (the WAL-replay
    /// stage did not evaluate) reads as false here — that condition is already surfaced, more
    /// precisely, by `has_stage_failure`.
    #[must_use]
    pub fn has_trailing_partial_wal(&self) -> bool {
        self.trailing_partial_wal_bytes.is_some_and(|n| n != 0)
    }

    /// Return true when all structurally verified publication objects also passed trust checks.
    #[must_use]
    pub fn has_publication_trust_issues(&self) -> bool {
        !self.publication_trust_issues.is_empty()
    }

    /// Return true when pointer/log state requires signer-backed recovery or manual intervention.
    #[must_use]
    pub fn has_blocking_ref_publication_issues(&self) -> bool {
        self.ref_publication_issues
            .iter()
            .any(|issue| issue.blocking)
    }

    /// Return true when a non-empty active WAL lacks valid ownership metadata. `None` (the
    /// active-WAL-metadata stage did not evaluate) reads as false here — see `has_trailing_partial_wal`.
    #[must_use]
    pub fn has_active_wal_metadata_integrity_issue(&self) -> bool {
        self.active_wal_metadata_status
            .as_ref()
            .is_some_and(ActiveWalMetadataStatus::has_integrity_issue)
    }

    /// Return true when an empty active WAL has stale local metadata debris. `None` reads as false —
    /// see `has_trailing_partial_wal`.
    #[must_use]
    pub fn has_active_wal_metadata_warning(&self) -> bool {
        self.active_wal_metadata_status
            .as_ref()
            .is_some_and(ActiveWalMetadataStatus::has_local_debris_warning)
    }

    /// Return true when the commit-index cache disagrees with the worktree for at least one path.
    #[must_use]
    pub fn has_commit_index_divergence(&self) -> bool {
        !self.commit_index_divergences.is_empty()
    }

    /// Return true when the incremental lifecycle-state cache disagrees with an independent replay.
    #[must_use]
    pub fn has_lifecycle_cache_divergence(&self) -> bool {
        !self.lifecycle_cache_divergences.is_empty()
    }

    /// Return true when the active WAL contains an out-of-order or duplicate sequence.
    #[must_use]
    pub fn has_active_wal_ordering_issue(&self) -> bool {
        !self.active_wal_ordering_issues.is_empty()
    }

    /// Return true when a `Merge` block's recorded baseline is not a common ancestor of its parents
    /// (DC-75) — a false claim, from data corruption or tampering.
    #[must_use]
    pub fn has_merge_baseline_divergence(&self) -> bool {
        !self.merge_baseline_divergences.is_empty()
    }
}

/// Active-WAL ref metadata status derived during repository verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveWalMetadataStatus {
    /// Empty active WAL and no metadata.
    MissingForEmptyWal,
    /// Empty active WAL with stale but valid local metadata.
    ValidForEmptyWal {
        /// Ref recorded in the stale metadata.
        ref_name: String,
    },
    /// Empty active WAL with malformed local metadata.
    InvalidForEmptyWal {
        /// Parse or validation failure.
        reason: String,
    },
    /// Non-empty active WAL with valid ownership metadata.
    ValidForNonEmptyWal {
        /// Ref recorded in the active metadata.
        ref_name: String,
    },
    /// Non-empty active WAL missing required ownership metadata.
    MissingForNonEmptyWal,
    /// Non-empty active WAL with malformed ownership metadata.
    InvalidForNonEmptyWal {
        /// Parse or validation failure.
        reason: String,
    },
}

impl ActiveWalMetadataStatus {
    /// Return true when the status represents a repository-integrity issue.
    #[must_use]
    pub const fn has_integrity_issue(&self) -> bool {
        matches!(
            self,
            Self::MissingForNonEmptyWal | Self::InvalidForNonEmptyWal { .. }
        )
    }

    /// Return true when the status represents local debris on an otherwise empty active WAL.
    #[must_use]
    pub const fn has_local_debris_warning(&self) -> bool {
        matches!(
            self,
            Self::ValidForEmptyWal { .. } | Self::InvalidForEmptyWal { .. }
        )
    }
}

/// Threads stage outcomes, and (optionally) an early-halt decision, through `verify_repository`'s
/// pipeline (DC-95 Stage 2 Level 1's `--stop-on-first-error`, design §7 and §12.3).
struct StagePipeline {
    outcomes: Vec<StageOutcome>,
    stop_on_first_error: bool,
    halted_by: Option<VerificationStage>,
}

impl StagePipeline {
    fn new(stop_on_first_error: bool) -> Self {
        Self {
            outcomes: Vec::with_capacity(12),
            stop_on_first_error,
            halted_by: None,
        }
    }

    /// Attempt a stage with no real dependency beyond a possible earlier halt. Returns the value on
    /// success; `None` on failure, or if an earlier stage already halted the walk. A stage reached
    /// through `run` never has a real declared dependency (a stage that does is gated behind an
    /// `if`/`else` at its call site and reaches `not_evaluated` instead when ungated) -- so an
    /// already-halted walk is always reported as `Halted`, never a fabricated `NotEvaluated`.
    fn run<T>(&mut self, stage: VerificationStage, result: Result<T>) -> Option<T> {
        if let Some(halted_by) = self.halted_by {
            self.outcomes.push(StageOutcome {
                stage,
                status: StageStatus::Halted { after: halted_by },
            });
            return None;
        }
        match result {
            Ok(value) => {
                self.outcomes.push(StageOutcome {
                    stage,
                    status: StageStatus::Evaluated,
                });
                Some(value)
            }
            Err(err) => {
                self.outcomes.push(StageOutcome {
                    stage,
                    status: StageStatus::Failed {
                        message: err.to_string(),
                    },
                });
                if self.stop_on_first_error {
                    self.halted_by = Some(stage);
                }
                None
            }
        }
    }

    /// Record a stage that cannot run because `blocked_by` -- a real dependency -- did not evaluate.
    /// Always reports `blocked_by` as given, never substituted: a caller only reaches this method when
    /// `blocked_by`'s own stage failed to produce a usable value (DC-95 Stage 2 Level 1 implementation
    /// review v1 §4), so the claim is true of the dependency graph regardless of *why* `blocked_by`
    /// itself did not evaluate -- including when `blocked_by` was itself `Halted`, in which case this
    /// stage is transitively halted too, discoverable by following the chain rather than by this call
    /// reaching past its own real dependency to name an unrelated stage. `--stop-on-first-error` never
    /// originates a fresh halt here: every halt traces back to a `Failed` outcome from `run`, which
    /// already recorded it.
    fn not_evaluated(&mut self, stage: VerificationStage, blocked_by: VerificationStage) {
        self.outcomes.push(StageOutcome {
            stage,
            status: StageStatus::NotEvaluated { blocked_by },
        });
    }

    /// Record a stage whose own check cannot fail (a pure function, or one that already converts
    /// errors into findings) -- still subject to an earlier halt. Returns whether the stage should
    /// actually run its own work. Never a real dependency (see `run`), so an already-halted walk is
    /// `Halted`, not `NotEvaluated`.
    fn run_infallible(&mut self, stage: VerificationStage) -> bool {
        if let Some(halted_by) = self.halted_by {
            self.outcomes.push(StageOutcome {
                stage,
                status: StageStatus::Halted { after: halted_by },
            });
            false
        } else {
            self.outcomes.push(StageOutcome {
                stage,
                status: StageStatus::Evaluated,
            });
            true
        }
    }
}

/// Options controlling how `verify_repository` walks its twelve stages (DC-95 Stage 2 Level 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerifyOptions {
    /// When true, stop at the first stage that fails or cannot evaluate, leaving every later stage
    /// `NotEvaluated` (naming the first halting stage) rather than continuing to accumulate.
    /// Preserves the pre-Stage-2 bounded-walk behavior for a large, badly-damaged repository where a
    /// full accumulating scan would be costly (design §7) -- unbounded growth is concentrated in the
    /// `Objects` stage's whole-store scan. Default `false` (full accumulation).
    pub stop_on_first_error: bool,
}

/// Verify a repository layout without modifying it, with the default options (full accumulation
/// across all twelve stages). See [`verify_repository_with_options`] for `--stop-on-first-error`.
pub fn verify_repository(layout: &RepositoryLayout) -> Result<RepositoryVerification> {
    verify_repository_with_options(layout, VerifyOptions::default())
}

/// Verify a repository layout without modifying it.
pub fn verify_repository_with_options(
    layout: &RepositoryLayout,
    options: VerifyOptions,
) -> Result<RepositoryVerification> {
    let object_store = FileObjectStore::new(layout.clone());
    let mut trust_verifier = PublicationTrustVerifier::new(layout);
    let mut pipeline = StagePipeline::new(options.stop_on_first_error);

    // Stage: Objects. No upstream stage dependency. `trust_verifier` is mutated by reference and its
    // state survives a `Failed` outcome here, since it lives in this function's own frame rather than
    // inside `verify_objects` -- reused safely by RefUpdateSchemaTrust and PublicationReclassification
    // below (DC-95 Stage 2 Step 0 §3: `PublicationTrustVerifier` cannot manufacture a false "trusted"
    // result from partial evaluation).
    let object_summary = pipeline.run(
        VerificationStage::Objects,
        verify_objects(layout, &object_store, &mut trust_verifier),
    );
    let objects_evaluated = object_summary.is_some();
    let (
        object_outcomes,
        block_state_outcomes,
        checked_objects,
        checked_blocks,
        checked_rollback_blocks,
        checked_sealed_rollback_patches,
        object_temp_paths,
        merge_baseline_divergences,
        block_seals,
        mut signature_envelope_issues,
    ) = match object_summary {
        Some(summary) => {
            let counts = phase_a_counts(&summary.item_outcomes)?;
            (
                summary.item_outcomes,
                summary.topological_outcomes,
                Some(counts.objects),
                Some(counts.blocks),
                Some(counts.rollback_blocks),
                Some(counts.rollback_patches),
                summary.temp_paths,
                summary.merge_baseline_divergences,
                summary.block_seals,
                summary.signature_issues,
            )
        }
        None => (
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
    };

    // Stage: Refs. No upstream stage dependency.
    let ref_verification = pipeline.run(VerificationStage::Refs, verify_refs(layout));

    // Stage: RefUpdateSchemaTrust. Depends on Refs for the envelope list.
    let ref_update_schema_trust_evaluated = if let Some(rv) = &ref_verification {
        pipeline
            .run(
                VerificationStage::RefUpdateSchemaTrust,
                (|| -> Result<()> {
                    for envelope in &rv.ref_update_envelopes {
                        crate::format::validate_read_schema(layout.format(), envelope)?;
                        trust_verifier.verify(envelope)?;
                    }
                    Ok(())
                })(),
            )
            .is_some()
    } else {
        pipeline.not_evaluated(
            VerificationStage::RefUpdateSchemaTrust,
            VerificationStage::Refs,
        );
        false
    };

    let refs_evaluated = ref_verification.is_some();
    let (
        checked_refs,
        checked_ref_log_records,
        mut ref_publication_issues,
        refs_signature_envelope_issues,
        pointer_outcomes,
        log_outcomes,
        ref_item_outcomes,
    ) = match ref_verification {
        Some(rv) => (
            Some(rv.pointer_count),
            Some(rv.log_record_count),
            rv.publication_issues,
            rv.signature_envelope_issues,
            rv.pointer_outcomes,
            rv.log_outcomes,
            rv.ref_item_outcomes,
        ),
        None => (
            None,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
    };

    // Stage: WalReplay. No upstream stage dependency.
    let wal = Wal::for_layout(layout);
    let replay = pipeline.run(VerificationStage::WalReplay, wal.replay());

    // Stage: WalPersistence. Depends on WalReplay.
    let persisted_wal_patches = if let Some(replay) = &replay {
        pipeline.run(
            VerificationStage::WalPersistence,
            verify_wal_persistence(&object_store, &replay.records),
        )
    } else {
        pipeline.not_evaluated(
            VerificationStage::WalPersistence,
            VerificationStage::WalReplay,
        );
        None
    };

    // Stage: RollbackDrafts. Depends on WalReplay.
    let checked_rollback_draft_records = if let Some(replay) = &replay {
        pipeline.run(
            VerificationStage::RollbackDrafts,
            verify_rollback_draft_wal_records(&replay.records),
        )
    } else {
        pipeline.not_evaluated(
            VerificationStage::RollbackDrafts,
            VerificationStage::WalReplay,
        );
        None
    };

    // Stage: WalRecordSchema. Depends on WalReplay.
    if let Some(replay) = &replay {
        pipeline.run(
            VerificationStage::WalRecordSchema,
            (|| -> Result<()> {
                for record in &replay.records {
                    crate::format::validate_read_schema(layout.format(), &record.envelope)?;
                    signature_envelope_issues.extend(classify_signature_envelope(
                        &record.envelope,
                        SignatureEnvelopeSource::ActiveWal {
                            sequence: record.seq,
                            object_id: record.envelope.object_id(),
                        },
                    )?);
                }
                Ok(())
            })(),
        );
        // Matches the pre-Level-1 merge order (Objects, then WAL, then Refs) -- deferred until here,
        // after the WAL per-record loop's own contributions, rather than appended immediately after
        // the Refs stage above, purely to preserve that existing, asserted-on order.
        signature_envelope_issues.extend(refs_signature_envelope_issues);
    } else {
        pipeline.not_evaluated(
            VerificationStage::WalRecordSchema,
            VerificationStage::WalReplay,
        );
        signature_envelope_issues.extend(refs_signature_envelope_issues);
    }

    // Stage: ActiveWalMetadata. Depends on WalReplay.
    let active_wal_metadata_status = if let Some(replay) = &replay {
        pipeline.run(
            VerificationStage::ActiveWalMetadata,
            classify_active_wal_metadata(layout, replay.records.is_empty()),
        )
    } else {
        pipeline.not_evaluated(
            VerificationStage::ActiveWalMetadata,
            VerificationStage::WalReplay,
        );
        None
    };

    // Stage: PublicationReclassification. Cannot run at all without Refs (needs `issues` to mutate),
    // WalReplay (needs `records`), or ActiveWalMetadata (needs `metadata`) -- `NotEvaluated`, naming
    // whichever of those three failed first, if any. Objects failing does *not* block this stage from
    // running: it only degrades `trust_is_valid` to a safe `false` (DC-95 Stage 2 Step 0 ruling §2-§3
    // -- an accumulator's emptiness means "none found" only if its producer ran to completion; reading
    // `trust_verifier.issues.is_empty()` alone would silently claim "proved" from an unrun check).
    match (&replay, refs_evaluated, &active_wal_metadata_status) {
        (Some(replay), true, Some(metadata)) => {
            let trust_is_valid = objects_evaluated && trust_verifier.issues.is_empty();
            pipeline.run(
                VerificationStage::PublicationReclassification,
                ref_publication::require_retained_evidence(
                    layout,
                    &replay.records,
                    metadata,
                    trust_is_valid,
                    &mut ref_publication_issues,
                ),
            );
        }
        _ => {
            let blocked_by = if replay.is_none() {
                VerificationStage::WalReplay
            } else if !refs_evaluated {
                VerificationStage::Refs
            } else {
                VerificationStage::ActiveWalMetadata
            };
            pipeline.not_evaluated(VerificationStage::PublicationReclassification, blocked_by);
        }
    }

    // Stage: CommitIndex. No upstream stage dependency; contained like any other fallible stage --
    // `commit_index::verify_divergence` does return `Result`, unlike `LifecycleCache`'s. An empty
    // `Vec` on `Failed`/`NotEvaluated` is safe here (unlike a count): the stage's own outcome above
    // already says whether "no divergences" was actually established.
    let commit_index_divergences = pipeline
        .run(VerificationStage::CommitIndex, verify_divergence(layout))
        .unwrap_or_default();

    // Stage: LifecycleCache. No upstream stage dependency; cannot fail by construction -- a replay
    // error is itself converted into a divergence entry rather than propagated (DC-95 Stage 1 round
    // 12). Still subject to an earlier halt under `--stop-on-first-error`.
    let lifecycle_cache_divergences = if pipeline.run_infallible(VerificationStage::LifecycleCache)
    {
        verify_lifecycle_cache_divergence(&object_store, layout)
    } else {
        Vec::new()
    };

    // Stage: WalOrdering. Depends on WalReplay; the check itself cannot fail (a pure function).
    let active_wal_ordering_issues = if let Some(replay) = &replay {
        if pipeline.run_infallible(VerificationStage::WalOrdering) {
            check_active_wal_ordering(&replay.records)
        } else {
            Vec::new()
        }
    } else {
        pipeline.not_evaluated(VerificationStage::WalOrdering, VerificationStage::WalReplay);
        Vec::new()
    };

    let checked_publication_trust_records = (objects_evaluated
        && ref_update_schema_trust_evaluated)
        .then_some(trust_verifier.checked_records);

    Ok(RepositoryVerification {
        legacy_state_roots_unverifiable: layout.format() == RepositoryFormat::LegacyV1,
        stage_outcomes: pipeline.outcomes,
        object_outcomes,
        block_state_outcomes,
        checked_objects,
        checked_wal_records: replay.as_ref().map(|replay| replay.records.len()),
        checked_blocks,
        checked_rollback_blocks,
        checked_sealed_rollback_patches,
        persisted_wal_patches,
        checked_refs,
        checked_ref_log_records,
        ref_publication_issues,
        pointer_outcomes,
        log_outcomes,
        ref_item_outcomes,
        signature_envelope_issues,
        checked_rollback_draft_records,
        checked_publication_trust_records,
        publication_trust_issues: trust_verifier.issues,
        object_temp_paths,
        trailing_partial_wal_bytes: replay.as_ref().map(|replay| replay.trailing_partial_bytes),
        active_wal_metadata_status,
        commit_index_divergences,
        lifecycle_cache_divergences,
        active_wal_ordering_issues,
        merge_baseline_divergences,
        block_seals,
    })
}

/// Aggregate counts derived from Phase A's per-item outcomes (DC-95 Stage 2 Level 2). Each field
/// counts only `Evaluated` entries -- a `Failed` object contributes to none of them, same as it never
/// contributed to the pre-Level-2 running totals a whole-stage failure would have zeroed out entirely.
struct PhaseACounts {
    objects: usize,
    blocks: usize,
    rollback_blocks: usize,
    rollback_patches: usize,
}

fn phase_a_counts(object_outcomes: &[ObjectItemOutcome]) -> Result<PhaseACounts> {
    let mut objects = 0_usize;
    let mut blocks = 0_usize;
    let mut rollback_blocks = 0_usize;
    let mut rollback_patches = 0_usize;
    for outcome in object_outcomes {
        let ObjectItemStatus::Evaluated(verification) = &outcome.status else {
            continue;
        };
        objects = objects.checked_add(1).ok_or_else(|| {
            PrikkError::Integrity("object verification count overflow".to_string())
        })?;
        if verification.object_type == ObjectType::Block {
            blocks = blocks.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("block verification count overflow".to_string())
            })?;
            if verification.rollback_patch_count != 0 {
                rollback_blocks = rollback_blocks.checked_add(1).ok_or_else(|| {
                    PrikkError::Integrity("rollback block count overflow".to_string())
                })?;
                rollback_patches = rollback_patches
                    .checked_add(verification.rollback_patch_count)
                    .ok_or_else(|| {
                        PrikkError::Integrity("rollback patch count overflow".to_string())
                    })?;
            }
        }
    }
    Ok(PhaseACounts {
        objects,
        blocks,
        rollback_blocks,
        rollback_patches,
    })
}

/// Check that active WAL record sequences strictly increase in replay (append) order. Reachable only
/// under direct file tampering — `Wal::append_patch` always assigns `previous.seq + 1` — but a queue
/// of N gives "ordering" its own meaning worth verifying explicitly (RFC criterion 6), not merely
/// assumed from successful structural decode.
fn check_active_wal_ordering(records: &[crate::wal::WalRecord]) -> Vec<ActiveWalOrderingIssue> {
    records
        .iter()
        .zip(records.iter().skip(1))
        .enumerate()
        .filter(|(_, (previous, current))| current.seq <= previous.seq)
        .map(|(index, (previous, current))| ActiveWalOrderingIssue {
            index: index + 1,
            previous_seq: previous.seq,
            seq: current.seq,
        })
        .collect()
}

fn classify_active_wal_metadata(
    layout: &RepositoryLayout,
    wal_is_empty: bool,
) -> Result<ActiveWalMetadataStatus> {
    match (wal_is_empty, read_active_ref_metadata(layout)?) {
        (true, ActiveRefMetadata::Missing) => Ok(ActiveWalMetadataStatus::MissingForEmptyWal),
        (true, ActiveRefMetadata::Valid(ref_name)) => {
            Ok(ActiveWalMetadataStatus::ValidForEmptyWal { ref_name })
        }
        (true, ActiveRefMetadata::Invalid(reason)) => {
            Ok(ActiveWalMetadataStatus::InvalidForEmptyWal { reason })
        }
        (false, ActiveRefMetadata::Missing) => Ok(ActiveWalMetadataStatus::MissingForNonEmptyWal),
        (false, ActiveRefMetadata::Valid(ref_name)) => {
            Ok(ActiveWalMetadataStatus::ValidForNonEmptyWal { ref_name })
        }
        (false, ActiveRefMetadata::Invalid(reason)) => {
            Ok(ActiveWalMetadataStatus::InvalidForNonEmptyWal { reason })
        }
    }
}

/// Phase A (DC-92 §4.2): every check that does not depend on lineage-state derivation order —
/// existence of referenced objects, rollback-patch counting, and (independent of the shared memo)
/// the merge-baseline re-derivation. A `CurrentV2` block's own state-root verification is
/// deliberately **not** done here; it is deferred to a batch, dependency-ordered pass
/// (`crate::block_state::verify_blocks_topological`) run once after every object type has been
/// scanned, so `pending_v2_blocks` collects this block's already-decoded payload rather than
/// discarding it. See that function's own doc for why deferring is what bounds
/// `LineageStateMemo`'s memory instead of merely avoiding redundant re-derivation.
fn verify_block_payload(
    object_store: &FileObjectStore,
    block_id: ObjectId,
    format: RepositoryFormat,
    canonical_payload: &[u8],
    pending_v2_blocks: &mut Vec<(ObjectId, BlockPayload)>,
) -> Result<(usize, Option<MergeBaselineDivergence>)> {
    let payload = BlockPayload::decode_canonical(canonical_payload)?;
    for parent in &payload.parent_block_ids {
        ensure_object_exists(
            object_store,
            ObjectType::Block,
            *parent,
            "parent block",
            block_id,
        )?;
    }
    let mut rollback_patch_count = 0_usize;
    for patch in &payload.patch_ids {
        let Some(envelope) = object_store.read_typed(*patch, ObjectType::Patch)? else {
            return Err(PrikkError::Integrity(format!(
                "object {block_id} references missing block patch {patch}"
            )));
        };
        let context = format!("sealed Block {block_id} Patch {patch}");
        if verify_rollback_patch_envelope(&envelope, &context)? {
            rollback_patch_count = rollback_patch_count.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("sealed rollback patch count overflow".to_string())
            })?;
        }
    }
    if let Some(snapshot) = payload.snapshot_blob_ref {
        ensure_object_exists(
            object_store,
            ObjectType::Blob,
            snapshot,
            "snapshot blob",
            block_id,
        )?;
    }
    let merge_baseline_divergence = if format == RepositoryFormat::CurrentV2 {
        verify_merge_baseline(object_store, block_id, &payload)?
    } else {
        None
    };
    if format == RepositoryFormat::CurrentV2 {
        pending_v2_blocks.push((block_id, payload));
    }
    Ok((rollback_patch_count, merge_baseline_divergence))
}

/// DC-75: for a `Merge` block, independently re-derive whether the recorded
/// `merge_baseline_block_id` is a common ancestor of both parents — a claim, not trusted. Shape
/// (kind, parent count, mainline/baseline presence) is already guaranteed by
/// `verify_block_v2_state`'s `validate_block_v2_shape` call above, so this only checks the claim's
/// content. Cost is the same full-parent reachability walk measured linear in
/// `baseline-recording-answer-v1.md` §1 — unconditional, not a gated "deep verify" mode.
fn verify_merge_baseline(
    object_store: &FileObjectStore,
    block_id: ObjectId,
    payload: &BlockPayload,
) -> Result<Option<MergeBaselineDivergence>> {
    if payload.kind != prikk_object::BlockKind::Merge {
        return Ok(None);
    }
    let (Some(mainline_parent_id), Some(recorded_baseline)) =
        (payload.mainline_parent_id, payload.merge_baseline_block_id)
    else {
        // Malformed shape already failed closed above via `validate_block_v2_shape`.
        return Ok(None);
    };
    let Some(&secondary_parent_id) = payload
        .parent_block_ids
        .iter()
        .find(|&&id| id != mainline_parent_id)
    else {
        return Ok(None);
    };
    let mainline_ancestors =
        crate::merge_evidence::ancestors_inclusive(object_store, mainline_parent_id)?;
    let secondary_ancestors =
        crate::merge_evidence::ancestors_inclusive(object_store, secondary_parent_id)?;
    let is_common_ancestor = mainline_ancestors.contains_key(&recorded_baseline)
        && secondary_ancestors.contains_key(&recorded_baseline);
    if is_common_ancestor {
        Ok(None)
    } else {
        Ok(Some(MergeBaselineDivergence {
            block_id,
            recorded_baseline,
            mainline_parent_id,
            secondary_parent_id,
        }))
    }
}

fn ensure_object_exists(
    object_store: &FileObjectStore,
    object_type: ObjectType,
    object_id: ObjectId,
    role: &str,
    owner: ObjectId,
) -> Result<()> {
    let exists = object_store.read_typed(object_id, object_type)?.is_some();
    if exists {
        return Ok(());
    }
    Err(PrikkError::Integrity(format!(
        "object {owner} references missing {role} {object_id}"
    )))
}

fn verify_wal_persistence(
    object_store: &FileObjectStore,
    records: &[crate::WalRecord],
) -> Result<usize> {
    let mut persisted = 0_usize;
    for record in records {
        if record.envelope.object_type != ObjectType::Patch {
            return Err(PrikkError::Integrity(format!(
                "active WAL record {} contains {}, expected patch",
                record.seq, record.envelope.object_type
            )));
        }
        if object_store.contains_object(ObjectType::Patch, record.envelope.object_id()) {
            persisted = persisted.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("persisted WAL patch count overflow".to_string())
            })?;
        }
    }
    Ok(persisted)
}

#[cfg(test)]
mod tests;
