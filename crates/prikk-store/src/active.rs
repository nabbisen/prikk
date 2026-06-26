//! Active-session commit helpers.
//!
//! This module is the narrow boundary between higher-level commit construction and the
//! durable active WAL. It owns lock acquisition for the default active session and appends only
//! already-constructed, signed patch envelopes.

use prikk_error::Result;
use prikk_object::ObjectEnvelope;

use crate::layout::RepositoryLayout;
use crate::lock::ActiveLock;
use crate::wal::Wal;

/// Result of appending a patch envelope to the active session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveCommitResult {
    /// WAL sequence assigned to the appended patch envelope.
    pub wal_sequence: u64,
}

/// Default active-session handle.
#[derive(Debug, Clone)]
pub struct ActiveSession {
    layout: RepositoryLayout,
}

impl ActiveSession {
    /// Create an active-session handle for a repository layout.
    #[must_use]
    pub fn new(layout: RepositoryLayout) -> Self {
        Self { layout }
    }

    /// Append one signed patch envelope while holding the active-session lock.
    pub fn append_patch(&self, envelope: &ObjectEnvelope) -> Result<ActiveCommitResult> {
        let _lock = ActiveLock::acquire(self.layout.default_active_lock_path())?;
        let wal = Wal::new(self.layout.default_queue_wal_path());
        let wal_sequence = wal.append_patch(envelope)?;
        Ok(ActiveCommitResult { wal_sequence })
    }
}
