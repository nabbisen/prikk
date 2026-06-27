//! Rollback draft append tests.

use prikk_object::ObjectType;

use crate::{append_rollback_draft, RepositoryLayout, Wal};

use super::helpers::{signed_patch_envelope, unique_temp_dir};
use super::patch_replay::{publish_snapshot_then_patch_block, publish_text_edit_block};

#[test]
fn rollback_draft_appends_inverse_patch_to_empty_active_wal() {
    let root = unique_temp_dir("rollback-draft-file-ops");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_snapshot_then_patch_block(&layout);
        assert!(result.is_ok());
        let report = append_rollback_draft(&layout, "heads/main", "rollback supported ops");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.ref_name, "heads/main");
            assert_eq!(report.wal_sequence, 1);
            assert_eq!(report.inverse_operation_count, 3);
            assert_eq!(report.preview_change_count, 3);
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
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_supports_full_file_text_inverse() {
    let root = unique_temp_dir("rollback-draft-text");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_text_edit_block(&layout);
        assert!(result.is_ok());
        let report = append_rollback_draft(&layout, "heads/main", "rollback text");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.inverse_operation_count, 1);
            assert_eq!(report.preview_change_count, 1);
            assert_eq!(report.would_replace_files, 1);
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
        let report = append_rollback_draft(&layout, "heads/main", "rollback with pending work");
        assert!(report.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}
