//! Rollback draft verification tests.

use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectType, OperationKind, PatchPurpose, Signature,
    SignatureAlgorithm, SignerRole,
};

use crate::{
    Ed25519AuthorSigner, FileObjectStore, ObjectWriter, RepositoryLayout, Wal,
    append_rollback_draft, author_signature, prepare_patch_inverse_plan,
    verify_active_rollback_draft, verify_repository,
};

use crate::test_support::{
    legacy_rollback_marker_signature, rollback_patch_envelope, signed_block, signed_patch_envelope,
    signed_ref_state_envelope, signed_ref_update_envelope, unique_temp_dir,
};
use crate::test_support::{publish_snapshot_then_patch_block, publish_text_create_then_edit_block};
use crate::{RefPublication, RefStore};

fn test_signer() -> Ed25519AuthorSigner {
    Ed25519AuthorSigner::from_seed("rollback-author-key", &[9_u8; 32]).unwrap()
}

#[test]
fn rollback_draft_verify_matches_current_inverse_plan() {
    let root = unique_temp_dir("rollback-draft-verify");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published = publish_snapshot_then_patch_block(&layout);
        assert!(published.is_ok());
        let signer = test_signer();
        let draft = append_rollback_draft(&layout, "heads/main", "rollback verify", &signer);
        assert!(draft.is_ok());
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_ok());
        if let Ok(verification) = verification {
            assert_eq!(verification.ref_name, "heads/main");
            assert_eq!(verification.wal_sequence, 1);
            assert_eq!(verification.author_key_id, "rollback-author-key");
            assert_eq!(verification.inverse_operation_count, 2);
            assert_eq!(verification.decoded_operation_count, 2);
        }
        let repository = verify_repository(&layout);
        assert!(repository.is_ok());
        if let Ok(repository) = repository {
            assert_eq!(repository.checked_rollback_draft_records, 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_verify_matches_arbitrary_span_text_inverse() {
    let root = unique_temp_dir("rollback-draft-verify-edit-text");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published =
            publish_text_create_then_edit_block(&layout, b"alpha beta\n", b"alpha BETA\n");
        assert!(published.is_ok());
        let signer = test_signer();
        let draft = append_rollback_draft(&layout, "heads/main", "rollback text verify", &signer);
        assert!(draft.is_ok());
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_ok());
        if let Ok(verification) = verification {
            assert_eq!(verification.author_key_id, "rollback-author-key");
            assert_eq!(verification.inverse_operation_count, 2);
            assert_eq!(verification.decoded_operation_count, 2);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_verify_refuses_plain_active_patch() {
    let root = unique_temp_dir("rollback-draft-verify-plain");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published = publish_snapshot_then_patch_block(&layout);
        assert!(published.is_ok());
        let wal = Wal::new(layout.default_queue_wal_path());
        let append = wal.append_patch(&signed_patch_envelope());
        assert!(append.is_ok());
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_verify_rejects_normal_purpose_byte_identical_inverse_ops() {
    let root = unique_temp_dir("rollback-draft-verify-normal-purpose");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published =
            publish_text_create_then_edit_block(&layout, b"alpha beta\n", b"alpha BETA\n");
        assert!(published.is_ok());
        let inverse = prepare_patch_inverse_plan(&layout, "heads/main");
        assert!(inverse.is_ok());
        if let Ok(mut inverse) = inverse {
            inverse.inverse_payload.purpose = PatchPurpose::Normal;
            let envelope = signed_patch_from_payload(inverse.inverse_payload, &test_signer());
            assert!(envelope.is_ok());
            if let Ok(envelope) = envelope {
                assert!(
                    Wal::new(layout.default_queue_wal_path())
                        .append_patch(&envelope)
                        .is_ok()
                );
            }
        }
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_verify_rejects_stale_inverse_anchor() {
    let root = unique_temp_dir("rollback-draft-verify-stale-anchor");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published =
            publish_text_create_then_edit_block(&layout, b"alpha beta\n", b"alpha BETA\n");
        assert!(published.is_ok());
        let inverse = prepare_patch_inverse_plan(&layout, "heads/main");
        assert!(inverse.is_ok());
        if let Ok(mut inverse) = inverse {
            inverse.inverse_payload.purpose = PatchPurpose::RollbackDraft;
            let operation = inverse.inverse_payload.operations.first_mut();
            assert!(operation.is_some());
            match operation.map(|operation| &mut operation.kind) {
                Some(OperationKind::EditText(edit)) => edit.left_anchor_hash[0] ^= 0x01,
                other => panic!("expected inverse EditText, got {other:?}"),
            }
            let envelope = signed_patch_from_payload(inverse.inverse_payload, &test_signer());
            assert!(envelope.is_ok());
            if let Ok(envelope) = envelope {
                assert!(
                    Wal::new(layout.default_queue_wal_path())
                        .append_patch(&envelope)
                        .is_ok()
                );
            }
        }
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_verify_rejects_generated_presentation_hint() {
    let root = unique_temp_dir("rollback-draft-verify-presentation-hint");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published =
            publish_text_create_then_edit_block(&layout, b"alpha beta\n", b"alpha BETA\n");
        assert!(published.is_ok());
        let inverse = prepare_patch_inverse_plan(&layout, "heads/main");
        assert!(inverse.is_ok());
        if let Ok(mut inverse) = inverse {
            inverse.inverse_payload.purpose = PatchPurpose::RollbackDraft;
            let operation = inverse.inverse_payload.operations.first_mut();
            assert!(operation.is_some());
            match operation.map(|operation| &mut operation.kind) {
                Some(OperationKind::EditText(edit)) => edit.presentation_hint_line = Some(1),
                other => panic!("expected inverse EditText, got {other:?}"),
            }
            let envelope = signed_patch_from_payload(inverse.inverse_payload, &test_signer());
            assert!(envelope.is_ok());
            if let Ok(envelope) = envelope {
                assert!(
                    Wal::new(layout.default_queue_wal_path())
                        .append_patch(&envelope)
                        .is_ok()
                );
            }
        }
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn sealed_history_classifies_payload_purpose_not_legacy_key_id() {
    let root = unique_temp_dir("rollback-draft-purpose-classification");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let rollback_patch = rollback_patch_envelope();
        assert_eq!(
            PatchPurpose::decode_from_patch_payload(&rollback_patch.canonical_payload),
            Ok(PatchPurpose::RollbackDraft)
        );
        let rollback_patch_id = object_store.write_object(&rollback_patch);
        assert!(rollback_patch_id.is_ok());
        if let Ok(rollback_patch_id) = rollback_patch_id {
            let block = signed_block(
                prikk_object::BlockKind::Root,
                Vec::new(),
                vec![rollback_patch_id],
                None,
            );
            let block_id = object_store.write_object(&block);
            assert!(block_id.is_ok());
            if let Ok(block_id) = block_id {
                let ref_store = RefStore::new(layout.clone());
                let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
                let ref_state_id = ref_state.object_id();
                let ref_update =
                    signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
                let published = ref_store.publish(&RefPublication {
                    ref_name: "heads/main".to_string(),
                    expected_previous_ref_state_id: None,
                    ref_state,
                    ref_update,
                });
                assert!(published.is_ok());

                let verification = verify_repository(&layout);
                assert!(verification.is_ok());
                if let Ok(verification) = verification {
                    assert_eq!(verification.checked_rollback_blocks, 1);
                    assert_eq!(verification.checked_sealed_rollback_patches, 1);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn legacy_key_id_without_payload_purpose_is_not_classified() {
    let root = unique_temp_dir("rollback-draft-legacy-clean-break");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let mut legacy = signed_patch_envelope();
        legacy.signatures.clear();
        assert!(
            legacy
                .add_signature(crate::test_support::legacy_rollback_marker_signature())
                .is_ok()
        );
        assert_eq!(
            PatchPurpose::decode_from_patch_payload(&legacy.canonical_payload),
            Ok(PatchPurpose::Normal)
        );
        let legacy_patch_id = object_store.write_object(&legacy);
        assert!(legacy_patch_id.is_ok());
        if let Ok(legacy_patch_id) = legacy_patch_id {
            let block = signed_block(
                prikk_object::BlockKind::Root,
                Vec::new(),
                vec![legacy_patch_id],
                None,
            );
            let block_id = object_store.write_object(&block);
            assert!(block_id.is_ok());
            if let Ok(block_id) = block_id {
                let ref_store = RefStore::new(layout.clone());
                let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
                let ref_state_id = ref_state.object_id();
                let ref_update =
                    signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
                let published = ref_store.publish(&RefPublication {
                    ref_name: "heads/main".to_string(),
                    expected_previous_ref_state_id: None,
                    ref_state,
                    ref_update,
                });
                assert!(published.is_ok());
                let verification = verify_repository(&layout);
                assert!(verification.is_ok());
                if let Ok(verification) = verification {
                    assert_eq!(verification.checked_rollback_blocks, 0);
                    assert_eq!(verification.checked_sealed_rollback_patches, 0);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_purpose_with_legacy_marker_signature_is_rejected() {
    let mut envelope = rollback_patch_envelope();
    envelope.signatures.clear();
    assert!(
        envelope
            .add_signature(legacy_rollback_marker_signature())
            .is_ok()
    );
    let verified = super::verify_rollback_patch_envelope(&envelope, "test rollback patch");
    assert!(verified.is_err());
}

#[test]
fn rollback_purpose_with_short_ed25519_author_signature_is_rejected() {
    let mut envelope = rollback_patch_envelope();
    envelope.signatures.clear();
    assert!(
        envelope
            .add_signature(Signature {
                algorithm: SignatureAlgorithm::Ed25519,
                key_id: "rollback-author-key".to_string(),
                signature_bytes: vec![1],
                created_at: 1,
                signer_role: SignerRole::Author,
            })
            .is_ok()
    );
    let verified = super::verify_rollback_patch_envelope(&envelope, "test rollback patch");
    assert!(verified.is_err());
}

#[test]
fn rollback_purpose_without_author_signature_is_rejected() {
    let mut envelope = rollback_patch_envelope();
    envelope
        .signatures
        .retain(|sig| sig.signer_role != SignerRole::Author);
    let verified = super::verify_rollback_patch_envelope(&envelope, "test rollback patch");
    assert!(verified.is_err());
}

fn signed_patch_from_payload(
    payload: prikk_object::PatchPayload,
    signer: &Ed25519AuthorSigner,
) -> prikk_error::Result<ObjectEnvelope> {
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload.to_canonical_bytes()?);
    let signature = author_signature(signer, envelope.object_id())?;
    envelope.add_signature(signature)?;
    Ok(envelope)
}
