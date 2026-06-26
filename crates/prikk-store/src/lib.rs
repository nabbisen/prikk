#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate scaffold.
//!
//! Persistent WAL/object-store/ref implementation belongs to later PRs. This crate currently
//! defines stable boundary traits so callers do not bypass object validation.

use prikk_error::Result;
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

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

/// In-memory test object store for fixtures and early callers.
#[derive(Debug, Default)]
pub struct MemoryObjectStore {
    objects: std::collections::BTreeMap<ObjectId, ObjectEnvelope>,
}

impl MemoryObjectStore {
    /// Create an empty memory store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return number of stored objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Return true when no objects are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Read and require a specific object type.
    pub fn read_typed(&self, id: ObjectId, object_type: ObjectType) -> Result<Option<ObjectEnvelope>> {
        if let Some(object) = self.objects.get(&id) {
            if object.object_type != object_type {
                return Err(prikk_error::PrikkError::ObjectTypeMismatch {
                    expected: object_type.to_string(),
                    actual: object.object_type.to_string(),
                });
            }
            return Ok(Some(object.clone()));
        }
        Ok(None)
    }
}

impl ObjectReader for MemoryObjectStore {
    fn read_object(&self, id: ObjectId) -> Result<Option<ObjectEnvelope>> {
        Ok(self.objects.get(&id).cloned())
    }
}

impl ObjectWriter for MemoryObjectStore {
    fn write_object(&mut self, envelope: &ObjectEnvelope) -> Result<ObjectId> {
        envelope.validate()?;
        let id = envelope.object_id();
        self.objects.insert(id, envelope.clone());
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryObjectStore, ObjectReader, ObjectWriter};
    use prikk_object::{ObjectEnvelope, ObjectType};

    #[test]
    fn memory_store_roundtrips_object() {
        let mut store = MemoryObjectStore::new();
        let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        let id = store.write_object(&envelope);
        assert!(id.is_ok());
        if let Ok(id) = id {
            let read = store.read_object(id);
            assert_eq!(read, Ok(Some(envelope)));
        }
    }
}
