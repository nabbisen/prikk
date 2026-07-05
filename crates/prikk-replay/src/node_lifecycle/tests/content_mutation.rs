use super::*;

fn binary_file_node(p: &str, blob: u8, mode: u32) -> LiveNode {
    LiveNode {
        path: path(p),
        kind: NodeKind::BinaryFile,
        content: NodeContent::File {
            blob_id: oid(blob),
            mode,
        },
    }
}

#[test]
fn replace_file_blob_updates_blob_preserving_mode() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), binary_file_node("a.bin", 7, 0o100_644))
        .expect("create");
    state
        .replace_file_blob(nid(1), oid(7), oid(8))
        .expect("replace");
    match &state.live_node(&nid(1)).expect("live").content {
        NodeContent::File { blob_id, mode } => {
            assert_eq!(*blob_id, oid(8));
            assert_eq!(*mode, 0o100_644);
        }
        other => panic!("expected file, got {other:?}"),
    }
}

#[test]
fn replace_file_blob_rejects_old_blob_mismatch() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), binary_file_node("a.bin", 7, 0o100_644))
        .expect("create");
    assert!(state.replace_file_blob(nid(1), oid(99), oid(8)).is_err());
}

#[test]
fn replace_file_blob_rejects_text_file_node() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), file_node("a.txt", 7, 0o100_644))
        .expect("create");
    assert!(state.replace_file_blob(nid(1), oid(7), oid(8)).is_err());
}

#[test]
fn replace_file_blob_rejects_dead_node() {
    let mut state = NodeLifecycleState::new();
    assert!(state.replace_file_blob(nid(9), oid(7), oid(8)).is_err());
}

#[test]
fn set_text_blob_updates_text_node_blob() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), file_node("a.txt", 7, 0o100_644))
        .expect("create");
    state.set_text_blob(nid(1), oid(8)).expect("set");
    match &state.live_node(&nid(1)).expect("live").content {
        NodeContent::File { blob_id, mode } => {
            assert_eq!(*blob_id, oid(8));
            assert_eq!(*mode, 0o100_644);
        }
        other => panic!("expected file, got {other:?}"),
    }
}

#[test]
fn set_text_blob_rejects_binary_node() {
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(1), binary_file_node("a.bin", 7, 0o100_644))
        .expect("create");
    assert!(state.set_text_blob(nid(1), oid(8)).is_err());
}

#[test]
fn set_text_blob_rejects_dead_node() {
    let mut state = NodeLifecycleState::new();
    assert!(state.set_text_blob(nid(9), oid(8)).is_err());
}
