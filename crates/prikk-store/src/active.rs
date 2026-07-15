//! Active-session commit helpers.
//!
//! This module is the narrow boundary between higher-level commit construction and the
//! durable active WAL. It owns lock acquisition for the default active session and appends only
//! already-constructed, signed patch envelopes. It also owns the local ref-name metadata that makes a
//! non-empty active WAL unambiguously belong to one target ref.

use prikk_error::{PrikkError, Result};
use prikk_object::ObjectEnvelope;

use crate::fsutil::{read_file_if_exists, remove_file_if_present_required, write_file_atomically};
use crate::layout::RepositoryLayout;
use crate::lock::ActiveLock;
use crate::refs::validate_local_branch_ref;
use crate::wal::Wal;

/// Result of appending a patch envelope to the active session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveCommitResult {
    /// WAL sequence assigned to the appended patch envelope.
    pub wal_sequence: u64,
}

/// Active-WAL ref metadata read result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveRefMetadata {
    /// Metadata file is absent.
    Missing,
    /// Metadata file contains a valid canonical local branch ref.
    Valid(String),
    /// Metadata file exists but is malformed or not a valid local branch ref.
    Invalid(String),
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
        let _lock = ActiveLock::acquire(&self.layout)?;
        let wal = Wal::for_layout(&self.layout);
        let replay = wal.replay()?;
        if replay.trailing_partial_bytes != 0 {
            return Err(PrikkError::Integrity(format!(
                "active WAL has {} trailing partial bytes; run doctor before appending",
                replay.trailing_partial_bytes
            )));
        }
        if !replay.records.is_empty() {
            require_active_ref_for_non_empty_wal(&self.layout, "heads/main")?;
            return Err(PrikkError::LockConflict(
                "active WAL already contains patches for heads/main; seal before appending again"
                    .to_string(),
            ));
        }
        prepare_empty_active_ref_for_append(&self.layout, "heads/main")?;
        let wal_sequence = wal.append_patch(envelope)?;
        Ok(ActiveCommitResult { wal_sequence })
    }
}

/// Read active-WAL ref metadata without mutating it.
pub fn read_active_ref_metadata(layout: &RepositoryLayout) -> Result<ActiveRefMetadata> {
    let relative = layout.repository_relative(&layout.default_active_ref_name_path())?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &relative)? else {
        return Ok(ActiveRefMetadata::Missing);
    };
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(err) => {
            return Ok(ActiveRefMetadata::Invalid(format!(
                "active ref metadata is not UTF-8: {err}"
            )));
        }
    };
    match validate_local_branch_ref(text) {
        Ok(canonical) => Ok(ActiveRefMetadata::Valid(canonical)),
        Err(err) => Ok(ActiveRefMetadata::Invalid(err.to_string())),
    }
}

/// Write active-WAL ref metadata through a durable atomic update.
pub fn write_active_ref_metadata(layout: &RepositoryLayout, ref_name: &str) -> Result<String> {
    let canonical = validate_local_branch_ref(ref_name)?;
    let relative = layout.repository_relative(&layout.default_active_ref_name_path())?;
    write_file_atomically(
        layout.repository_mutation_root(),
        &relative,
        canonical.as_bytes(),
    )?;
    Ok(canonical)
}

/// Remove active-WAL ref metadata and fsync the active-session directory.
pub fn remove_active_ref_metadata(layout: &RepositoryLayout) -> Result<bool> {
    let relative = layout.repository_relative(&layout.default_active_ref_name_path())?;
    remove_file_if_present_required(layout.repository_mutation_root(), &relative)
}

/// Prepare active ref metadata for the first WAL append.
///
/// Caller must hold the active-session lock and must call this only after replay has proven that the
/// active WAL has no records and no trailing partial bytes.
pub(crate) fn prepare_empty_active_ref_for_append(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<String> {
    match read_active_ref_metadata(layout)? {
        ActiveRefMetadata::Missing => {}
        ActiveRefMetadata::Valid(_) | ActiveRefMetadata::Invalid(_) => {
            remove_active_ref_metadata(layout)?;
        }
    }
    write_active_ref_metadata(layout, ref_name)
}

/// Validate active ref metadata for a non-empty active WAL.
pub(crate) fn require_active_ref_for_non_empty_wal(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<String> {
    let expected = validate_local_branch_ref(ref_name)?;
    match read_active_ref_metadata(layout)? {
        ActiveRefMetadata::Valid(actual) if actual == expected => Ok(actual),
        ActiveRefMetadata::Valid(actual) => Err(PrikkError::LockConflict(format!(
            "active WAL is owned by {actual}; requested ref {expected}"
        ))),
        ActiveRefMetadata::Missing => Err(PrikkError::Integrity(
            "active WAL has records but active ref metadata is missing".to_string(),
        )),
        ActiveRefMetadata::Invalid(reason) => Err(PrikkError::Integrity(format!(
            "active WAL has records but active ref metadata is malformed: {reason}"
        ))),
    }
}

#[cfg(test)]
mod tests;
