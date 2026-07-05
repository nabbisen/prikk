//! Node lifecycle state for node-aware patch replay (DC-09 Phase 4.4 substrate).
//!
//! This index is **derived, rebuildable, and explicitly not a root of trust**
//! (FDD-02 §12): the authoritative artifacts are the persisted patch operations,
//! and this structure is reconstructed by replaying a block/patch sequence under
//! its parent-state context. It centralises the FDD-02 §12 / FDD-01 §7.2 node
//! rules so the replay/inverse/rollback paths cannot diverge on them:
//!
//! - per-`CleanTree` live-node uniqueness — no two live nodes share a `node_id`;
//! - reintroduction of a currently-live `node_id` is rejected;
//! - reintroduction of a non-live (previously-seen) `node_id` is accepted only when
//!   it is **restoration-equivalent** to that node's latest deletion preimage
//!   (tombstone): same kind, same content payload, same mode where applicable, and
//!   same path. Non-liveness is necessary but **not** sufficient (DC-09a §4);
//! - `node_id` is preserved across `RenamePath`.
//!
//! It is the substrate type for Phase 4.4 and is threaded into replay, inverse, and
//! rollback in the following increment (which must first settle how the clean-tree
//! baseline carries node identity — see the 4.4 threading note). Until that wiring
//! lands, the module declaration allows `dead_code`.

use std::collections::{BTreeMap, BTreeSet};

use prikk_error::{PrikkError, Result};
use prikk_object::{NodeId, NodeKind, ObjectId};

use crate::path::RepoPath;

/// Content-payload identity of a node (FDD-03 §10.2 `node_payload`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeContent {
    /// Text or binary file: blob identity plus mode bits.
    File { blob_id: ObjectId, mode: u32 },
    /// Symlink: target string (mode is normatively `u32be(0)`, FDD-03 §10.2).
    Symlink { target: String },
}

/// A node currently live in the reconstructed clean tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveNode {
    pub(crate) path: RepoPath,
    pub(crate) kind: NodeKind,
    pub(crate) content: NodeContent,
}

/// The latest deletion preimage retained per node for restoration-equivalence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tombstone {
    pub(crate) kind: NodeKind,
    pub(crate) content: NodeContent,
    pub(crate) path: RepoPath,
}

/// Replay-derived node lifecycle index. Not a root of trust (FDD-02 §12).
///
/// Live-node uniqueness is structural: `live_by_id` is keyed by `node_id`, and
/// every introduction path rejects a currently-live `node_id`, so no reconstructed
/// clean tree can hold two live nodes sharing an identity.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct NodeLifecycleState {
    live_by_id: BTreeMap<NodeId, LiveNode>,
    path_to_id: BTreeMap<RepoPath, NodeId>,
    latest_tombstone_by_id: BTreeMap<NodeId, Tombstone>,
    seen_ids: BTreeSet<NodeId>,
}

impl NodeLifecycleState {
    /// An empty lifecycle state (genesis parent-state context).
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Introduce a node (`CreateFile` / `CreateSymlink`).
    ///
    /// Rejects reuse of a currently-live `node_id`, rejects occupying an
    /// already-live path, and — for a non-live but previously-seen `node_id` —
    /// requires restoration-equivalence to that node's latest tombstone.
    pub(crate) fn create_node(&mut self, node_id: NodeId, node: LiveNode) -> Result<()> {
        // Fail closed at the central node-introduction boundary, matching seed_live_node /
        // seed_tombstone, rather than relying on decode/generator correctness.
        ensure_node_id_nonzero(node_id)?;
        // Erratum P1: fail closed at the central boundary on an inconsistent
        // kind/content discriminator, so no store path can seed a symlink-as-file
        // (or file-as-symlink) node.
        validate_kind_content_shape(node.kind, &node.content)?;
        if self.live_by_id.contains_key(&node_id) {
            return Err(PrikkError::Integrity(
                "cannot create a node whose node_id is already live (per-CleanTree uniqueness)"
                    .to_string(),
            ));
        }
        if self.path_to_id.contains_key(&node.path) {
            return Err(PrikkError::Integrity(format!(
                "cannot create node at already-occupied path {}",
                node.path.as_str()
            )));
        }
        if self.seen_ids.contains(&node_id) {
            // Non-live reintroduction: non-liveness alone is not sufficient.
            let tombstone = self.latest_tombstone_by_id.get(&node_id).ok_or_else(|| {
                PrikkError::Integrity(
                    "seen node_id has no recorded tombstone for restoration-equivalence"
                        .to_string(),
                )
            })?;
            ensure_restoration_equivalent(tombstone, &node)?;
        }
        // A restoration-equivalent reintroduction makes the node live again; its tombstone is
        // no longer the node's latest state, so clear it to keep live and tombstone disjoint
        // (no node_id may be both live and tombstoned). No-op for a fresh node_id.
        self.latest_tombstone_by_id.remove(&node_id);
        self.seen_ids.insert(node_id);
        self.path_to_id.insert(node.path.clone(), node_id);
        self.live_by_id.insert(node_id, node);
        Ok(())
    }

    /// Delete a live node (`DeleteNode`), recording its deletion preimage as the
    /// node's latest tombstone.
    pub(crate) fn delete_node(&mut self, node_id: NodeId) -> Result<LiveNode> {
        let node = self.live_by_id.remove(&node_id).ok_or_else(|| {
            PrikkError::Integrity("DeleteNode target node_id is not live".to_string())
        })?;
        // Erratum P2: live_by_id and path_to_id must stay in lockstep; refuse to
        // silently desynchronise if the path index does not point back at this node.
        match self.path_to_id.get(&node.path) {
            Some(id) if *id == node_id => {
                self.path_to_id.remove(&node.path);
            }
            Some(_) => {
                return Err(PrikkError::Integrity(
                    "path index points at a different node than the live node being deleted"
                        .to_string(),
                ));
            }
            None => {
                return Err(PrikkError::Integrity(
                    "live node has no path-index entry (internal inconsistency)".to_string(),
                ));
            }
        }
        self.latest_tombstone_by_id.insert(
            node_id,
            Tombstone {
                kind: node.kind,
                content: node.content.clone(),
                path: node.path.clone(),
            },
        );
        Ok(node)
    }

    /// Rename a live node, preserving its `node_id` (FDD-01 §7.2 / FDD-02 §12).
    pub(crate) fn rename_node(&mut self, node_id: NodeId, new_path: RepoPath) -> Result<()> {
        if let Some(existing) = self.path_to_id.get(&new_path) {
            if *existing != node_id {
                return Err(PrikkError::Integrity(format!(
                    "rename target path {} is occupied by another live node",
                    new_path.as_str()
                )));
            }
        }
        let node = self.live_by_id.get_mut(&node_id).ok_or_else(|| {
            PrikkError::Integrity("RenamePath target node_id is not live".to_string())
        })?;
        let old_path = node.path.clone();
        // Erratum P2-1: refuse to mutate if the path index does not point back at this
        // node, so a corrupted internal state is surfaced rather than silently healed.
        match self.path_to_id.get(&old_path) {
            Some(id) if *id == node_id => {}
            _ => {
                return Err(PrikkError::Integrity(
                    "path index does not point at the live node being renamed".to_string(),
                ));
            }
        }
        node.path = new_path.clone();
        self.path_to_id.remove(&old_path);
        self.path_to_id.insert(new_path, node_id);
        Ok(())
    }

    /// Delete a node, first verifying the persisted `DeleteNode` record's old-state assertion
    /// (path, kind, content/preimage) against the replayed live node (2c-2bR, P1-1). Exact replay
    /// must reject a record whose preimage disagrees with reality rather than tombstone from live
    /// state regardless. `expected` is the live node the record claims to delete.
    pub(crate) fn delete_node_checked(
        &mut self,
        node_id: NodeId,
        expected: &LiveNode,
    ) -> Result<LiveNode> {
        let live = self.live_by_id.get(&node_id).ok_or_else(|| {
            PrikkError::Integrity("DeleteNode target node_id is not live".to_string())
        })?;
        if live != expected {
            return Err(PrikkError::Integrity(
                "DeleteNode preimage (path/kind/content) does not match the replayed live node"
                    .to_string(),
            ));
        }
        self.delete_node(node_id)
    }

    /// Rename a node, first verifying the persisted `RenamePath` record's `old_path` against the
    /// replayed live node's current path (2c-2bR, P1-2), then renaming to `new_path`.
    pub(crate) fn rename_node_checked(
        &mut self,
        node_id: NodeId,
        expected_old_path: &RepoPath,
        new_path: RepoPath,
    ) -> Result<()> {
        let live = self.live_by_id.get(&node_id).ok_or_else(|| {
            PrikkError::Integrity("RenamePath target node_id is not live".to_string())
        })?;
        if live.path != *expected_old_path {
            return Err(PrikkError::Integrity(format!(
                "RenamePath old_path {} does not match the live path {}",
                expected_old_path.as_str(),
                live.path.as_str()
            )));
        }
        self.rename_node(node_id, new_path)
    }

    /// The live node for an identity, if any.
    pub(crate) fn live_node(&self, node_id: &NodeId) -> Option<&LiveNode> {
        self.live_by_id.get(node_id)
    }

    /// True if `node_id` is in the complete known-id set (live, tombstoned, or otherwise seen in
    /// this baseline). Fresh node-id minting rejects any candidate for which this holds, so a
    /// random draw can never alias an existing node identity (4.4a-1, erratum E2).
    pub(crate) fn contains_seen_node_id(&self, node_id: &NodeId) -> bool {
        self.seen_ids.contains(node_id)
    }

    /// Iterate the live clean-tree nodes as `(node_id, node)`. Used by worktree authoring to build
    /// the baseline `path → (node_id, kind, content)` view (4.4a-2).
    pub(crate) fn live_nodes(&self) -> impl Iterator<Item = (&NodeId, &LiveNode)> {
        self.live_by_id.iter()
    }

    /// Apply a `ChangePerm` to a live file node, preserving its `node_id` and path. The mode is
    /// recorded exactly (O1: the lifecycle index must carry post-mutation mode, since a later
    /// deletion's tombstone — and §10.2 `EntryHash` — bind it). Fails closed if the node is not
    /// live, is a symlink (whose mode is normatively zero), or if the stated `old_mode` does not
    /// match the replayed mode.
    pub(crate) fn change_file_mode(
        &mut self,
        node_id: NodeId,
        old_mode: u32,
        new_mode: u32,
    ) -> Result<()> {
        let node = self.live_by_id.get_mut(&node_id).ok_or_else(|| {
            PrikkError::Integrity("ChangePerm target node_id is not live".to_string())
        })?;
        match &mut node.content {
            NodeContent::File { mode, .. } => {
                if *mode != old_mode {
                    return Err(PrikkError::Integrity(format!(
                        "ChangePerm old_mode {old_mode:#o} does not match the live mode {:#o}",
                        *mode
                    )));
                }
                *mode = new_mode;
                Ok(())
            }
            NodeContent::Symlink { .. } => Err(PrikkError::Integrity(
                "ChangePerm cannot apply to a symlink node (mode is normatively zero)".to_string(),
            )),
        }
    }

    /// Apply a `ReplaceBinary` to a live binary-file node (2c-2c). Requires the node to be live,
    /// a `BinaryFile`, and to currently reference `old_blob_id`; replaces its blob with
    /// `new_blob_id` exactly, preserving mode. Text files are out of scope (that is `EditText`).
    pub(crate) fn replace_file_blob(
        &mut self,
        node_id: NodeId,
        old_blob_id: ObjectId,
        new_blob_id: ObjectId,
    ) -> Result<()> {
        let node = self.live_by_id.get_mut(&node_id).ok_or_else(|| {
            PrikkError::Integrity("ReplaceBinary target node_id is not live".to_string())
        })?;
        if node.kind != NodeKind::BinaryFile {
            return Err(PrikkError::Integrity(
                "ReplaceBinary target is not a binary-file node".to_string(),
            ));
        }
        match &mut node.content {
            NodeContent::File { blob_id, .. } => {
                if *blob_id != old_blob_id {
                    return Err(PrikkError::Integrity(
                        "ReplaceBinary old_blob_id does not match the live blob".to_string(),
                    ));
                }
                *blob_id = new_blob_id;
                Ok(())
            }
            NodeContent::Symlink { .. } => Err(PrikkError::Integrity(
                "ReplaceBinary cannot apply to a symlink node".to_string(),
            )),
        }
    }

    /// Set a live text-file node's content blob id (2c-2d). `EditText` replay derives a new
    /// full-text `BlobPayload(Text, …)` id and records it here, preserving `node_id`, path, and
    /// mode. The new bytes themselves live in the replay-local materialized-text cache, not the
    /// index. Fails closed if the node is absent or not a `TextFile`.
    pub(crate) fn set_text_blob(&mut self, node_id: NodeId, new_blob_id: ObjectId) -> Result<()> {
        let node = self.live_by_id.get_mut(&node_id).ok_or_else(|| {
            PrikkError::Integrity("EditText target node_id is not live".to_string())
        })?;
        if node.kind != NodeKind::TextFile {
            return Err(PrikkError::Integrity(
                "EditText target is not a text-file node".to_string(),
            ));
        }
        match &mut node.content {
            NodeContent::File { blob_id, .. } => {
                *blob_id = new_blob_id;
                Ok(())
            }
            NodeContent::Symlink { .. } => Err(PrikkError::Integrity(
                "EditText cannot apply to a symlink node".to_string(),
            )),
        }
    }

    /// The `node_id` currently live at a path, if any. (Production consumer is 4.4a-2 worktree
    /// authoring, which resolves an existing path to its node id here.)
    #[cfg(test)]
    pub(crate) fn node_id_at(&self, path: &RepoPath) -> Option<NodeId> {
        self.path_to_id.get(path).copied()
    }

    /// The latest deletion preimage retained for an identity, if the node is tombstoned.
    #[cfg(test)]
    pub(crate) fn latest_tombstone(&self, node_id: &NodeId) -> Option<&Tombstone> {
        self.latest_tombstone_by_id.get(node_id)
    }

    /// Number of live nodes in the reconstructed clean tree.
    #[cfg(test)]
    pub(crate) fn live_count(&self) -> usize {
        self.live_by_id.len()
    }

    /// Seed a live clean-tree node from a baseline cache (erratum P1, 4.4-2).
    ///
    /// Reuses the same gates as a fresh `create_node` so a cache cannot inject what
    /// an operation could not: rejects the all-zero `node_id`, an inconsistent
    /// kind/content shape, a duplicate live `node_id`, and an occupied path. (The
    /// symlink `normalized_mode == 0` rule is enforced at the cache-parse boundary,
    /// before a `NodeContent::Symlink` — which carries no mode — is constructed.)
    #[cfg(test)]
    pub(crate) fn seed_live_node(&mut self, node_id: NodeId, node: LiveNode) -> Result<()> {
        ensure_node_id_nonzero(node_id)?;
        validate_kind_content_shape(node.kind, &node.content)?;
        if self.live_by_id.contains_key(&node_id) {
            return Err(PrikkError::Integrity(
                "baseline seeds a duplicate live node_id".to_string(),
            ));
        }
        if self.path_to_id.contains_key(&node.path) {
            return Err(PrikkError::Integrity(format!(
                "baseline seeds a duplicate live path {}",
                node.path.as_str()
            )));
        }
        self.seen_ids.insert(node_id);
        self.path_to_id.insert(node.path.clone(), node_id);
        self.live_by_id.insert(node_id, node);
        Ok(())
    }

    /// Seed the latest deletion preimage for a previously-seen, non-live node from a
    /// baseline lifecycle summary (erratum P1, 4.4-2). Records `seen_ids` so the
    /// historical-reintroduction rule applies across the snapshot boundary.
    #[cfg(test)]
    pub(crate) fn seed_tombstone(&mut self, node_id: NodeId, tombstone: Tombstone) -> Result<()> {
        ensure_node_id_nonzero(node_id)?;
        validate_kind_content_shape(tombstone.kind, &tombstone.content)?;
        if self.live_by_id.contains_key(&node_id) {
            return Err(PrikkError::Integrity(
                "baseline seeds a tombstone for a currently-live node_id".to_string(),
            ));
        }
        self.seen_ids.insert(node_id);
        self.latest_tombstone_by_id.insert(node_id, tombstone);
        Ok(())
    }

    /// Validator (erratum P2/P1-4): confirm `live_by_id` and `path_to_id` form a
    /// bijection over live nodes, and that every live node and every tombstoned node
    /// is recorded as seen (a tombstone or live entry for a never-seen node_id is a
    /// cache inconsistency). For assertions now and the 4.6 deep-verify validator.
    pub(crate) fn validate_internal_consistency(&self) -> Result<()> {
        if self.live_by_id.len() != self.path_to_id.len() {
            return Err(PrikkError::Integrity(
                "node lifecycle index: live_by_id and path_to_id differ in size".to_string(),
            ));
        }
        for (node_id, node) in &self.live_by_id {
            match self.path_to_id.get(&node.path) {
                Some(id) if id == node_id => {}
                _ => {
                    return Err(PrikkError::Integrity(
                        "node lifecycle index: live node has no matching path-index entry"
                            .to_string(),
                    ));
                }
            }
            if !self.seen_ids.contains(node_id) {
                return Err(PrikkError::Integrity(
                    "node lifecycle index: live node_id is not recorded as seen".to_string(),
                ));
            }
        }
        for node_id in self.latest_tombstone_by_id.keys() {
            if self.live_by_id.contains_key(node_id) {
                return Err(PrikkError::Integrity(
                    "node lifecycle index: node_id is both live and tombstoned".to_string(),
                ));
            }
            if !self.seen_ids.contains(node_id) {
                return Err(PrikkError::Integrity(
                    "node lifecycle index: tombstoned node_id is not recorded as seen".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Structural kind/content discriminator (erratum P1/P2): no node or tombstone may
/// hold a payload whose kind disagrees with its content. Shared by `create_node`,
/// `seed_live_node`, and `seed_tombstone` so the discriminator cannot diverge. Blob-
/// kind matching (`TextFile -> Text`, `BinaryFile -> Binary`) stays at the blob-
/// resolution boundary, since this substrate stores only `blob_id`.
pub(crate) fn validate_kind_content_shape(kind: NodeKind, content: &NodeContent) -> Result<()> {
    let consistent = matches!(
        (kind, content),
        (
            NodeKind::TextFile | NodeKind::BinaryFile,
            NodeContent::File { .. }
        ) | (NodeKind::Symlink, NodeContent::Symlink { .. })
    );
    if consistent {
        Ok(())
    } else {
        Err(PrikkError::Integrity(
            "node kind/content discriminator mismatch (kind does not match payload)".to_string(),
        ))
    }
}

/// Reject the reserved all-zero `node_id` at every seeding boundary (erratum P1-3),
/// matching the object decoder's `NodeId::try_from_bytes` rule.
pub(crate) fn ensure_node_id_nonzero(node_id: NodeId) -> Result<()> {
    if node_id.as_bytes() == &[0_u8; 32] {
        return Err(PrikkError::Integrity(
            "baseline seed carries the reserved all-zero node_id".to_string(),
        ));
    }
    Ok(())
}

/// Restoration-equivalence (DC-09a §4, FDD-02 §12): a reintroduced non-live node
/// must match its latest tombstone in kind, content payload, mode (where
/// applicable, carried inside [`NodeContent::File`]), and path.
fn ensure_restoration_equivalent(tombstone: &Tombstone, node: &LiveNode) -> Result<()> {
    if tombstone.kind != node.kind
        || tombstone.content != node.content
        || tombstone.path != node.path
    {
        return Err(PrikkError::Integrity(
            "reintroduced node_id is not restoration-equivalent to its latest tombstone"
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
