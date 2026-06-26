//! File-backed object store.

use std::fs;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

use crate::file_codec::{decode_envelope_file, encode_envelope_file};
use crate::fsutil::{sync_directory_best_effort, write_file_atomically};
use crate::layout::{persisted_object_types, RepositoryLayout};

/// Read-only object access boundary.
pub trait ObjectReader {
    /// Read an object by ID.
    fn read_object(&self, id: ObjectId) -> Result<Option<ObjectEnvelope>>;
}

/// Write object boundary.
pub trait ObjectWriter {
    /// Write an object envelope after validation.
    fn write_object(&mut self, envelope: &ObjectEnvelope) -> Result<ObjectId>;
}

/// File-backed object store.
#[derive(Debug, Clone)]
pub struct FileObjectStore {
    layout: RepositoryLayout,
}

impl FileObjectStore {
    /// Create a file object store for a repository layout.
    #[must_use]
    pub fn new(layout: RepositoryLayout) -> Self {
        Self { layout }
    }

    /// Return the repository layout.
    #[must_use]
    pub fn layout(&self) -> &RepositoryLayout {
        &self.layout
    }

    /// Return true if an object path exists.
    #[must_use]
    pub fn contains_object(&self, object_type: ObjectType, id: ObjectId) -> bool {
        if object_type == ObjectType::RefUpdate {
            return false;
        }
        self.layout.object_path(object_type, id).is_file()
    }

    /// Read and require a specific object type.
    pub fn read_typed(
        &self,
        id: ObjectId,
        object_type: ObjectType,
    ) -> Result<Option<ObjectEnvelope>> {
        let Some(envelope) = self.read_object(id)? else {
            return Ok(None);
        };
        if envelope.object_type != object_type {
            return Err(PrikkError::ObjectTypeMismatch {
                expected: object_type.to_string(),
                actual: envelope.object_type.to_string(),
            });
        }
        Ok(Some(envelope))
    }
}

impl ObjectReader for FileObjectStore {
    fn read_object(&self, id: ObjectId) -> Result<Option<ObjectEnvelope>> {
        for object_type in persisted_object_types() {
            let path = self.layout.object_path(object_type, id);
            if path.is_file() {
                let bytes = fs::read(&path)?;
                let envelope = decode_envelope_file(&bytes)?;
                let computed = envelope.object_id();
                if computed != id {
                    return Err(PrikkError::Integrity(format!(
                        "object path {id} contains envelope with computed id {computed}"
                    )));
                }
                if envelope.object_type != object_type {
                    return Err(PrikkError::Integrity(format!(
                        "object path type {object_type} contains envelope type {}",
                        envelope.object_type
                    )));
                }
                return Ok(Some(envelope));
            }
        }
        Ok(None)
    }
}

impl ObjectWriter for FileObjectStore {
    fn write_object(&mut self, envelope: &ObjectEnvelope) -> Result<ObjectId> {
        if envelope.object_type == ObjectType::RefUpdate {
            return Err(PrikkError::UnsupportedObjectType(
                "RefUpdate is stored inline in ref logs for v1".to_string(),
            ));
        }
        envelope.validate()?;
        let id = envelope.object_id();
        let path = self.layout.object_path(envelope.object_type, id);
        if path.is_file() {
            return Ok(id);
        }
        let Some(parent) = path.parent() else {
            return Err(PrikkError::Io("object path has no parent directory".to_string()));
        };
        fs::create_dir_all(parent)?;
        let bytes = encode_envelope_file(envelope)?;
        write_file_atomically(&path, &bytes)?;
        sync_directory_best_effort(parent)?;
        Ok(id)
    }
}
