//! Read-only inverse planning for the supported patch-operation subset.
//!
//! PR-026 deliberately keeps inverse handling non-mutating. It validates a supported single-parent
//! patch chain while replaying it, derives the inverse operation sequence for the currently
//! supported operation subset, and exposes the unsigned inverse Patch payload as planning metadata.
//! Publishing, rollback refs, conflict witnesses, and full patch algebra remain later increments.

use std::collections::BTreeMap;

use prikk_error::{PrikkError, Result};
use prikk_object::{
    CanonicalEncode, CreateFile, DeleteNode, DeleteNodePreimage, ObjectEnvelope, ObjectId,
    ObjectType, Operation, OperationKind, PatchPayload, PatchPurpose,
};

use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::patch_replay::decode::{
    DecodedDeletePreimage, DecodedOperationKind, DecodedPatchOperation, decode_patch_operations,
};

mod read;

use read::{
    current_target_block, load_snapshot_files, read_blob_bytes_with_kind, read_block, read_patch,
    single_parent_chain,
};

/// Read-only inverse plan for the supported patch-operation subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchInversePlan {
    /// Ref used as the inverse planning target.
    pub ref_name: String,
    /// Target block ID whose supported patch chain was inspected.
    pub target_block_id: ObjectId,
    /// Number of blocks walked from the root side to the target.
    pub block_count: usize,
    /// Number of patch objects inspected.
    pub patch_count: usize,
    /// Number of original supported operations validated.
    pub original_operation_count: usize,
    /// Number of inverse operations generated.
    pub inverse_operation_count: usize,
    /// Unsigned inverse Patch object ID hint.
    ///
    /// This ID is only a deterministic planning hint for the unsigned payload. It is not a
    /// published object, and it is not sufficient authorization for rollback.
    pub inverse_patch_id_hint: ObjectId,
    /// Unsigned inverse Patch payload.
    pub inverse_payload: PatchPayload,
    /// Human-readable summary of inverse operations in application order.
    pub operations: Vec<PatchInverseOperationSummary>,
}

/// Summary of one inverse operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchInverseOperationSummary {
    /// Operation sequence inside the inverse payload.
    pub op_seq: u32,
    /// Repository-relative path affected by the inverse operation.
    pub path: String,
    /// Inverse operation kind.
    pub kind: PatchInverseOperationKind,
}

/// Supported inverse operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchInverseOperationKind {
    /// Inverse operation creates a file.
    CreateFile,
    /// Inverse operation deletes a file.
    DeleteFile,
    /// Inverse operation replaces a binary blob.
    ReplaceBinary,
    /// Inverse operation performs a text edit.
    EditText,
}

impl PatchInverseOperationKind {
    /// Return a stable CLI label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CreateFile => "create-file",
            Self::DeleteFile => "delete-file",
            Self::ReplaceBinary => "replace-binary",
            Self::EditText => "edit-text",
        }
    }
}

/// Prepare an unsigned inverse Patch payload for the supported patch-operation subset.
pub fn prepare_patch_inverse_plan(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchInversePlan> {
    let object_store = FileObjectStore::new(layout.clone());
    let target_block_id = current_target_block(layout, &object_store, ref_name)?;
    let block_ids = single_parent_chain(&object_store, target_block_id)?;
    let mut files = BTreeMap::new();
    let mut inverse_operations = Vec::new();
    let mut patch_count = 0_usize;
    let mut original_operation_count = 0_usize;

    for block_id in &block_ids {
        let block = read_block(&object_store, *block_id)?;
        if let Some(snapshot_blob_ref) = block.snapshot_blob_ref {
            files = load_snapshot_files(&object_store, snapshot_blob_ref)?;
            inverse_operations.clear();
            patch_count = 0;
            original_operation_count = 0;
        }
        for patch_id in block.patch_ids {
            let patch = read_patch(&object_store, patch_id)?;
            let operations = decode_patch_operations(&patch.canonical_payload)?;
            for operation in operations {
                let inverse = derive_inverse_operation(&object_store, &mut files, operation)?;
                inverse_operations.push(inverse);
                original_operation_count += 1;
            }
            patch_count += 1;
        }
    }

    inverse_operations.reverse();
    renumber_operations(&mut inverse_operations)?;
    let summaries = summarize_operations(&inverse_operations);
    let inverse_payload = PatchPayload {
        operations: inverse_operations,
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let inverse_payload_bytes = inverse_payload.to_canonical_bytes()?;
    let inverse_patch_id_hint =
        ObjectEnvelope::unsigned(ObjectType::Patch, 1, inverse_payload_bytes).object_id();

    Ok(PatchInversePlan {
        ref_name: ref_name.to_string(),
        target_block_id,
        block_count: block_ids.len(),
        patch_count,
        original_operation_count,
        inverse_operation_count: inverse_payload.operations.len(),
        inverse_patch_id_hint,
        inverse_payload,
        operations: summaries,
    })
}

fn derive_inverse_operation(
    object_store: &FileObjectStore,
    files: &mut BTreeMap<String, Vec<u8>>,
    operation: DecodedPatchOperation,
) -> Result<Operation> {
    match operation.kind {
        DecodedOperationKind::CreateFile {
            path,
            node_id,
            blob_id,
            mode,
        } => {
            if files.contains_key(&path) {
                return Err(PrikkError::Integrity(format!(
                    "CreateFile would overwrite existing path {path}"
                )));
            }
            let (old_node_kind, bytes) = read_blob_bytes_with_kind(object_store, blob_id)?;
            files.insert(path.clone(), bytes);
            Ok(Operation {
                op_seq: 0,
                op_id: Some(format!("inverse-delete-{path}")),
                preconditions: Vec::new(),
                kind: OperationKind::DeleteNode(DeleteNode {
                    path,
                    node_id,
                    old_node_kind,
                    preimage: DeleteNodePreimage::File {
                        old_blob_id: blob_id,
                        old_mode: mode,
                    },
                }),
            })
        }
        DecodedOperationKind::DeleteNode {
            path,
            node_id,
            preimage:
                DecodedDeletePreimage::File {
                    old_node_kind,
                    old_blob_id,
                    old_mode,
                },
        } => {
            let old_bytes = files.get(&path).ok_or_else(|| {
                PrikkError::Integrity(format!("DeleteNode path is absent: {path}"))
            })?;
            crate::blob_access::ensure_blob_matches_node_kind(
                old_bytes,
                old_blob_id,
                old_node_kind,
            )?;
            files.remove(&path);
            Ok(Operation {
                op_seq: 0,
                op_id: Some(format!("inverse-create-{path}")),
                preconditions: Vec::new(),
                kind: OperationKind::CreateFile(CreateFile {
                    path,
                    node_id,
                    blob_id: old_blob_id,
                    mode: old_mode,
                }),
            })
        }
        DecodedOperationKind::EditText { .. } => Err(PrikkError::UnsupportedObjectType(
            "inverse planning for arbitrary-span EditText is deferred until direct-inverse vectors land"
                .to_string(),
        )),
        DecodedOperationKind::DeleteNode {
            preimage: DecodedDeletePreimage::Symlink { .. },
            ..
        } => Err(PrikkError::UnsupportedObjectType(
            "inverse planning for symlink DeleteNode is deferred".to_string(),
        )),
        DecodedOperationKind::ReplaceBinary { .. }
        | DecodedOperationKind::RenamePath { .. }
        | DecodedOperationKind::ChangePerm { .. }
        | DecodedOperationKind::CreateSymlink { .. } => Err(PrikkError::UnsupportedObjectType(
            "inverse planning for this node-addressed operation is deferred".to_string(),
        )),
    }
}

fn renumber_operations(operations: &mut [Operation]) -> Result<()> {
    for (index, operation) in operations.iter_mut().enumerate() {
        let next = index
            .checked_add(1)
            .ok_or_else(|| PrikkError::CanonicalEncoding("operation count overflow".to_string()))?;
        operation.op_seq = u32::try_from(next).map_err(|_| {
            PrikkError::CanonicalEncoding("operation count exceeds u32".to_string())
        })?;
    }
    Ok(())
}

fn summarize_operations(operations: &[Operation]) -> Vec<PatchInverseOperationSummary> {
    operations
        .iter()
        .map(|operation| {
            let (kind, path) = match &operation.kind {
                OperationKind::CreateFile(value) => {
                    (PatchInverseOperationKind::CreateFile, value.path.clone())
                }
                OperationKind::DeleteNode(value) => {
                    (PatchInverseOperationKind::DeleteFile, value.path.clone())
                }
                OperationKind::ReplaceBinary(_)
                | OperationKind::EditText(_)
                | OperationKind::RenamePath(_)
                | OperationKind::ChangePerm(_)
                | OperationKind::CreateSymlink(_) => {
                    unreachable!("inverse plan contains unsupported operation kind")
                }
            };
            PatchInverseOperationSummary {
                op_seq: operation.op_seq,
                path,
                kind,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
