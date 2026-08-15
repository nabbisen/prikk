#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate for Prikk repositories.
//!
//! This crate provides persistent layout, object storage, WAL durability, deeper read-only
//! repository verification, initial ref-state/ref-log publication primitives, a narrow
//! active-session append API, opt-in safe doctor repairs, conservative snapshot materialization,
//! read-only worktree status, minimal worktree-to-patch draft generation, supported patch replay
//! planning and materialization, explicit opt-in deletion of patch-removed files, deterministic
//! arbitrary-span text edit replay and generation from worktree changes, explicit unborn local branch
//! genesis through active-WAL ref ownership,
//! read-only inverse planning for the supported patch subset, non-mutating rollback preview,
//! conservative rollback draft append to an empty active WAL, rollback draft verification, sealed
//! rollback block classification, and an internal patch-algebra foundation. Production confluence,
//! plugin execution, and remote sync remain separate increments.

mod active;
mod author_signing;
mod blob_access;
mod block_state;
mod bundle;
mod byte_cursor;
mod checkout;
mod commit_index;
mod container;
mod doctor;
mod file_codec;
mod format;
mod frame_resync;
mod fsutil;
mod history;
mod index;
mod layout;
mod lifecycle_cache;
mod lock;
mod maintainer_signing;
mod memory_store;
mod merge_evidence;
mod merge_execute;
// Production node-id minting (DC-09 Phase 4.4a-1), consumed by node-addressed worktree authoring
// (4.4a-2) for fresh-node creation.
mod node_id_gen;
mod node_lifecycle;
mod object_store;
// Patch algebra foundation and evidence contract (DC-16/DC-21), now production-compiled through the
// DC-22 read-only merge-evidence store boundary.
mod patch_algebra;
mod patch_checkout;
mod patch_inverse;
mod patch_replay;
mod path;
mod received;
mod received_index;
mod refs;
mod rollback_draft;
mod rollback_preview;
mod rollback_verify;
mod signature_diagnostics;
mod snapshot;
mod state_root;
mod text_span;
mod trust;
mod trust_index;
mod verify;
mod wal;
mod worktree;
mod worktree_marker;
mod worktree_patch;
mod worktree_status;

#[cfg(test)]
mod dc55_identity_evidence;
#[cfg(test)]
mod signature_contract_tests;
#[cfg(test)]
mod test_support;

pub use active::{
    ActiveCommitResult, ActiveRefMetadata, ActiveSession, finish_active_publication_cleanup,
    read_active_ref_metadata, remove_active_ref_metadata, require_active_ref_for_non_empty_wal,
    write_active_ref_metadata,
};
pub use author_signing::{AuthorSigner, Ed25519AuthorSigner, author_signature};
pub use block_state::{
    BlockStateOutcome, BlockStateStatus, derive_next_state_root, validate_block_v2_shape,
};
pub use bundle::{
    BundleExportReport, BundleImportOptions, BundleImportReport, DEFAULT_BUNDLE_MAX_OBJECT_COUNT,
    DEFAULT_BUNDLE_MAX_TOTAL_BYTES, export_bundle, import_bundle,
};
pub use checkout::{
    CheckoutMaterialization, CheckoutPlan, DEFAULT_CHECKOUT_REF, SnapshotCheckoutPlan,
    prepare_checkout_plan, prepare_snapshot_checkout_plan,
};
pub use commit_index::CommitIndexDivergence;
pub use doctor::{
    DoctorIssue, DoctorRepairOptions, DoctorRepairReport, DoctorReport, DoctorSeverity,
    doctor_repository, repair_repository,
};
pub use history::{
    DEFAULT_HISTORY_LIMIT, HistoryEntry, RefHistory, load_received_ref_history, load_ref_history,
};
pub use layout::{ContainerSlot, RepositoryFormat, RepositoryLayout};
pub use lifecycle_cache::incremental::LifecycleCacheDivergence;
pub use lock::{ActiveLock, RefLock};
pub use maintainer_signing::{Ed25519MaintainerSigner, MaintainerSigner, maintainer_signature};
pub use memory_store::MemoryObjectStore;
pub use merge_evidence::{
    MergeEvidenceDisplay, MergeEvidenceDisplayItem, MergeEvidenceDisplayOperation,
    MergeEvidenceDisplaySelector, MergeEvidenceTarget, MergePlanDisplay, prepare_merge_evidence,
    prepare_merge_plan,
};
pub use merge_execute::{MergeExecutionReport, execute_merge};
pub use object_store::{FileObjectStore, ObjectReader, ObjectWriter};
pub use patch_checkout::{
    PatchDeletionConflict, PatchDeletionPlan, PatchMaterializationReport,
    materialize_patch_checkout, materialize_patch_checkout_with_deletions,
    plan_patch_checkout_deletions,
};
pub use patch_inverse::{
    PatchInverseOperationKind, PatchInverseOperationSummary, PatchInversePlan,
    prepare_patch_inverse_plan,
};
pub use patch_replay::{PatchReplayPlan, prepare_patch_replay_plan};
pub use path::{RepoPath, validate_no_path_collisions, validate_repo_path};
pub use received::{
    ReceivedPointer, list_received_pointers, read_received_pointer, validate_received_ref,
};
pub use refs::{
    RefFileOutcome, RefFileStatus, RefItemOutcome, RefItemStatus, RefLogRecord, RefLogReplay,
    RefPointerSummary, RefPublication, RefPublicationIssue, RefRecoveryCandidate,
    RefRecoveryRepair, RefStore, validate_local_branch_ref, validate_local_tag_ref,
};
#[cfg(feature = "test-support")]
pub use refs::{
    force_ref_pointer_to_arbitrary_state_for_test_support,
    remove_ref_pointer_entry_for_test_support,
};
pub use rollback_draft::{RollbackDraftReport, append_rollback_draft};
pub use rollback_preview::{
    RollbackPreviewChange, RollbackPreviewChangeKind, RollbackPreviewPlan, prepare_rollback_preview,
};
pub use rollback_verify::{RollbackDraftVerification, verify_active_rollback_draft};
pub use signature_diagnostics::{SignatureEnvelopeIssue, SignatureEnvelopeSource};
pub use snapshot::{SnapshotEntry, SnapshotManifest};
pub use state_root::{
    StateRootContent, StateRootEntry, compute_state_root, state_leaf_hash, state_leaf_preimage,
};
pub use trust::{
    AdoptedMaintainerKey, MaintainerTrustPolicy, PublicationTrustIssue, add_trusted_maintainer,
    load_maintainer_trust_policy, remove_trusted_maintainer, verify_signer_trusted,
    verify_trusted_publication_envelope,
};
pub use verify::{
    ActiveWalMetadataStatus, ActiveWalOrderingIssue, BlockSealVerification, ObjectItemOutcome,
    ObjectItemStatus, ObjectVerification, RepositoryVerification, StageOutcome, StageStatus,
    VerificationStage, VerifyOptions, verify_repository, verify_repository_with_options,
};
pub use wal::{Wal, WalRecord, WalRepair, WalReplay};
pub use worktree::{SnapshotMaterializationReport, materialize_snapshot_checkout};
pub use worktree_patch::{
    DEFAULT_ACTIVE_PATCH_LIMIT, WorktreePatchCommitOptions, WorktreePatchCommitReport,
    WorktreePatchOperationKind, WorktreePatchOperationSummary, commit_worktree_changes_signed,
};
pub use worktree_status::{
    WorktreeChange, WorktreeChangeKind, WorktreeStatusReport, worktree_status,
};
