//! Rollback draft verification tests.

use prikk_object::{SignatureAlgorithm, SignerRole};

use crate::{
    append_rollback_draft, verify_active_rollback_draft, verify_repository, RepositoryLayout,
    Wal,
};

use super::helpers::{signed_patch_envelope, unique_temp_dir};
use super::patch_replay::publish_snapshot_then_patch_block;

#[test]
fn rollback_draft_verify_matches_current_inverse_plan() {
    let root = unique_temp_dir("rollback-draft-verify");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published = publish_snapshot_then_patch_block(&layout);
        assert!(published.is_ok());
        let draft = append_rollback_draft(&layout, "heads/main", "rollback verify");
        assert!(draft.is_ok());
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_ok());
        if let Ok(verification) = verification {
            assert_eq!(verification.ref_name, "heads/main");
            assert_eq!(verification.wal_sequence, 1);
            assert_eq!(verification.inverse_operation_count, 3);
            assert_eq!(verification.decoded_operation_count, 3);
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
fn rollback_draft_uses_dedicated_signature_marker() {
    let root = unique_temp_dir("rollback-draft-signature-marker");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published = publish_snapshot_then_patch_block(&layout);
        assert!(published.is_ok());
        let draft = append_rollback_draft(&layout, "heads/main", "rollback marker");
        assert!(draft.is_ok());
        let wal = Wal::new(layout.default_queue_wal_path());
        let replay = wal.replay();
        assert!(replay.is_ok());
        if let Ok(replay) = replay {
            let first = replay.records.first();
            assert!(first.is_some());
            if let Some(first) = first {
                let marker = first.envelope.signatures.iter().any(|signature| {
                    signature.signer_role == SignerRole::Author
                        && signature.algorithm == SignatureAlgorithm::Ed25519
                        && signature.key_id == "dev-placeholder-rollback-author"
                });
                assert!(marker);
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}
