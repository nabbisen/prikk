//! Format-2 Block shape and authoritative clean-state derivation.

use std::collections::BTreeSet;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockKind, BlockPayload, MerkleRoot, ObjectId, ObjectType};

use crate::lifecycle_cache::replay::replay_with_appended_patches;
use crate::object_store::ObjectReader;
use crate::state_root::{compute_state_root, entries_from_state};

/// Validate the format-2 Block kind and parent cardinality contract.
pub fn validate_block_v2_shape(payload: &BlockPayload) -> Result<()> {
    match (payload.kind, payload.parent_block_ids.as_slice()) {
        (BlockKind::Root, []) | (BlockKind::Normal, [_]) => Ok(()),
        (BlockKind::Root, _) => Err(PrikkError::Integrity(
            "format-2 Root Block must have zero parents".to_string(),
        )),
        (BlockKind::Normal, _) => Err(PrikkError::Integrity(
            "format-2 Normal Block must have exactly one parent".to_string(),
        )),
        (BlockKind::Merge | BlockKind::Repair | BlockKind::Import, _) => Err(
            PrikkError::Integrity("format-2 Block kind is not authorized".to_string()),
        ),
    }
}

/// Derive the state root for a proposed format-2 Block from its parent and ordered Patches.
pub fn derive_next_state_root(
    reader: &impl ObjectReader,
    parent: Option<ObjectId>,
    patch_ids: &[ObjectId],
) -> Result<MerkleRoot> {
    if let Some(parent) = parent {
        let lineage = validate_v2_lineage(reader, parent)?;
        verify_v2_lineage_roots(reader, &lineage)?;
    }
    let state = replay_with_appended_patches(reader, parent, patch_ids)?;
    compute_state_root(&entries_from_state(&state)?)
}

/// Recompute and compare one persisted format-2 Block's state root.
pub(crate) fn verify_block_v2_state(
    reader: &impl ObjectReader,
    block_id: ObjectId,
    payload: &BlockPayload,
) -> Result<()> {
    validate_block_v2_shape(payload)?;
    let parent = payload.parent_block_ids.first().copied();
    let computed = derive_next_state_root(reader, parent, &payload.patch_ids)?;
    if computed != payload.state_merkle_root {
        return Err(PrikkError::Integrity(format!(
            "format-2 Block {block_id} state root does not match authoritative replay"
        )));
    }
    Ok(())
}

fn validate_v2_lineage(
    reader: &impl ObjectReader,
    tip: ObjectId,
) -> Result<Vec<(ObjectId, BlockPayload)>> {
    let mut visited = BTreeSet::new();
    let mut lineage = Vec::new();
    let mut current = Some(tip);
    while let Some(block_id) = current {
        if !visited.insert(block_id) {
            return Err(PrikkError::Integrity(format!(
                "format-2 Block lineage cycle at {block_id}"
            )));
        }
        let envelope = reader.read_object(block_id)?.ok_or_else(|| {
            PrikkError::Integrity(format!("format-2 parent Block {block_id} is missing"))
        })?;
        if envelope.object_type != ObjectType::Block {
            return Err(PrikkError::ObjectTypeMismatch {
                expected: ObjectType::Block.to_string(),
                actual: envelope.object_type.to_string(),
            });
        }
        if envelope.schema_version != 2 {
            return Err(PrikkError::Integrity(format!(
                "format-2 lineage contains Block {block_id} with schema {}",
                envelope.schema_version
            )));
        }
        let payload = BlockPayload::decode_canonical(&envelope.canonical_payload)?;
        validate_block_v2_shape(&payload)?;
        current = payload.parent_block_ids.first().copied();
        lineage.push((block_id, payload));
    }
    Ok(lineage)
}

fn verify_v2_lineage_roots(
    reader: &impl ObjectReader,
    lineage_from_tip: &[(ObjectId, BlockPayload)],
) -> Result<()> {
    for (block_id, payload) in lineage_from_tip.iter().rev() {
        let parent = payload.parent_block_ids.first().copied();
        let state = replay_with_appended_patches(reader, parent, &payload.patch_ids)?;
        let computed = compute_state_root(&entries_from_state(&state)?)?;
        if computed != payload.state_merkle_root {
            return Err(PrikkError::Integrity(format!(
                "format-2 parent Block {block_id} state root does not match authoritative replay"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
