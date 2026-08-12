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

/// Repository verification summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryVerification {
    /// True when format-1 scaffold roots cannot be verified as clean-state commitments.
    pub legacy_state_roots_unverifiable: bool,
    /// Number of persisted object files checked successfully.
    pub checked_objects: usize,
    /// Number of active WAL records replayed successfully.
    pub checked_wal_records: usize,
    /// Number of persisted block objects whose references were checked.
    pub checked_blocks: usize,
    /// Number of persisted Block objects classified as rollback blocks.
    pub checked_rollback_blocks: usize,
    /// Number of sealed rollback-marked Patch objects referenced by verified Blocks.
    pub checked_sealed_rollback_patches: usize,
    /// Number of active WAL patch records that already exist as persisted patch objects.
    pub persisted_wal_patches: usize,
    /// Number of ref pointer files checked successfully.
    pub checked_refs: usize,
    /// Number of inline ref-log records checked successfully.
    pub checked_ref_log_records: usize,
    /// Interrupted ref-publication and candidate-debris conditions found by joint verification.
    pub ref_publication_issues: Vec<crate::refs::RefPublicationIssue>,
    /// Warning-level format-1 signature-envelope compatibility findings in deterministic order.
    pub signature_envelope_issues: Vec<SignatureEnvelopeIssue>,
    /// Number of active WAL records classified and decoded as rollback drafts.
    pub checked_rollback_draft_records: usize,
    /// Number of publication envelopes checked against repository-local trust.
    pub checked_publication_trust_records: usize,
    /// Publication-trust issues found while structural verification succeeded.
    pub publication_trust_issues: Vec<PublicationTrustIssue>,
    /// Recognized non-authoritative object publication temps left for explicit maintenance.
    pub object_temp_paths: Vec<PathBuf>,
    /// Number of trailing bytes in the active WAL that look like an incomplete final record.
    pub trailing_partial_wal_bytes: usize,
    /// Active-WAL ref metadata status relative to the replayed WAL.
    pub active_wal_metadata_status: ActiveWalMetadataStatus,
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
    /// Return true when legacy scaffold roots prevent state-commitment verification.
    #[must_use]
    pub const fn has_unverifiable_state_roots(&self) -> bool {
        self.legacy_state_roots_unverifiable
    }

    /// Return true if the active WAL contained an incomplete trailing record.
    #[must_use]
    pub const fn has_trailing_partial_wal(&self) -> bool {
        self.trailing_partial_wal_bytes != 0
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

    /// Return true when a non-empty active WAL lacks valid ownership metadata.
    #[must_use]
    pub const fn has_active_wal_metadata_integrity_issue(&self) -> bool {
        self.active_wal_metadata_status.has_integrity_issue()
    }

    /// Return true when an empty active WAL has stale local metadata debris.
    #[must_use]
    pub const fn has_active_wal_metadata_warning(&self) -> bool {
        self.active_wal_metadata_status.has_local_debris_warning()
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

/// Verify a repository layout without modifying it.
pub fn verify_repository(layout: &RepositoryLayout) -> Result<RepositoryVerification> {
    let object_store = FileObjectStore::new(layout.clone());
    let mut trust_verifier = PublicationTrustVerifier::new(layout);
    let object_summary = verify_objects(layout, &object_store, &mut trust_verifier)?;
    let ref_verification = verify_refs(layout)?;
    for envelope in &ref_verification.ref_update_envelopes {
        crate::format::validate_read_schema(layout.format(), envelope)?;
        trust_verifier.verify(envelope)?;
    }
    let wal = Wal::for_layout(layout);
    let replay = wal.replay()?;
    let persisted_wal_patches = verify_wal_persistence(&object_store, &replay.records)?;
    let checked_rollback_draft_records = verify_rollback_draft_wal_records(&replay.records)?;
    let mut signature_envelope_issues = object_summary.signature_issues;
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
    signature_envelope_issues.extend(ref_verification.signature_envelope_issues);
    let active_wal_metadata_status =
        classify_active_wal_metadata(layout, replay.records.is_empty())?;
    let mut ref_publication_issues = ref_verification.publication_issues;
    ref_publication::require_retained_evidence(
        layout,
        &replay.records,
        &active_wal_metadata_status,
        trust_verifier.issues.is_empty(),
        &mut ref_publication_issues,
    )?;
    let commit_index_divergences = verify_divergence(layout)?;
    let lifecycle_cache_divergences = verify_lifecycle_cache_divergence(&object_store, layout);
    let active_wal_ordering_issues = check_active_wal_ordering(&replay.records);
    let merge_baseline_divergences = object_summary.merge_baseline_divergences;
    Ok(RepositoryVerification {
        legacy_state_roots_unverifiable: layout.format() == RepositoryFormat::LegacyV1,
        checked_objects: object_summary.object_count,
        checked_wal_records: replay.records.len(),
        checked_blocks: object_summary.block_count,
        checked_rollback_blocks: object_summary.rollback_block_count,
        checked_sealed_rollback_patches: object_summary.rollback_patch_count,
        persisted_wal_patches,
        checked_refs: ref_verification.pointer_count,
        checked_ref_log_records: ref_verification.log_record_count,
        ref_publication_issues,
        signature_envelope_issues,
        checked_rollback_draft_records,
        checked_publication_trust_records: trust_verifier.checked_records,
        publication_trust_issues: trust_verifier.issues,
        object_temp_paths: object_summary.temp_paths,
        trailing_partial_wal_bytes: replay.trailing_partial_bytes,
        active_wal_metadata_status,
        commit_index_divergences,
        lifecycle_cache_divergences,
        active_wal_ordering_issues,
        merge_baseline_divergences,
        block_seals: object_summary.block_seals,
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
