//! Storage tests.

use prikk_object::{
    CanonicalEncode, EditText, ObjectEnvelope, ObjectType, Operation, OperationKind, PatchPayload,
    Signature, SignatureAlgorithm, SignerRole,
};

use crate::{
    ActiveLock, FileObjectStore, MemoryObjectStore, ObjectReader, ObjectWriter, RepositoryLayout,
    Wal,
};

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
fn active_lock_rejects_second_writer() {
    let root = unique_temp_dir("lock");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let first = ActiveLock::acquire(layout.default_active_lock_path());
        assert!(first.is_ok());
        let second = ActiveLock::acquire(layout.default_active_lock_path());
        assert!(second.is_err());
        drop(first);
        let third = ActiveLock::acquire(layout.default_active_lock_path());
        assert!(third.is_ok());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn wal_roundtrips_signed_patch_envelope() {
    let root = unique_temp_dir("wal");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::new(layout.default_queue_wal_path());
        let envelope = signed_patch_envelope();
        let seq = wal.append_patch(&envelope);
        assert_eq!(seq, Ok(1));
        let replay = wal.replay();
        assert!(replay.is_ok());
        if let Ok(replay) = replay {
            assert_eq!(replay.trailing_partial_bytes, 0);
            assert_eq!(replay.records.len(), 1);
            let first = replay.records.first();
            assert!(first.is_some());
            if let Some(first) = first {
                assert_eq!(first.seq, 1);
                assert_eq!(first.envelope, envelope);
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn wal_rejects_unsigned_patch_envelope() {
    let root = unique_temp_dir("wal-unsigned");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::new(layout.default_queue_wal_path());
        let mut envelope = signed_patch_envelope();
        envelope.signatures.clear();
        let result = wal.append_patch(&envelope);
        assert!(result.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

fn signed_patch_envelope() -> ObjectEnvelope {
    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::EditText(EditText {
                path: "a.txt".to_string(),
                anchor_id: "anchor-1".to_string(),
                old_span_hash: vec![1, 2, 3],
                replacement: "hello".to_string(),
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, bytes);
    assert!(envelope.add_signature(dummy_signature()).is_ok());
    envelope
}

fn dummy_signature() -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "author-key".to_string(),
        signature_bytes: vec![1, 2, 3, 4],
        created_at: 7,
        signer_role: SignerRole::Author,
    }
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("prikk-pr004-{name}-{}-{}", std::process::id(), monotonic_suffix()));
    path
}

fn monotonic_suffix() -> u128 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    }
}
