use super::*;

#[test]
fn create_node_rejects_all_zero_node_id() {
    let mut state = NodeLifecycleState::new();
    let err = state
        .create_node(nid(0x00), file_node("a.txt", 0xaa, 0o100_644))
        .expect_err("all-zero node_id");
    assert!(format!("{err:?}").contains("zero"));
}

#[test]
fn change_file_mode_updates_mode_exactly() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), file_node("a.txt", 7, 0o100_644))
        .expect("create");
    state
        .change_file_mode(nid(1), 0o100_644, 0o100_755)
        .expect("chmod");
    match &state.live_node(&nid(1)).expect("live").content {
        NodeContent::File { mode, .. } => assert_eq!(*mode, 0o100_755),
        other => panic!("expected file content, got {other:?}"),
    }
}

#[test]
fn change_file_mode_rejects_old_mode_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), file_node("a.txt", 7, 0o100_644))
        .expect("create");
    assert!(
        state
            .change_file_mode(nid(1), 0o100_600, 0o100_755)
            .is_err()
    );
}

#[test]
fn change_file_mode_rejects_symlink() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(2), symlink_node("link", "target"))
        .expect("create symlink");
    assert!(state.change_file_mode(nid(2), 0, 0o100_755).is_err());
}

#[test]
fn change_file_mode_rejects_dead_node() {
    let mut state = NodeLifecycleState::new();
    assert!(
        state
            .change_file_mode(nid(9), 0o100_644, 0o100_755)
            .is_err()
    );
}

#[test]
fn delete_node_checked_rejects_preimage_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), file_node("a.txt", 7, 0o100_644))
        .expect("create");
    // Expected node claims a different mode than the live node.
    let wrong = file_node("a.txt", 7, 0o100_600);
    assert!(state.delete_node_checked(nid(1), &wrong).is_err());
    // The node must remain live after a rejected delete.
    assert!(state.live_node(&nid(1)).is_some());
}

#[test]
fn delete_node_checked_accepts_exact_preimage() {
    let mut state = NodeLifecycleState::new();
    let node = file_node("a.txt", 7, 0o100_644);
    state.create_node(nid(1), node.clone()).expect("create");
    let deleted = state.delete_node_checked(nid(1), &node).expect("delete");
    assert_eq!(deleted, node);
    assert!(state.live_node(&nid(1)).is_none());
}

#[test]
fn rename_node_checked_rejects_old_path_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), file_node("a.txt", 7, 0o100_644))
        .expect("create");
    assert!(
        state
            .rename_node_checked(nid(1), &path("wrong.txt"), path("b.txt"))
            .is_err()
    );
    // Path index unchanged after rejection.
    assert_eq!(state.node_id_at(&path("a.txt")), Some(nid(1)));
}

#[test]
fn rename_node_checked_accepts_correct_old_path() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), file_node("a.txt", 7, 0o100_644))
        .expect("create");
    state
        .rename_node_checked(nid(1), &path("a.txt"), path("b.txt"))
        .expect("rename");
    assert_eq!(state.node_id_at(&path("b.txt")), Some(nid(1)));
    assert_eq!(state.node_id_at(&path("a.txt")), None);
}
