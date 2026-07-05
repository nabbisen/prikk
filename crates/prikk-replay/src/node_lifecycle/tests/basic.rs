use super::*;

#[test]
fn create_makes_node_live_and_indexed_by_path() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("fresh create");
    assert_eq!(state.live_count(), 1);
    assert_eq!(state.node_id_at(&path("a.txt")), Some(nid(0x11)));
    assert!(state.live_node(&nid(0x11)).is_some());
}

#[test]
fn create_rejects_currently_live_node_id() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("first create");
    // Same node_id, different path: per-CleanTree uniqueness violation.
    let err = state
        .create_node(nid(0x11), file_node("b.txt", 0xbb, 0o100_644))
        .expect_err("live reuse rejected");
    assert!(format!("{err:?}").contains("already live"));
    assert_eq!(state.live_count(), 1);
}

#[test]
fn create_rejects_occupied_path() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("first create");
    let err = state
        .create_node(nid(0x22), file_node("a.txt", 0xbb, 0o100_644))
        .expect_err("occupied path rejected");
    assert!(format!("{err:?}").contains("occupied"));
}

#[test]
fn delete_removes_node_from_live_and_path_index() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.delete_node(nid(0x11)).expect("delete");
    assert_eq!(state.live_count(), 0);
    assert_eq!(state.node_id_at(&path("a.txt")), None);
    assert!(state.live_node(&nid(0x11)).is_none());
}

#[test]
fn delete_then_restoration_equivalent_recreate_is_accepted() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.delete_node(nid(0x11)).expect("delete");
    // Same kind, same blob, same mode, same path: restoration-equivalent.
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("restoration-equivalent recreate");
    assert_eq!(state.live_count(), 1);
    assert_eq!(state.node_id_at(&path("a.txt")), Some(nid(0x11)));
}

#[test]
fn reintroduction_rejected_on_blob_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.delete_node(nid(0x11)).expect("delete");
    let err = state
        .create_node(nid(0x11), file_node("a.txt", 0xbb, 0o100_644))
        .expect_err("blob mismatch rejected");
    assert!(format!("{err:?}").contains("restoration-equivalent"));
}

#[test]
fn reintroduction_rejected_on_mode_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.delete_node(nid(0x11)).expect("delete");
    let err = state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_755))
        .expect_err("mode mismatch rejected");
    assert!(format!("{err:?}").contains("restoration-equivalent"));
}

#[test]
fn reintroduction_rejected_on_path_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.delete_node(nid(0x11)).expect("delete");
    let err = state
        .create_node(nid(0x11), file_node("b.txt", 0xaa, 0o100_644))
        .expect_err("path mismatch rejected");
    assert!(format!("{err:?}").contains("restoration-equivalent"));
}

#[test]
fn reintroduction_rejected_on_kind_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.delete_node(nid(0x11)).expect("delete");
    // Same id and path, but now a symlink: not restoration-equivalent.
    let err = state
        .create_node(nid(0x11), symlink_node("a.txt", "t.txt"))
        .expect_err("kind mismatch rejected");
    assert!(format!("{err:?}").contains("restoration-equivalent"));
}

#[test]
fn symlink_restoration_equivalence_matches_on_target() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), symlink_node("link", "t.txt"))
        .expect("create symlink");
    state.delete_node(nid(0x11)).expect("delete");
    state
        .create_node(nid(0x11), symlink_node("link", "t.txt"))
        .expect("restoration-equivalent symlink recreate");
    // Mismatched target is rejected.
    state.delete_node(nid(0x11)).expect("delete again");
    let err = state
        .create_node(nid(0x11), symlink_node("link", "other.txt"))
        .expect_err("target mismatch rejected");
    assert!(format!("{err:?}").contains("restoration-equivalent"));
}

#[test]
fn rename_preserves_node_id_and_moves_path() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.rename_node(nid(0x11), path("b.txt")).expect("rename");
    assert_eq!(state.node_id_at(&path("a.txt")), None);
    assert_eq!(state.node_id_at(&path("b.txt")), Some(nid(0x11)));
    let live = state.live_node(&nid(0x11)).expect("still live");
    assert_eq!(live.path.as_str(), "b.txt");
}

#[test]
fn rename_rejects_target_occupied_by_other_live_node() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create a");
    state
        .create_node(nid(0x22), file_node("b.txt", 0xbb, 0o100_644))
        .expect("create b");
    let err = state
        .rename_node(nid(0x11), path("b.txt"))
        .expect_err("occupied rename target rejected");
    assert!(format!("{err:?}").contains("occupied"));
}

#[test]
fn delete_and_rename_reject_non_live_node_id() {
    let mut state = NodeLifecycleState::new();
    let del_err = state.delete_node(nid(0x99)).expect_err("delete non-live");
    assert!(format!("{del_err:?}").contains("not live"));
    let ren_err = state
        .rename_node(nid(0x99), path("z.txt"))
        .expect_err("rename non-live");
    assert!(format!("{ren_err:?}").contains("not live"));
}

#[test]
fn create_rejects_file_kind_with_symlink_content() {
    let mut state = NodeLifecycleState::new();
    let bad = LiveNode {
        path: path("a.txt"),
        kind: NodeKind::TextFile,
        content: NodeContent::Symlink {
            target: "t.txt".to_string(),
        },
    };
    let err = state
        .create_node(nid(0x11), bad)
        .expect_err("kind/content mismatch rejected");
    assert!(format!("{err:?}").contains("discriminator mismatch"));
}

#[test]
fn create_rejects_symlink_kind_with_file_content() {
    let mut state = NodeLifecycleState::new();
    let bad = LiveNode {
        path: path("link"),
        kind: NodeKind::Symlink,
        content: NodeContent::File {
            blob_id: oid(0xaa),
            mode: 0o100_644,
        },
    };
    let err = state
        .create_node(nid(0x11), bad)
        .expect_err("kind/content mismatch rejected");
    assert!(format!("{err:?}").contains("discriminator mismatch"));
}

#[test]
fn create_accepts_binary_file_kind_with_file_content() {
    let mut state = NodeLifecycleState::new();
    let node = LiveNode {
        path: path("img.bin"),
        kind: NodeKind::BinaryFile,
        content: NodeContent::File {
            blob_id: oid(0xcc),
            mode: 0o100_644,
        },
    };
    state
        .create_node(nid(0x33), node)
        .expect("binary file accepted");
    assert_eq!(state.live_count(), 1);
}

#[test]
fn internal_consistency_holds_after_operations() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create a");
    state
        .create_node(nid(0x22), file_node("b.txt", 0xbb, 0o100_644))
        .expect("create b");
    state.rename_node(nid(0x11), path("c.txt")).expect("rename");
    state.delete_node(nid(0x22)).expect("delete b");
    state
        .validate_internal_consistency()
        .expect("bijection holds");
    assert_eq!(state.live_count(), 1);
    assert_eq!(state.node_id_at(&path("c.txt")), Some(nid(0x11)));
}
