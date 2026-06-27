#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate for PRIKK repositories.
//!
//! PR-022 contains persistent layout, object storage, WAL durability, deeper read-only
//! repository verification, initial ref-state/ref-log publication primitives, a narrow
//! active-session append API, opt-in safe doctor repairs, conservative snapshot materialization,
//! read-only worktree status, minimal worktree-to-patch draft generation, and supported file-level
//! patch replay planning, conservative patch replay materialization, and explicit opt-in
//! deletion of patch-removed files. Patch algebra,
//! plugin execution, and remote sync remain separate increments.

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
mod patch_replay;
mod refs;
mod snapshot;
mod wal;
mod verify;
mod worktree;
mod worktree_status;
mod worktree_patch;

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
pub use patch_replay::{prepare_patch_replay_plan, PatchReplayPlan};
pub use refs::{
    RefLogReplay, RefLogRecord, RefPublication, RefRecoveryCandidate, RefRecoveryRepair, RefStore,
};
pub use snapshot::{SnapshotEntry, SnapshotManifest};
pub use wal::{Wal, WalRecord, WalReplay, WalRepair};
pub use verify::{verify_repository, ObjectVerification, RepositoryVerification};
pub use worktree::{materialize_snapshot_checkout, SnapshotMaterializationReport};
pub use worktree_status::{
    worktree_status, WorktreeChange, WorktreeChangeKind, WorktreeStatusReport,
};
pub use worktree_patch::{
    commit_worktree_changes, WorktreePatchCommitReport, WorktreePatchOperationKind,
    WorktreePatchOperationSummary,
};
