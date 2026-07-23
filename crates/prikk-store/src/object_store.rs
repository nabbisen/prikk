//! File-backed object store.

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

use crate::file_codec::{decode_envelope_file, encode_envelope_file};
use crate::fsutil::{
    EntryKind, ensure_directory_required, inspect_entry, publish_immutable_file,
    read_file_if_exists,
};
use crate::layout::{RepositoryLayout, persisted_object_types};

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
        let path = self.layout.object_path(object_type, id);
        let Ok(relative) = self.layout.repository_relative(&path) else {
            return false;
        };
        matches!(
            inspect_entry(self.layout.repository_mutation_root(), &relative),
            Ok(Some(EntryKind::Regular))
        )
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
            let relative = self.layout.repository_relative(&path)?;
            if let Some(bytes) =
                read_file_if_exists(self.layout.repository_mutation_root(), &relative)?
            {
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
                crate::format::validate_read_schema(self.layout.format(), &envelope)?;
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
        self.layout.validate_format()?;
        crate::format::validate_object_envelope(self.layout.format(), envelope)?;
        let id = envelope.object_id();
        let path = self.layout.object_path(envelope.object_type, id);
        let relative = self.layout.repository_relative(&path)?;
        let Some(parent) = relative.parent() else {
            return Err(PrikkError::Io(
                "object path has no parent directory".to_string(),
            ));
        };
        ensure_directory_required(self.layout.repository_mutation_root(), parent)?;
        let bytes = encode_envelope_file(envelope)?;
        publish_immutable_file(
            self.layout.repository_mutation_root(),
            &relative,
            &bytes,
            |existing| validate_existing_object(existing, envelope.object_type, id),
        )?;
        Ok(id)
    }
}

fn validate_existing_object(
    bytes: &[u8],
    expected_type: ObjectType,
    expected_id: ObjectId,
) -> Result<()> {
    let envelope = decode_envelope_file(bytes).map_err(|error| {
        PrikkError::Integrity(format!("existing immutable object is malformed: {error}"))
    })?;
    if envelope.object_type != expected_type {
        return Err(PrikkError::Integrity(format!(
            "existing object type {} differs from path type {expected_type}",
            envelope.object_type
        )));
    }
    let actual_id = envelope.object_id();
    if actual_id != expected_id {
        return Err(PrikkError::Integrity(format!(
            "existing object id {actual_id} differs from path id {expected_id}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
