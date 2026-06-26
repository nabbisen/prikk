#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate for PRIKK repositories.
//!
//! PR-006 contains persistent layout, object storage, WAL durability, and read-only
//! repository verification. Ref publication, patch algebra, plugin execution, and remote
//! sync remain separate increments.

mod byte_cursor;
mod file_codec;
mod fsutil;
mod layout;
mod lock;
mod memory_store;
mod object_store;
mod wal;
mod verify;

#[cfg(test)]
mod tests;

pub use layout::RepositoryLayout;
pub use lock::ActiveLock;
pub use memory_store::MemoryObjectStore;
pub use object_store::{FileObjectStore, ObjectReader, ObjectWriter};
pub use wal::{Wal, WalRecord, WalReplay};
pub use verify::{verify_repository, ObjectVerification, RepositoryVerification};
