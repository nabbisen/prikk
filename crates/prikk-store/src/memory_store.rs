//! In-memory object store for tests and early callers.

use std::collections::BTreeMap;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

use crate::object_store::{ObjectReader, ObjectWriter};

/// In-memory test object store for fixtures and early callers.
#[derive(Debug, Default)]
pub struct MemoryObjectStore {
    objects: BTreeMap<ObjectId, ObjectEnvelope>,
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
    pub fn read_typed(
        &self,
        id: ObjectId,
        object_type: ObjectType,
    ) -> Result<Option<ObjectEnvelope>> {
        if let Some(object) = self.objects.get(&id) {
            if object.object_type != object_type {
                return Err(PrikkError::ObjectTypeMismatch {
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
        envelope.validate_strict()?;
        let id = envelope.object_id();
        self.objects.insert(id, envelope.clone());
        Ok(id)
    }
}
