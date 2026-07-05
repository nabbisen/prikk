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

mod basic;
mod checked_mutation;
mod content_mutation;
mod seed;
