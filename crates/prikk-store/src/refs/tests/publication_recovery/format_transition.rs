use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType, RefKind,
    RefStatePayload, RefUpdatePayload,
};

use crate::test_support::{signed_patch_blob_envelope, signed_patch_envelope, unique_temp_dir};
use crate::{
    ActiveLock, Ed25519MaintainerSigner, FileObjectStore, MaintainerSigner, ObjectWriter,
    RefPublication, RefStore, RepositoryLayout, Wal, add_trusted_maintainer,
    finish_legacy_active_publication_cleanup, maintainer_signature, verify_repository,
    write_active_ref_metadata,
};

#[test]
fn genuine_format1_ahead_log_promotes_without_identity_rewrite() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc40-genuine-format1-ahead");
    let layout = RepositoryLayout::init(root.clone())?;
    let signer = Ed25519MaintainerSigner::from_seed("legacy-maintainer", &[0x35; 32])?;
    let public_key = signer
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    add_trusted_maintainer(&layout, "legacy-maintainer", &public_key)?;

    let patch = signed_patch_envelope();
    let patch_id = patch.object_id();
    let mut objects = FileObjectStore::new(layout.clone());
    objects.write_object(&signed_patch_blob_envelope())?;
    objects.write_object(&patch)?;
    write_active_ref_metadata(&layout, "heads/main")?;
    Wal::for_layout(&layout).append_patch(&patch)?;

    let block_payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: vec![patch_id],
        state_merkle_root: MerkleRoot([0; 32]),
        snapshot_blob_ref: None,
    };
    let block = sign_legacy_publication(
        ObjectType::Block,
        block_payload.to_canonical_bytes()?,
        &signer,
    )?;
    let block_id = block.object_id();
    let block_path = layout.object_path(ObjectType::Block, block_id);
    std::fs::create_dir_all(block_path.parent().ok_or_else(|| {
        prikk_error::PrikkError::Io("legacy Block path has no parent".to_string())
    })?)?;
    let block_bytes = crate::file_codec::encode_envelope_file(&block)?;
    std::fs::write(&block_path, &block_bytes)?;

    let state_payload = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: block_id,
        update_seq: 1,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let ref_state = sign_legacy_publication(
        ObjectType::RefState,
        state_payload.to_canonical_bytes()?,
        &signer,
    )?;
    let ref_state_id = ref_state.object_id();
    let update_payload = RefUpdatePayload {
        ref_name: "heads/main".to_string(),
        old_ref_state_id: None,
        new_ref_state_id: ref_state_id,
        new_target_object_id: block_id,
        update_seq: 1,
        created_at: 0,
        author_key_id: "legacy-maintainer".to_string(),
    };
    let publication = RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: sign_legacy_publication(
            ObjectType::RefUpdate,
            update_payload.to_canonical_bytes()?,
            &signer,
        )?,
        ref_state,
    };
    RefStore::new(layout.clone()).publish(&publication)?;
    std::fs::remove_file(layout.ref_pointer_path("heads/main"))?;
    let log_before = std::fs::read(layout.ref_log_path("heads/main"))?;
    std::fs::write(layout.format_path(), b"1\n")?;

    let legacy_layout = RepositoryLayout::open(root.clone())?;
    let legacy_store = RefStore::new(legacy_layout.clone());
    let verification = verify_repository(&legacy_layout)?;
    assert!(verification.has_unverifiable_state_roots());
    assert!(
        verification
            .ref_publication_issues
            .iter()
            .any(|issue| { issue.code == "PRIKK-VERIFY-REF-POINTER-MISSING" && issue.blocking })
    );
    let active_lock = ActiveLock::acquire(&legacy_layout)?;
    let (_, cleanup_authorization) = legacy_store
        .finish_interrupted_publication_with_cleanup_authorization(&active_lock, &publication)?;
    assert_eq!(
        std::fs::read(legacy_layout.ref_log_path("heads/main"))?,
        log_before
    );
    assert_eq!(std::fs::read(&block_path)?, block_bytes);
    assert_eq!(
        legacy_store.read_current_ref_state_id("heads/main")?,
        Some(ref_state_id)
    );
    let cleanup_authorization = cleanup_authorization.ok_or_else(|| {
        prikk_error::PrikkError::Integrity(
            "legacy publication completion did not issue cleanup authority".to_string(),
        )
    })?;
    finish_legacy_active_publication_cleanup(&legacy_layout, &active_lock, cleanup_authorization)?;
    assert!(std::fs::read(legacy_layout.default_queue_wal_path())?.is_empty());
    assert!(!legacy_layout.default_active_ref_name_path().exists());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

fn sign_legacy_publication(
    object_type: ObjectType,
    payload: Vec<u8>,
    signer: &impl MaintainerSigner,
) -> prikk_error::Result<ObjectEnvelope> {
    let mut envelope = ObjectEnvelope::unsigned(object_type, 1, payload);
    envelope.add_signature(maintainer_signature(
        signer,
        object_type,
        envelope.object_id(),
    )?)?;
    Ok(envelope)
}
