use prikk_error::{PrikkError, Result};
use prikk_object::{NodeId, NodeKind};

use super::{LiveNode, NodeContent, NodeLifecycleState, Tombstone};

impl NodeLifecycleState {
    /// Validator (erratum P2/P1-4): confirm `live_by_id` and `path_to_id` form a
    /// bijection over live nodes, and that every live node and every tombstoned node
    /// is recorded as seen (a tombstone or live entry for a never-seen node_id is a
    /// cache inconsistency). For assertions now and the 4.6 deep-verify validator.
    pub fn validate_internal_consistency(&self) -> Result<()> {
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
pub fn validate_kind_content_shape(kind: NodeKind, content: &NodeContent) -> Result<()> {
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
pub fn ensure_node_id_nonzero(node_id: NodeId) -> Result<()> {
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
pub(super) fn ensure_restoration_equivalent(tombstone: &Tombstone, node: &LiveNode) -> Result<()> {
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
