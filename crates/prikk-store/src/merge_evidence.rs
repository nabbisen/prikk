//! Public read-only merge/conflict evidence display boundary (DC-22).

mod display;
mod merge_plan;

use std::collections::BTreeSet;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectId, ObjectType, RefStatePayload};

pub use display::{
    MergeEvidenceDisplay, MergeEvidenceDisplayItem, MergeEvidenceDisplayOperation,
    MergeEvidenceDisplaySelector,
};
pub use merge_plan::MergePlanDisplay;

use crate::lifecycle_cache::replay_derived_state;
use crate::object_store::FileObjectStore;
use crate::patch_algebra::{EvidenceScope, StorePatchAlgebraEvidence, analyze_merge_evidence};
use crate::patch_replay::decode::{DecodedPatchOperation, decode_patch_operations};
use crate::refs::RefStore;
use crate::{RepositoryLayout, validate_local_branch_ref};

/// Target selector for `prikk merge-evidence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeEvidenceTarget {
    /// Select a sealed target block directly.
    Block(ObjectId),
    /// Select the current target block of a local branch ref.
    Ref(String),
}

/// Prepare a read-only merge evidence display report.
pub fn prepare_merge_evidence(
    layout: &RepositoryLayout,
    baseline_block_id: ObjectId,
    left_target: MergeEvidenceTarget,
    right_target: MergeEvidenceTarget,
) -> Result<MergeEvidenceDisplay> {
    let object_store = FileObjectStore::new(layout.clone());
    let baseline_horizon = lineage_horizon(&object_store, baseline_block_id)?;
    let replay = replay_derived_state(&object_store, baseline_block_id, baseline_horizon)?;
    let evidence =
        StorePatchAlgebraEvidence::from_replay_derived(&object_store, baseline_horizon, replay)
            .map_err(|err| PrikkError::Integrity(format!("merge evidence baseline: {err:?}")))?;
    let left_selector = resolve_target(layout, &object_store, left_target)?;
    let right_selector = resolve_target(layout, &object_store, right_target)?;
    let left_operations = candidate_sequence(
        &object_store,
        baseline_block_id,
        left_selector.target_block_id,
    )?;
    let right_operations = candidate_sequence(
        &object_store,
        baseline_block_id,
        right_selector.target_block_id,
    )?;
    let report = analyze_merge_evidence(
        baseline_block_id,
        Some(baseline_horizon),
        evidence.baseline_state(),
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left_operations,
        &right_operations,
    );
    Ok(MergeEvidenceDisplay::from_report(
        report,
        left_selector,
        right_selector,
    ))
}

/// Prepare a read-only merge plan display report.
pub fn prepare_merge_plan(
    layout: &RepositoryLayout,
    baseline_block_id: ObjectId,
    left_target: MergeEvidenceTarget,
    right_target: MergeEvidenceTarget,
) -> Result<MergePlanDisplay> {
    let evidence = prepare_merge_evidence(layout, baseline_block_id, left_target, right_target)?;
    Ok(MergePlanDisplay::from_evidence(evidence))
}

fn resolve_target(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    target: MergeEvidenceTarget,
) -> Result<MergeEvidenceDisplaySelector> {
    match target {
        MergeEvidenceTarget::Block(block_id) => {
            read_block(object_store, block_id)?;
            Ok(MergeEvidenceDisplaySelector {
                selector: format!("block {block_id}"),
                target_block_id: block_id,
            })
        }
        MergeEvidenceTarget::Ref(ref_name) => {
            let ref_name = validate_local_branch_ref(&ref_name)?;
            let ref_store = RefStore::new(layout.clone());
            let ref_state_id = ref_store
                .read_current_ref_state_id(&ref_name)?
                .ok_or_else(|| PrikkError::Integrity(format!("ref {ref_name} is not published")))?;
            let envelope = object_store
                .read_typed(ref_state_id, ObjectType::RefState)?
                .ok_or_else(|| {
                    PrikkError::Integrity(format!("ref {ref_name} points to missing RefState"))
                })?;
            let ref_state = RefStatePayload::decode_canonical(
                &envelope.canonical_payload,
                envelope.schema_version,
            )?;
            if ref_state.ref_name != ref_name {
                return Err(PrikkError::Integrity(format!(
                    "RefState name mismatch: expected {ref_name}, got {}",
                    ref_state.ref_name
                )));
            }
            read_block(object_store, ref_state.target_object_id)?;
            Ok(MergeEvidenceDisplaySelector {
                selector: format!("ref {ref_name}"),
                target_block_id: ref_state.target_object_id,
            })
        }
    }
}

fn lineage_horizon(object_store: &FileObjectStore, baseline: ObjectId) -> Result<ObjectId> {
    let mut visited = BTreeSet::new();
    let mut current = baseline;
    loop {
        if !visited.insert(current) {
            return Err(PrikkError::Integrity(format!(
                "block parent chain contains a cycle at {current}"
            )));
        }
        let block = read_block(object_store, current)?;
        match block.parent_block_ids.as_slice() {
            [] => return Ok(current),
            [parent] => current = *parent,
            parents => {
                return Err(PrikkError::UnsupportedObjectType(format!(
                    "merge evidence requires a single-parent baseline lineage; block {current} has {} parents",
                    parents.len()
                )));
            }
        }
    }
}

fn candidate_sequence(
    object_store: &FileObjectStore,
    baseline: ObjectId,
    target: ObjectId,
) -> Result<Vec<DecodedPatchOperation>> {
    let mut newest_first = Vec::new();
    let mut visited = BTreeSet::new();
    let mut current = target;
    loop {
        if !visited.insert(current) {
            return Err(PrikkError::Integrity(format!(
                "block parent chain contains a cycle at {current}"
            )));
        }
        if current == baseline {
            break;
        }
        let block = read_block(object_store, current)?;
        let parent = match block.parent_block_ids.as_slice() {
            [parent] => *parent,
            [] => {
                return Err(PrikkError::Integrity(format!(
                    "baseline Block {baseline} is not an ancestor of target Block {target}"
                )));
            }
            parents => {
                return Err(PrikkError::UnsupportedObjectType(format!(
                    "merge evidence requires single-parent candidate chains; block {current} has {} parents",
                    parents.len()
                )));
            }
        };
        newest_first.push(block);
        current = parent;
    }
    newest_first.reverse();
    let mut operations = Vec::new();
    for block in newest_first {
        for patch_id in block.patch_ids {
            let envelope = object_store
                .read_typed(patch_id, ObjectType::Patch)?
                .ok_or_else(|| PrikkError::Integrity(format!("missing Patch {patch_id}")))?;
            operations.extend(decode_patch_operations(&envelope.canonical_payload)?);
        }
    }
    Ok(operations)
}

fn read_block(object_store: &FileObjectStore, block_id: ObjectId) -> Result<BlockPayload> {
    let envelope = object_store
        .read_typed(block_id, ObjectType::Block)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Block {block_id}")))?;
    BlockPayload::decode_canonical(&envelope.canonical_payload)
}

#[cfg(test)]
mod tests;
