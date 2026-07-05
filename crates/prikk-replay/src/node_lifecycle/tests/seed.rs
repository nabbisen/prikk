use super::*;

fn file_tombstone(p: &str, blob: u8, mode: u32) -> Tombstone {
    Tombstone {
        path: path(p),
        kind: NodeKind::TextFile,
        content: NodeContent::File {
            blob_id: oid(blob),
            mode,
        },
    }
}

#[test]
fn seed_live_node_builds_live_state() {
    let mut state = NodeLifecycleState::new();
    state
        .seed_live_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("seed live");
    assert_eq!(state.live_count(), 1);
    assert_eq!(state.node_id_at(&path("a.txt")), Some(nid(0x11)));
    state
        .validate_internal_consistency()
        .expect("consistent after seed");
}

#[test]
fn seeded_tombstone_accepts_restoration_equivalent_create_across_boundary() {
    // Simulates: node deleted before a snapshot, its tombstone seeded from the
    // lifecycle summary, then re-created after the boundary.
    let mut state = NodeLifecycleState::new();
    state
        .seed_tombstone(nid(0x11), file_tombstone("a.txt", 0xaa, 0o100_644))
        .expect("seed tombstone");
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("restoration-equivalent recreate accepted across boundary");
    assert_eq!(state.live_count(), 1);
}

#[test]
fn seeded_tombstone_rejects_non_equivalent_create_across_boundary() {
    let mut state = NodeLifecycleState::new();
    state
        .seed_tombstone(nid(0x11), file_tombstone("a.txt", 0xaa, 0o100_644))
        .expect("seed tombstone");
    // Same historical node_id, different blob: identity-resurrection attempt.
    let err = state
        .create_node(nid(0x11), file_node("a.txt", 0xbb, 0o100_644))
        .expect_err("non-equivalent reuse rejected across boundary");
    assert!(format!("{err:?}").contains("restoration-equivalent"));
}

#[test]
fn seed_rejects_all_zero_node_id() {
    let mut state = NodeLifecycleState::new();
    let live_err = state
        .seed_live_node(nid(0x00), file_node("a.txt", 0xaa, 0o100_644))
        .expect_err("all-zero live id rejected");
    assert!(format!("{live_err:?}").contains("all-zero"));
    let tomb_err = state
        .seed_tombstone(nid(0x00), file_tombstone("a.txt", 0xaa, 0o100_644))
        .expect_err("all-zero tombstone id rejected");
    assert!(format!("{tomb_err:?}").contains("all-zero"));
}

#[test]
fn seed_live_node_rejects_duplicate_id_and_path() {
    let mut state = NodeLifecycleState::new();
    state
        .seed_live_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("first seed");
    let dup_id = state
        .seed_live_node(nid(0x11), file_node("b.txt", 0xbb, 0o100_644))
        .expect_err("duplicate live id rejected");
    assert!(format!("{dup_id:?}").contains("duplicate live node_id"));
    let dup_path = state
        .seed_live_node(nid(0x22), file_node("a.txt", 0xcc, 0o100_644))
        .expect_err("duplicate live path rejected");
    assert!(format!("{dup_path:?}").contains("duplicate live path"));
}

#[test]
fn seed_tombstone_rejects_currently_live_node() {
    let mut state = NodeLifecycleState::new();
    state
        .seed_live_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("seed live");
    let err = state
        .seed_tombstone(nid(0x11), file_tombstone("a.txt", 0xaa, 0o100_644))
        .expect_err("tombstone for live node rejected");
    assert!(format!("{err:?}").contains("currently-live"));
}

#[test]
fn seed_rejects_kind_content_mismatch() {
    let mut state = NodeLifecycleState::new();
    let bad = LiveNode {
        path: path("a.txt"),
        kind: NodeKind::Symlink,
        content: NodeContent::File {
            blob_id: oid(0xaa),
            mode: 0o100_644,
        },
    };
    let err = state
        .seed_live_node(nid(0x11), bad)
        .expect_err("mismatched seed rejected");
    assert!(format!("{err:?}").contains("discriminator mismatch"));
}

#[test]
fn restoration_equivalent_recreate_clears_tombstone() {
    // create -> delete -> restoration-equivalent re-create must leave the node live with
    // no lingering tombstone (no node_id both live and tombstoned).
    let mut state = NodeLifecycleState::new();
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("create");
    state.delete_node(nid(0x11)).expect("delete");
    state
        .create_node(nid(0x11), file_node("a.txt", 0xaa, 0o100_644))
        .expect("restore");
    assert!(state.live_node(&nid(0x11)).is_some());
    // Passes only if the tombstone was cleared on restore (no live/tombstone overlap).
    state
        .validate_internal_consistency()
        .expect("no live/tombstone overlap after restore");
}
