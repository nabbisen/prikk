//! Storage tests.

use prikk_object::{
    CanonicalEncode, EditText, ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind,
    PatchPayload, RefKind, RefStatePayload, RefUpdatePayload, Signature, SignatureAlgorithm,
    SignerRole,
};

use crate::{
    ActiveLock, FileObjectStore, MemoryObjectStore, ObjectReader, ObjectWriter, RefLock,
    RefPublication, RefStore, RepositoryLayout, verify_repository, Wal,
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
fn ref_lock_rejects_second_writer() {
    let root = unique_temp_dir("ref-lock");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let first = RefLock::acquire(layout.ref_lock_path("heads/main"));
        assert!(first.is_ok());
        let second = RefLock::acquire(layout.ref_lock_path("heads/main"));
        assert!(second.is_err());
        drop(first);
        let third = RefLock::acquire(layout.ref_lock_path("heads/main"));
        assert!(third.is_ok());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ref_store_publishes_ref_state_and_log() {
    let root = unique_temp_dir("ref-store");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let store = RefStore::new(layout.clone());
        let target = sample_object_id("target-block");
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        let published = store.publish(&publication);
        assert_eq!(published, Ok(ref_state_id));
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(Some(ref_state_id)));
        let log = store.replay_log("heads/main");
        assert!(log.is_ok());
        if let Ok(log) = log {
            assert_eq!(log.records.len(), 1);
            assert_eq!(log.trailing_partial_bytes, 0);
        }
        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.checked_refs, 1);
            assert_eq!(report.checked_ref_log_records, 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ref_store_rejects_cas_mismatch() {
    let root = unique_temp_dir("ref-cas");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let store = RefStore::new(layout);
        let target = sample_object_id("target-block");
        let bogus_previous = Some(sample_object_id("bogus-previous"));
        let ref_state = signed_ref_state_envelope("heads/main", bogus_previous, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", bogus_previous, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: bogus_previous,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_err());
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(None));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_detects_missing_ref_state_object() {
    let root = unique_temp_dir("ref-missing-state");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let store = RefStore::new(layout.clone());
        let target = sample_object_id("target-block");
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_ok());
        let ref_state_path = layout.object_path(ObjectType::RefState, ref_state_id);
        assert!(std::fs::remove_file(ref_state_path).is_ok());
        assert!(verify_repository(&layout).is_err());
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


#[test]
fn verify_repository_counts_objects_and_wal_records() {
    let root = unique_temp_dir("verify");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let mut blob = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        assert!(blob.add_signature(dummy_signature()).is_ok());
        assert!(store.write_object(&blob).is_ok());

        let wal = Wal::new(layout.default_queue_wal_path());
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.checked_objects, 1);
            assert_eq!(report.checked_wal_records, 1);
            assert_eq!(report.checked_refs, 0);
            assert_eq!(report.checked_ref_log_records, 0);
            assert_eq!(report.trailing_partial_wal_bytes, 0);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_detects_object_file_in_wrong_prefix() {
    let root = unique_temp_dir("verify-wrong-prefix");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        let id = store.write_object(&envelope);
        assert!(id.is_ok());
        if let Ok(id) = id {
            let correct = layout.object_path(ObjectType::Blob, id);
            let wrong_dir = layout.object_type_dir(ObjectType::Blob).join("ff");
            assert!(std::fs::create_dir_all(&wrong_dir).is_ok());
            let wrong = wrong_dir.join(format!("{}.pobj", id.to_hex()));
            assert!(std::fs::rename(correct, wrong).is_ok());
            assert!(verify_repository(&layout).is_err());
        }
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

fn signed_ref_state_envelope(
    ref_name: &str,
    previous_ref_state_id: Option<ObjectId>,
    target_object_id: ObjectId,
    update_seq: u64,
) -> ObjectEnvelope {
    let payload = RefStatePayload {
        ref_name: ref_name.to_string(),
        kind: RefKind::Branch,
        target_object_id,
        update_seq,
        previous_ref_state_id,
        required_attestation_ids: Vec::new(),
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::RefState, 1, bytes);
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}

fn signed_ref_update_envelope(
    ref_name: &str,
    old_ref_state_id: Option<ObjectId>,
    new_ref_state_id: ObjectId,
    new_target_object_id: ObjectId,
    update_seq: u64,
) -> ObjectEnvelope {
    let payload = RefUpdatePayload {
        ref_name: ref_name.to_string(),
        old_ref_state_id,
        new_ref_state_id,
        new_target_object_id,
        update_seq,
        created_at: 7,
        author_key_id: "maintainer-key".to_string(),
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::RefUpdate, 1, bytes);
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}

fn sample_object_id(label: &str) -> ObjectId {
    ObjectId::from_canonical_payload(ObjectType::Blob, 1, label.as_bytes())
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

fn maintainer_signature() -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "maintainer-key".to_string(),
        signature_bytes: vec![5, 6, 7, 8],
        created_at: 8,
        signer_role: SignerRole::Maintainer,
    }
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("prikk-pr007-{name}-{}-{}", std::process::id(), monotonic_suffix()));
    path
}

fn monotonic_suffix() -> u128 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    }
}
