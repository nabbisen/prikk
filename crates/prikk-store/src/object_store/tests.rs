//! Object store and layout tests.

// DC-98 Stage 1: `mod immutable;` and `mod races;` (G5's own conformance and race tests) removed
// along with `DurabilityContract::publish_immutable` -- zero production callers, and G5 retired as
// a guarantee. DC-97 had deferred splitting `races.rs`'s helper rather than deciding it under time
// pressure; the answer turned out to be that the file goes away entirely, resolving that question
// by deletion.

use prikk_object::{ObjectEnvelope, ObjectType};

use crate::{FileObjectStore, MemoryObjectStore, ObjectReader, ObjectWriter, RepositoryLayout};

use crate::test_support::{dummy_signature, unique_temp_dir};

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

#[test]
fn repository_init_creates_required_directories() {
    let root = unique_temp_dir("layout");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        for dir in layout.required_directories() {
            assert!(dir.is_dir(), "missing directory: {}", dir.display());
        }
        assert!(layout.default_queue_wal_path().parent().is_some());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn file_store_roundtrips_signed_object() {
    let root = unique_temp_dir("filestore");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        assert!(envelope.add_signature(dummy_signature()).is_ok());
        let mut store = FileObjectStore::new(layout);
        let id = store.write_object(&envelope);
        assert!(id.is_ok());
        if let Ok(id) = id {
            let read = store.read_object(id);
            assert_eq!(read, Ok(Some(envelope)));
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn object_reads_and_writes_remain_on_retained_repository_root() -> prikk_error::Result<()> {
    let root = unique_temp_dir("object-root-replacement");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut first = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"first".to_vec());
    first.add_signature(dummy_signature())?;
    let mut store = FileObjectStore::new(layout.clone());
    let first_id = store.write_object(&first)?;
    let displaced = root.join(".prikk-displaced");
    std::fs::rename(layout.prikk_dir(), &displaced)?;
    std::fs::create_dir(root.join(".prikk"))?;
    std::fs::write(root.join(".prikk/FORMAT"), b"replacement")?;

    assert!(layout.validate_format().is_ok());
    assert_eq!(store.read_object(first_id)?, Some(first));
    let mut second = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"second".to_vec());
    second.add_signature(dummy_signature())?;
    let second_id = store.write_object(&second)?;
    assert!(store.read_object(second_id)?.is_some());
    assert_eq!(std::fs::read(root.join(".prikk/FORMAT"))?, b"replacement");

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
