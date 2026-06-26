#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate for PRIKK repositories.
//!
//! PR-015 contains persistent layout, object storage, WAL durability, deeper read-only
//! repository verification, initial ref-state/ref-log publication primitives, a narrow
//! active-session append API, and opt-in safe doctor repairs for incomplete WAL tails and missing
//! ref pointers. Patch algebra, plugin execution, and remote sync remain separate increments.

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
mod refs;
mod wal;
mod verify;

#[cfg(test)]
mod tests;

pub use active::{ActiveCommitResult, ActiveSession};
pub use checkout::{
    prepare_checkout_plan, CheckoutMaterialization, CheckoutPlan, DEFAULT_CHECKOUT_REF,
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
pub use refs::{
    RefLogReplay, RefLogRecord, RefPublication, RefRecoveryCandidate, RefRecoveryRepair, RefStore,
};
pub use wal::{Wal, WalRecord, WalReplay, WalRepair};
pub use verify::{verify_repository, ObjectVerification, RepositoryVerification};
