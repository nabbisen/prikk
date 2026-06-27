//! Ref-state pointer and ref-log publication primitives.
//!
//! PR-007 introduced the storage mechanics needed before a full seal command exists: a RefState is
//! stored as a normal content-addressed object, the ref file is a durable pointer to that object,
//! and RefUpdate entries are stored inline in an append-only log. The module does not yet perform
//! publication-policy evaluation or patch/block sealing.

mod log;
mod pointer;
mod verify;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType, RefStatePayload, RefUpdatePayload};

use crate::fsutil::sync_directory_best_effort;
use crate::layout::RepositoryLayout;
use crate::lock::RefLock;
use crate::object_store::{FileObjectStore, ObjectReader, ObjectWriter};

pub use log::{RefLogRecord, RefLogReplay};
pub(crate) use verify::verify_refs;

/// Recoverable ref candidate reconstructed from an append-only ref log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRecoveryCandidate {
    /// Human-readable ref name.
    pub ref_name: String,
    /// RefState ID selected by the latest valid ref-log record.
    pub ref_state_id: ObjectId,
    /// Target Block ID selected by the RefState.
    pub target_object_id: ObjectId,
    /// Update sequence of the latest ref-log record.
    pub update_seq: u64,
}

/// Result of a guarded ref-pointer reconstruction attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRecoveryRepair {
    /// Human-readable ref name.
    pub ref_name: String,
    /// RefState ID reconstructed into the pointer file.
    pub ref_state_id: ObjectId,
    /// Whether a pointer file was written.
    pub wrote_pointer: bool,
}

/// Inputs for a single ref publication primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefPublication {
    /// Human-readable ref name, such as `heads/main`.
    pub ref_name: String,
    /// Expected current RefState ID for CAS. Use `None` to create a new ref.
    pub expected_previous_ref_state_id: Option<ObjectId>,
    /// Signed RefState object envelope to persist before publishing the pointer.
    pub ref_state: ObjectEnvelope,
    /// Signed RefUpdate envelope to append after the ref pointer is durable.
    pub ref_update: ObjectEnvelope,
}

/// File-backed ref-state and ref-log store.
#[derive(Debug, Clone)]
pub struct RefStore {
    layout: RepositoryLayout,
}

impl RefStore {
    /// Create a ref store for a repository layout.
    #[must_use]
    pub fn new(layout: RepositoryLayout) -> Self {
        Self { layout }
    }

    /// Return the repository layout.
    #[must_use]
    pub fn layout(&self) -> &RepositoryLayout {
        &self.layout
    }

    /// Publish a signed RefState with ref-specific locking and CAS.
    pub fn publish(&self, publication: &RefPublication) -> Result<ObjectId> {
        validate_publication(publication)?;
        let ref_state_id = publication.ref_state.object_id();
        let ref_lock = RefLock::acquire(self.layout.ref_lock_path(&publication.ref_name))?;
        let mut object_store = FileObjectStore::new(self.layout.clone());
        object_store.write_object(&publication.ref_state)?;
        self.ensure_current_matches(
            &publication.ref_name,
            publication.expected_previous_ref_state_id,
        )?;
        self.write_ref_pointer_candidate(&publication.ref_name, ref_state_id)?;
        self.ensure_current_matches(
            &publication.ref_name,
            publication.expected_previous_ref_state_id,
        )?;
        self.promote_ref_pointer_candidate(&publication.ref_name)?;
        log::append_log_record(&self.layout, &publication.ref_name, &publication.ref_update)?;
        drop(ref_lock);
        Ok(ref_state_id)
    }

    /// Read the current RefState object ID for a ref name.
    pub fn read_current_ref_state_id(&self, ref_name: &str) -> Result<Option<ObjectId>> {
        let path = self.layout.ref_pointer_path(ref_name);
        if !path.exists() {
            return Ok(None);
        }
        let pointer = pointer::read_ref_pointer(&path)?;
        if pointer.ref_name != ref_name {
            return Err(PrikkError::Integrity(format!(
                "ref pointer name mismatch: expected {ref_name}, got {}",
                pointer.ref_name
            )));
        }
        Ok(Some(pointer.ref_state_id))
    }

    /// Replay the inline ref-update log for a ref name.
    pub fn replay_log(&self, ref_name: &str) -> Result<RefLogReplay> {
        log::replay_log(&self.layout, ref_name)
    }

    /// Return a recoverable ref-pointer candidate when the pointer is missing but the log is valid.
    pub fn recoverable_missing_ref(&self, ref_name: &str) -> Result<Option<RefRecoveryCandidate>> {
        if self.read_current_ref_state_id(ref_name)?.is_some() {
            return Ok(None);
        }
        let replay = self.replay_log(ref_name)?;
        if replay.records.is_empty() {
            return Ok(None);
        }
        if replay.trailing_partial_bytes != 0 {
            return Err(PrikkError::Integrity(format!(
                "ref log for {ref_name} has trailing partial bytes"
            )));
        }
        let object_store = FileObjectStore::new(self.layout.clone());
        let mut previous_ref_state_id = None;
        let mut latest = None;
        for record in &replay.records {
            let update = RefUpdatePayload::decode_canonical(&record.envelope.canonical_payload)?;
            if update.ref_name != ref_name {
                return Err(PrikkError::Integrity(format!(
                    "ref-log record name mismatch: expected {ref_name}, got {}",
                    update.ref_name
                )));
            }
            if update.old_ref_state_id != previous_ref_state_id {
                return Err(PrikkError::Integrity(format!(
                    "ref-log chain mismatch for {ref_name} at update {}",
                    update.update_seq
                )));
            }
            let ref_state = verified_ref_state_payload(
                &object_store,
                update.new_ref_state_id,
                ref_name,
                update.new_target_object_id,
            )?;
            if ref_state.previous_ref_state_id != update.old_ref_state_id {
                return Err(PrikkError::Integrity(format!(
                    "RefState previous link disagrees with RefUpdate for {ref_name}"
                )));
            }
            if ref_state.update_seq != update.update_seq {
                return Err(PrikkError::Integrity(format!(
                    "RefState update sequence disagrees with RefUpdate for {ref_name}"
                )));
            }
            previous_ref_state_id = Some(update.new_ref_state_id);
            latest = Some(update);
        }
        let Some(update) = latest else {
            return Ok(None);
        };
        Ok(Some(RefRecoveryCandidate {
            ref_name: ref_name.to_string(),
            ref_state_id: update.new_ref_state_id,
            target_object_id: update.new_target_object_id,
            update_seq: update.update_seq,
        }))
    }

    /// Reconstruct a missing ref pointer from the latest valid log record.
    ///
    /// This method is deliberately narrow: it writes only the pointer file for a ref whose log and
    /// target RefState object already verify. It does not synthesize objects or repair malformed
    /// logs.
    pub fn reconstruct_missing_ref_from_log(&self, ref_name: &str) -> Result<RefRecoveryRepair> {
        let ref_lock = RefLock::acquire(self.layout.ref_lock_path(ref_name))?;
        if let Some(current) = self.read_current_ref_state_id(ref_name)? {
            drop(ref_lock);
            return Ok(RefRecoveryRepair {
                ref_name: ref_name.to_string(),
                ref_state_id: current,
                wrote_pointer: false,
            });
        }
        let candidate = self.recoverable_missing_ref(ref_name)?.ok_or_else(|| {
            PrikkError::Integrity(format!(
                "ref {ref_name} has no recoverable committed ref-log record"
            ))
        })?;
        self.write_ref_pointer_candidate(ref_name, candidate.ref_state_id)?;
        self.ensure_current_matches(ref_name, None)?;
        self.promote_ref_pointer_candidate(ref_name)?;
        drop(ref_lock);
        Ok(RefRecoveryRepair {
            ref_name: ref_name.to_string(),
            ref_state_id: candidate.ref_state_id,
            wrote_pointer: true,
        })
    }

    fn ensure_current_matches(&self, ref_name: &str, expected: Option<ObjectId>) -> Result<()> {
        let current = self.read_current_ref_state_id(ref_name)?;
        if current != expected {
            return Err(PrikkError::LockConflict(format!(
                "ref CAS mismatch for {ref_name}: expected {:?}, got {:?}",
                expected, current
            )));
        }
        Ok(())
    }

    fn write_ref_pointer_candidate(&self, ref_name: &str, ref_state_id: ObjectId) -> Result<()> {
        pointer::write_ref_pointer_candidate(&self.layout, ref_name, ref_state_id)
    }

    fn promote_ref_pointer_candidate(&self, ref_name: &str) -> Result<()> {
        let candidate = self.layout.ref_tmp_path(ref_name);
        let pointer = self.layout.ref_pointer_path(ref_name);
        let Some(parent) = pointer.parent() else {
            return Err(PrikkError::Io(
                "ref pointer path has no parent directory".to_string(),
            ));
        };
        std::fs::create_dir_all(parent)?;
        std::fs::rename(candidate, &pointer)?;
        sync_directory_best_effort(parent)?;
        Ok(())
    }
}

fn verified_ref_state_payload(
    object_store: &FileObjectStore,
    ref_state_id: ObjectId,
    ref_name: &str,
    target_object_id: ObjectId,
) -> Result<RefStatePayload> {
    let Some(envelope) = object_store.read_typed(ref_state_id, ObjectType::RefState)? else {
        return Err(PrikkError::Integrity(format!(
            "missing RefState object for ref recovery: {ref_state_id}"
        )));
    };
    if envelope.signatures.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} is unsigned"
        )));
    }
    let payload = RefStatePayload::decode_canonical(&envelope.canonical_payload)?;
    if payload.ref_name != ref_name {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} name mismatch: expected {ref_name}, got {}",
            payload.ref_name
        )));
    }
    if payload.target_object_id != target_object_id {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} target disagrees with ref log for {ref_name}"
        )));
    }
    let Some(target) = object_store.read_object(target_object_id)? else {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} targets missing block {target_object_id}"
        )));
    };
    if target.object_type != ObjectType::Block {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} targets {}, expected block",
            target.object_type
        )));
    }
    Ok(payload)
}

pub(crate) fn validate_publication(publication: &RefPublication) -> Result<()> {
    if publication.ref_name.is_empty() {
        return Err(PrikkError::InvalidName(
            "ref name must not be empty".to_string(),
        ));
    }
    require_signed_type(&publication.ref_state, ObjectType::RefState)?;
    require_signed_type(&publication.ref_update, ObjectType::RefUpdate)?;
    Ok(())
}

pub(crate) fn require_signed_type(
    envelope: &ObjectEnvelope,
    object_type: ObjectType,
) -> Result<()> {
    if envelope.object_type != object_type {
        return Err(PrikkError::ObjectTypeMismatch {
            expected: object_type.to_string(),
            actual: envelope.object_type.to_string(),
        });
    }
    if envelope.signatures.is_empty() {
        return Err(PrikkError::InvalidSignature(format!(
            "{object_type} publication envelope must be signed"
        )));
    }
    envelope.validate()
}
