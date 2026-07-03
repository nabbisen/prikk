//! Rollback draft append tests.

use prikk_crypto::verify_ed25519;
use prikk_object::{ObjectType, PatchPurpose, Signature, SignatureAlgorithm, SignerRole};

use crate::{Ed25519AuthorSigner, RepositoryLayout, Wal, append_rollback_draft};

use crate::test_support::publish_snapshot_then_patch_block;
use crate::test_support::{signed_patch_envelope, unique_temp_dir};

fn test_signer() -> Ed25519AuthorSigner {
    Ed25519AuthorSigner::from_seed("rollback-author-key", &[9_u8; 32])
}

#[test]
fn rollback_draft_appends_inverse_patch_to_empty_active_wal() {
    let root = unique_temp_dir("rollback-draft-file-ops");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_snapshot_then_patch_block(&layout);
        assert!(result.is_ok());
        let signer = test_signer();
        let report =
            append_rollback_draft(&layout, "heads/main", "rollback supported ops", &signer);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.ref_name, "heads/main");
            assert_eq!(report.wal_sequence, 1);
            assert_eq!(report.author_key_id, "rollback-author-key");
            assert_eq!(report.inverse_operation_count, 2);
            assert_eq!(report.preview_change_count, 2);
            let wal = Wal::new(layout.default_queue_wal_path());
            let replay = wal.replay();
            assert!(replay.is_ok());
            if let Ok(replay) = replay {
                assert_eq!(replay.records.len(), 1);
                let first = replay.records.first();
                assert!(first.is_some());
                if let Some(first) = first {
                    assert_eq!(first.envelope.object_type, ObjectType::Patch);
                    assert_eq!(first.envelope.object_id(), report.inverse_patch_id);
                    assert!(!first.envelope.signatures.is_empty());
                    assert!(!first.envelope.canonical_payload.is_empty());
                    assert_eq!(
                        PatchPurpose::decode_from_patch_payload(&first.envelope.canonical_payload),
                        Ok(PatchPurpose::RollbackDraft)
                    );
                    let signature = first
                        .envelope
                        .signatures
                        .iter()
                        .find(|signature| signature.signer_role == SignerRole::Author);
                    assert!(signature.is_some());
                    if let Some(signature) = signature {
                        assert_eq!(signature.algorithm, SignatureAlgorithm::Ed25519);
                        assert_eq!(signature.key_id, "rollback-author-key");
                        let preimage = Signature::signed_bytes(
                            SignatureAlgorithm::Ed25519,
                            ObjectType::Patch,
                            first.envelope.object_id(),
                            SignerRole::Author,
                            &signature.key_id,
                        );
                        assert!(
                            verify_ed25519(
                                &signer.public_key_bytes(),
                                &preimage,
                                &signature.signature_bytes
                            )
                            .is_ok()
                        );
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_refuses_non_empty_active_wal() {
    let root = unique_temp_dir("rollback-draft-non-empty-wal");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_snapshot_then_patch_block(&layout);
        assert!(result.is_ok());
        let wal = Wal::new(layout.default_queue_wal_path());
        let append = wal.append_patch(&signed_patch_envelope());
        assert!(append.is_ok());
        let signer = test_signer();
        let report =
            append_rollback_draft(&layout, "heads/main", "rollback with pending work", &signer);
        assert!(report.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}
