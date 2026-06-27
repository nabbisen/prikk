//! Rollback preview tests.

use crate::{RepositoryLayout, RollbackPreviewChangeKind, prepare_rollback_preview};

use super::helpers::unique_temp_dir;
use super::patch_replay::{publish_snapshot_then_patch_block, publish_text_edit_block};

#[test]
fn rollback_preview_reports_file_level_changes_to_snapshot_baseline() {
    let root = unique_temp_dir("rollback-preview-file-ops");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_snapshot_then_patch_block(&layout);
        assert!(result.is_ok());
        let preview = prepare_rollback_preview(&layout, "heads/main");
        assert!(preview.is_ok());
        if let Ok(preview) = preview {
            assert_eq!(preview.block_count, 2);
            assert_eq!(preview.patch_count, 1);
            assert_eq!(preview.inverse_operation_count, 3);
            assert_eq!(preview.current_file_count, 2);
            assert_eq!(preview.preview_file_count, 2);
            assert_eq!(preview.change_count, 3);
            assert_eq!(preview.would_create_files, 1);
            assert_eq!(preview.would_delete_files, 1);
            assert_eq!(preview.would_replace_files, 1);
            let changes: Vec<(&str, RollbackPreviewChangeKind)> = preview
                .changes
                .iter()
                .map(|change| (change.path.as_str(), change.kind))
                .collect();
            assert_eq!(
                changes,
                vec![
                    ("README.md", RollbackPreviewChangeKind::WouldReplace),
                    ("extra.txt", RollbackPreviewChangeKind::WouldDelete),
                    ("old.txt", RollbackPreviewChangeKind::WouldCreate),
                ]
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_preview_reports_full_file_text_change() {
    let root = unique_temp_dir("rollback-preview-edit-text");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_text_edit_block(&layout);
        assert!(result.is_ok());
        let preview = prepare_rollback_preview(&layout, "heads/main");
        assert!(preview.is_ok());
        if let Ok(preview) = preview {
            assert_eq!(preview.inverse_operation_count, 1);
            assert_eq!(preview.current_file_count, 1);
            assert_eq!(preview.preview_file_count, 1);
            assert_eq!(preview.current_content_bytes, 12);
            assert_eq!(preview.preview_content_bytes, 11);
            assert_eq!(preview.change_count, 1);
            let mut changes = preview.changes.iter();
            if let Some(change) = changes.next() {
                assert_eq!(change.path, "note.txt");
                assert_eq!(change.kind, RollbackPreviewChangeKind::WouldReplace);
                assert_eq!(change.current_bytes, Some(12));
                assert_eq!(change.preview_bytes, Some(11));
            } else {
                panic!("expected one preview change");
            }
            assert!(changes.next().is_none());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}
