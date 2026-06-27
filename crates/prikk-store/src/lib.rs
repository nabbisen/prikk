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
mod path;
mod patch_checkout;
mod patch_inverse;
mod patch_replay;
mod refs;
mod rollback_draft;
mod rollback_preview;
mod rollback_verify;
mod snapshot;
mod wal;
mod verify;
mod worktree;
mod worktree_patch;
mod worktree_status;

#[cfg(test)]
mod tests;

pub use active::{ActiveCommitResult, ActiveSession};
pub use checkout::{
    prepare_checkout_plan, prepare_snapshot_checkout_plan, CheckoutMaterialization, CheckoutPlan,
    SnapshotCheckoutPlan, DEFAULT_CHECKOUT_REF,
};
pub use doctor::{
    doctor_repository, repair_repository, DoctorIssue, DoctorRepairOptions, DoctorRepairReport,
    DoctorReport, DoctorSeverity,
};
pub use history::{DEFAULT_HISTORY_LIMIT, HistoryEntry, RefHistory, load_ref_history};
pub use layout::RepositoryLayout;
pub use lock::{ActiveLock, RefLock};
pub use memory_store::MemoryObjectStore;
pub use object_store::{FileObjectStore, ObjectReader, ObjectWriter};
pub use path::{validate_no_path_collisions, validate_repo_path, RepoPath};
pub use patch_checkout::{
    materialize_patch_checkout, materialize_patch_checkout_with_deletions,
    plan_patch_checkout_deletions, PatchDeletionConflict, PatchDeletionPlan,
    PatchMaterializationReport,
};
pub use patch_inverse::{
    prepare_patch_inverse_plan, PatchInverseOperationKind, PatchInverseOperationSummary,
    PatchInversePlan,
};
pub use patch_replay::{prepare_patch_replay_plan, PatchReplayPlan};
pub use rollback_draft::{append_rollback_draft, RollbackDraftReport};
pub use refs::{
    RefLogReplay, RefLogRecord, RefPublication, RefRecoveryCandidate, RefRecoveryRepair, RefStore,
};
pub use rollback_preview::{
    prepare_rollback_preview, RollbackPreviewChange, RollbackPreviewChangeKind,
    RollbackPreviewPlan,
};
pub use rollback_verify::{verify_active_rollback_draft, RollbackDraftVerification};
pub use snapshot::{SnapshotEntry, SnapshotManifest};
pub use wal::{Wal, WalRecord, WalReplay, WalRepair};
pub use verify::{verify_repository, ObjectVerification, RepositoryVerification};
pub use worktree::{materialize_snapshot_checkout, SnapshotMaterializationReport};
pub use worktree_status::{
    worktree_status, WorktreeChange, WorktreeChangeKind, WorktreeStatusReport,
};
pub use worktree_patch::{
    commit_worktree_changes, commit_worktree_changes_with_options, WorktreePatchCommitOptions,
    WorktreePatchCommitReport, WorktreePatchOperationKind, WorktreePatchOperationSummary,
};
