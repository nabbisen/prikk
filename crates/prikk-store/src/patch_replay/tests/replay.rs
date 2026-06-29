//! Patch replay planning tests.

use crate::test_support::{publish_snapshot_then_patch_block, unique_temp_dir};
use crate::{RepositoryLayout, prepare_patch_replay_plan};

#[test]
fn patch_replay_applies_create_delete_and_replace() {
    let root = unique_temp_dir("patch-replay");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_snapshot_then_patch_block(&layout);
        assert!(result.is_ok());
        let plan = prepare_patch_replay_plan(&layout, "heads/main");
        assert!(plan.is_ok());
        if let Ok(plan) = plan {
            assert_eq!(plan.block_count, 2);
            assert_eq!(plan.patch_count, 1);
            assert_eq!(plan.applied_operation_count, 2);
            assert_eq!(plan.file_count, 2);
            assert!(plan.paths.contains(&"README.md".to_string()));
            assert!(plan.paths.contains(&"extra.txt".to_string()));
            assert!(!plan.paths.contains(&"old.txt".to_string()));
        }
    }
    let _ = std::fs::remove_dir_all(root);
}
