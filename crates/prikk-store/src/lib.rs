#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate for PRIKK repositories.
//!
//! PR-008 contains persistent layout, object storage, WAL durability, read-only
//! repository verification, initial ref-state/ref-log publication primitives, and a narrow
//! active-session append API. Patch algebra, plugin execution, and remote sync remain separate
//! increments.

mod active;
mod byte_cursor;
mod file_codec;
mod fsutil;
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
pub use layout::RepositoryLayout;
pub use lock::{ActiveLock, RefLock};
pub use memory_store::MemoryObjectStore;
pub use object_store::{FileObjectStore, ObjectReader, ObjectWriter};
pub use refs::{RefLogReplay, RefLogRecord, RefPublication, RefStore};
pub use wal::{Wal, WalRecord, WalReplay};
pub use verify::{verify_repository, ObjectVerification, RepositoryVerification};
