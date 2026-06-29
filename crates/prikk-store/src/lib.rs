#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate for Prikk repositories.
//!
//! PR-030 contains persistent layout, object storage, WAL durability, deeper read-only
//! repository verification, initial ref-state/ref-log publication primitives, a narrow
//! active-session append API, opt-in safe doctor repairs, conservative snapshot materialization,
//! read-only worktree status, minimal worktree-to-patch draft generation, supported patch replay
//! planning and materialization, explicit opt-in deletion of patch-removed files, conservative
//! full-file text edit replay, opt-in full-file text edit generation from worktree changes, and
//! read-only inverse planning for the supported patch subset, non-mutating rollback preview,
//! conservative rollback draft append to an empty active WAL, rollback draft verification, and sealed rollback block classification.
//! Full patch algebra, plugin execution, and remote sync remain separate increments.

mod active;
mod blob_access;
mod byte_cursor;
mod checkout;
mod doctor;
mod file_codec;
mod fsutil;
mod history;
mod layout;
mod lock;
mod memory_store;
mod object_store;
mod patch_checkout;
mod patch_inverse;
mod patch_replay;
mod path;
mod refs;
mod rollback_draft;
mod rollback_preview;
mod rollback_verify;
mod snapshot;
mod verify;
mod wal;
mod worktree;
mod worktree_patch;
mod worktree_status;

#[cfg(test)]
mod test_support;

pub use active::{ActiveCommitResult, ActiveSession};
pub use checkout::{
    CheckoutMaterialization, CheckoutPlan, DEFAULT_CHECKOUT_REF, SnapshotCheckoutPlan,
    prepare_checkout_plan, prepare_snapshot_checkout_plan,
};
pub use doctor::{
    DoctorIssue, DoctorRepairOptions, DoctorRepairReport, DoctorReport, DoctorSeverity,
    doctor_repository, repair_repository,
};
pub use history::{DEFAULT_HISTORY_LIMIT, HistoryEntry, RefHistory, load_ref_history};
pub use layout::RepositoryLayout;
pub use lock::{ActiveLock, RefLock};
pub use memory_store::MemoryObjectStore;
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
pub use refs::{
    RefLogRecord, RefLogReplay, RefPublication, RefRecoveryCandidate, RefRecoveryRepair, RefStore,
};
pub use rollback_draft::{RollbackDraftReport, append_rollback_draft};
pub use rollback_preview::{
    RollbackPreviewChange, RollbackPreviewChangeKind, RollbackPreviewPlan, prepare_rollback_preview,
};
pub use rollback_verify::{RollbackDraftVerification, verify_active_rollback_draft};
pub use snapshot::{SnapshotEntry, SnapshotManifest};
pub use verify::{ObjectVerification, RepositoryVerification, verify_repository};
pub use wal::{Wal, WalRecord, WalRepair, WalReplay};
pub use worktree::{SnapshotMaterializationReport, materialize_snapshot_checkout};
pub use worktree_patch::{
    WorktreePatchCommitOptions, WorktreePatchCommitReport, WorktreePatchOperationKind,
    WorktreePatchOperationSummary, commit_worktree_changes, commit_worktree_changes_with_options,
};
pub use worktree_status::{
    WorktreeChange, WorktreeChangeKind, WorktreeStatusReport, worktree_status,
};
