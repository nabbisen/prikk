//! Container-backed object store (RFC 102 Stage 3). Public API (`FileObjectStore`, `ObjectReader`,
//! `ObjectWriter`) is unchanged from the loose-file implementation this replaces -- every other call
//! site in the workspace uses only that trait interface, so none of them needed to change. Only the
//! internals moved: reads and writes now go through `index.rs`'s lookup/write-protocol functions,
//! which target `container.rs`'s per-type container files instead of one file per object.

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

use crate::index::{lookup_object_location, read_object_envelope_at, write_object_to_container};
use crate::layout::RepositoryLayout;

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

    /// Return true if an object with this id and type is indexed.
    #[must_use]
    pub fn contains_object(&self, object_type: ObjectType, id: ObjectId) -> bool {
        if object_type == ObjectType::RefUpdate {
            return false;
        }
        matches!(
            lookup_object_location(&self.layout, id),
            Ok(Some(entry)) if entry.object_type == object_type
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
        let Some(entry) = lookup_object_location(&self.layout, id)? else {
            return Ok(None);
        };
        // "One seek" (design §12/§10.3): the index already named exactly where this object is, so
        // this decodes directly at that offset rather than scanning the container from the start.
        let envelope = read_object_envelope_at(&self.layout, &entry)?;
        let computed = envelope.object_id();
        if computed != id {
            // The read validation the ruling requires: the index is trusted for *location*, but the
            // bytes found there are always checked against the id actually asked for by recomputing
            // it from the decoded content -- free, since decoding already happened. A mismatch is
            // reported, never silently accepted and never a fallback to scanning.
            return Err(PrikkError::Integrity(format!(
                "index entry for {id} resolves to an envelope with computed id {computed}"
            )));
        }
        if envelope.object_type != entry.object_type {
            return Err(PrikkError::Integrity(format!(
                "index entry for {id} names type {}, envelope decoded as {}",
                entry.object_type, envelope.object_type
            )));
        }
        crate::format::validate_read_schema(self.layout.format(), &envelope)?;
        Ok(Some(envelope))
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
        // The write protocol (design §5, handoff §3) lives in `index.rs`, not here: append the
        // object record to its container and make it durable, then and only then append the index
        // entry. Stated at that call site too, not only here.
        write_object_to_container(&self.layout, envelope.object_type, envelope)
    }
}

// DC-97 correction of the comment this replaced: the Linux/macOS-only reasoning was true when
// written (DC-71/DC-81, before DC-87 made Windows a mutating platform) and nobody revisited it once
// Windows mutation shipped -- found only by DC-97's own G5 investigation, back when this module's
// now-deleted `tests::immutable` still made the claimed Windows evidence for G5. `publish_immutable`
// and its tests are gone entirely as of DC-98 (G5 retired, zero production callers). What remains
// here is gated the same way regardless: `RepositoryLayout::init` and real repository mutation are
// not Linux/macOS-only, so what is still unix-only inside this module (failpoints, symlinks, FIFOs)
// is gated per-test/per-file instead of by one blanket gate.
#[cfg(all(
    test,
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
mod tests;
