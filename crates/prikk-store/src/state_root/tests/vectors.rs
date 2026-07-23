use prikk_hash::{sha256, to_hex};
use prikk_object::{NodeId, NodeKind, ObjectId};

use super::super::{
    NODE_DOMAIN, StateRootContent, StateRootEntry, compute_state_root, state_leaf_hash,
    state_leaf_preimage,
};
use crate::path::RepoPath;

const PREIMAGE_1: &str = concat!(
    "5052494b4b2d53544154452d4c4541462d763200000005612e747874",
    "0101010101010101010101010101010101010101010101010101010101010101",
    "0001000081a40000000000000020",
    "1111111111111111111111111111111111111111111111111111111111111111"
);
const PREIMAGE_2: &str = concat!(
    "5052494b4b2d53544154452d4c4541462d76320000000862696e2f746f6f6c",
    "0202020202020202020202020202020202020202020202020202020202020202",
    "0002000081ed0000000000000020",
    "2222222222222222222222222222222222222222222222222222222222222222"
);
const PREIMAGE_3: &str = concat!(
    "5052494b4b2d53544154452d4c4541462d7632000000046c696e6b",
    "0303030303030303030303030303030303030303030303030303030303030303",
    "00030000000000000000000000042e2e2f61"
);
const LEAF_1: &str = "80962c43625664183297a86af5a02b17e9a4c469ece220834414bfbbe580a266";
const LEAF_2: &str = "2263ed30af96cdcf49eb2275eaee15fc14286733f10f2855f12e74acd8fa7389";
const LEAF_3: &str = "fd8dcf58e7c2627967c49f891bd2aa42d0a80d4fefb7d3fecaf2cc7c5441d617";
const NODE_12: &str = "c5b85bd6adb13c17028ac092e3d7680cc465cefaee29780040296f94ba21f001";
const EMPTY_ROOT: &str = "b0fb79bf047ff6ed385f25a35ac9318e9c69152e1213ecc419f4aaad558a54b3";
const ONE_ROOT: &str = "1384dbfcfc5198c3294a7606848653df12b4a494dab5194544cb7ad49de4674b";
const TWO_ROOT: &str = "04c42752953cb7b510935a74b0a23dedcf82037b0f2eaa3aa0fbaeec65ae584b";
const THREE_ROOT: &str = "3e9944796035bb706653da6fb41c6f0883748754cb0ff0c705c1505939f75bd4";

#[test]
fn accepted_literal_preimages_and_leaf_hashes_are_stable() -> prikk_error::Result<()> {
    for (entry, preimage, leaf) in [
        (text_entry()?, PREIMAGE_1, LEAF_1),
        (binary_entry()?, PREIMAGE_2, LEAF_2),
        (symlink_entry()?, PREIMAGE_3, LEAF_3),
    ] {
        assert_eq!(to_hex(&state_leaf_preimage(&entry)?), preimage);
        assert_eq!(to_hex(&state_leaf_hash(&entry)?), leaf);
    }
    Ok(())
}

#[test]
fn accepted_empty_and_odd_even_reduction_roots_are_stable() -> prikk_error::Result<()> {
    let entries = [text_entry()?, binary_entry()?, symlink_entry()?];
    assert_eq!(to_hex(&compute_state_root(&[])?.0), EMPTY_ROOT);
    assert_eq!(to_hex(&compute_state_root(&entries[..1])?.0), ONE_ROOT);
    assert_eq!(to_hex(&compute_state_root(&entries[..2])?.0), TWO_ROOT);
    assert_eq!(to_hex(&compute_state_root(&entries)?.0), THREE_ROOT);

    let mut node_preimage = Vec::from(NODE_DOMAIN);
    node_preimage.extend_from_slice(&state_leaf_hash(&entries[0])?);
    node_preimage.extend_from_slice(&state_leaf_hash(&entries[1])?);
    assert_eq!(to_hex(&sha256(&node_preimage)), NODE_12);
    Ok(())
}

pub(super) fn text_entry() -> prikk_error::Result<StateRootEntry> {
    file_entry("a.txt", 1, NodeKind::TextFile, 0o100644, 0x11)
}

pub(super) fn binary_entry() -> prikk_error::Result<StateRootEntry> {
    file_entry("bin/tool", 2, NodeKind::BinaryFile, 0o100755, 0x22)
}

pub(super) fn symlink_entry() -> prikk_error::Result<StateRootEntry> {
    Ok(StateRootEntry {
        path: RepoPath::parse("link")?,
        node_id: NodeId::from_bytes([3; 32]),
        kind: NodeKind::Symlink,
        mode: 0,
        content: StateRootContent::Symlink("../a".to_string()),
    })
}

fn file_entry(
    path: &str,
    node: u8,
    kind: NodeKind,
    mode: u32,
    blob: u8,
) -> prikk_error::Result<StateRootEntry> {
    Ok(StateRootEntry {
        path: RepoPath::parse(path)?,
        node_id: NodeId::from_bytes([node; 32]),
        kind,
        mode,
        content: StateRootContent::Blob(ObjectId::from_bytes([blob; 32])),
    })
}
