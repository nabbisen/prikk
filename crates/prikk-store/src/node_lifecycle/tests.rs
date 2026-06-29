//! Tests for the node lifecycle substrate (FDD-02 §12 / FDD-01 §7.2 rules):
//! per-CleanTree live uniqueness, live-reuse rejection, restoration-equivalence on
//! non-live reintroduction (file and symlink, with mismatch rejection), and
//! node_id preservation across rename.
#![allow(clippy::expect_used)]

use super::{LiveNode, NodeContent, NodeLifecycleState, Tombstone};

use prikk_object::{NodeId, NodeKind, ObjectId};

use crate::path::RepoPath;

fn nid(byte: u8) -> NodeId {
    NodeId::from_bytes([byte; 32])
}

fn path(value: &str) -> RepoPath {
    RepoPath::parse(value).expect("valid repo path")
}

fn oid(byte: u8) -> ObjectId {
    ObjectId::from_bytes([byte; 32])
}

fn file_node(p: &str, blob: u8, mode: u32) -> LiveNode {
    LiveNode {
        path: path(p),
        kind: NodeKind::TextFile,
        content: NodeContent::File {
            blob_id: oid(blob),
            mode,
        },
    }
}

fn symlink_node(p: &str, target: &str) -> LiveNode {
    LiveNode {
        path: path(p),
        kind: NodeKind::Symlink,
        content: NodeContent::Symlink {
            target: target.to_string(),
        },
    }
}

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

#[test]
fn create_node_rejects_all_zero_node_id() {
    let mut state = NodeLifecycleState::new();
    let err = state
        .create_node(nid(0x00), file_node("a.txt", 0xaa, 0o100_644))
        .expect_err("all-zero node_id");
    assert!(format!("{err:?}").contains("zero"));
}
