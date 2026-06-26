//! Ref-state pointer and ref-log publication primitives.
//!
//! PR-007 implements the storage mechanics needed before a full seal command exists: a RefState is
//! stored as a normal content-addressed object, the ref file is a durable pointer to that object,
//! and RefUpdate entries are stored inline in an append-only log. The module does not yet perform
//! publication-policy evaluation or patch/block sealing.

mod log;
mod pointer;
mod verify;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

use crate::fsutil::sync_directory_best_effort;
use crate::layout::RepositoryLayout;
use crate::lock::RefLock;
use crate::object_store::{FileObjectStore, ObjectWriter};

pub use log::{RefLogRecord, RefLogReplay};
pub(crate) use verify::verify_refs;

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

    fn ensure_current_matches(
        &self,
        ref_name: &str,
        expected: Option<ObjectId>,
    ) -> Result<()> {
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
            return Err(PrikkError::Io("ref pointer path has no parent directory".to_string()));
        };
        std::fs::create_dir_all(parent)?;
        std::fs::rename(candidate, &pointer)?;
        sync_directory_best_effort(parent)?;
        Ok(())
    }
}

pub(crate) fn validate_publication(publication: &RefPublication) -> Result<()> {
    if publication.ref_name.is_empty() {
        return Err(PrikkError::InvalidName("ref name must not be empty".to_string()));
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
