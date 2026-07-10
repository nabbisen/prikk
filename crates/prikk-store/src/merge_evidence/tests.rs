//! Merge-evidence store boundary tests.

use prikk_error::Result;
use prikk_object::{
    BlobKind, BlobPayload, BlockKind, CanonicalEncode, CreateFile, NodeId, ObjectEnvelope,
    ObjectId, ObjectType, Operation, OperationKind, PatchPayload, PatchPurpose,
};

use super::{MergeEvidenceTarget, prepare_merge_evidence};
use crate::test_support::{
    dummy_signature, maintainer_signature, signed_block, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::{FileObjectStore, ObjectWriter, RefPublication, RefStore, RepositoryLayout};

#[test]
fn block_targets_report_confluent_for_independent_create_sequences() -> Result<()> {
    let root = unique_temp_dir("merge-evidence-block-targets");
    let layout = RepositoryLayout::init(root.clone())?;
    let baseline = write_block(&layout, BlockKind::Root, Vec::new(), Vec::new())?;
    let left = write_create_block(&layout, BlockKind::Normal, vec![baseline], "left.txt", 0x21)?;
    let right = write_create_block(
        &layout,
        BlockKind::Normal,
        vec![baseline],
        "right.txt",
        0x22,
    )?;

    let report = prepare_merge_evidence(
        &layout,
        baseline,
        MergeEvidenceTarget::Block(left),
        MergeEvidenceTarget::Block(right),
    )?;

    assert_eq!(report.outcome, "Confluent");
    assert_eq!(report.reason, Some("proven_confluent"));
    assert_eq!(report.left_operation_count, 1);
    assert_eq!(report.right_operation_count, 1);
    assert_eq!(report.left_selector.target_block_id, left);
    assert_eq!(report.right_selector.target_block_id, right);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn ref_target_reports_selector_and_resolved_block() -> Result<()> {
    let root = unique_temp_dir("merge-evidence-ref-target");
    let layout = RepositoryLayout::init(root.clone())?;
    let baseline = write_block(&layout, BlockKind::Root, Vec::new(), Vec::new())?;
    let left = write_create_block(&layout, BlockKind::Normal, vec![baseline], "left.txt", 0x23)?;
    publish_ref(&layout, "heads/left", left)?;

    let report = prepare_merge_evidence(
        &layout,
        baseline,
        MergeEvidenceTarget::Ref("heads/left".to_string()),
        MergeEvidenceTarget::Block(baseline),
    )?;

    assert_eq!(report.left_selector.selector, "ref heads/left");
    assert_eq!(report.left_selector.target_block_id, left);
    assert_eq!(report.right_operation_count, 0);
    assert!(matches!(
        report.outcome,
        "Confluent" | "Unsupported" | "InvalidCandidate"
    ));
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn empty_sequences_are_report_level_success() -> Result<()> {
    let root = unique_temp_dir("merge-evidence-empty-sequences");
    let layout = RepositoryLayout::init(root.clone())?;
    let baseline = write_block(&layout, BlockKind::Root, Vec::new(), Vec::new())?;

    let report = prepare_merge_evidence(
        &layout,
        baseline,
        MergeEvidenceTarget::Block(baseline),
        MergeEvidenceTarget::Block(baseline),
    )?;

    assert_eq!(report.outcome, "Confluent");
    assert_eq!(report.reason, Some("proven_confluent"));
    assert_eq!(report.left_operation_count, 0);
    assert_eq!(report.right_operation_count, 0);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn missing_ancestry_fails_before_report() -> Result<()> {
    let root = unique_temp_dir("merge-evidence-missing-ancestry");
    let layout = RepositoryLayout::init(root.clone())?;
    let baseline = write_block(&layout, BlockKind::Root, Vec::new(), Vec::new())?;
    let other_root = write_create_block(&layout, BlockKind::Root, Vec::new(), "other.txt", 0x24)?;

    let err = match prepare_merge_evidence(
        &layout,
        baseline,
        MergeEvidenceTarget::Block(other_root),
        MergeEvidenceTarget::Block(baseline),
    ) {
        Ok(_) => panic!("missing ancestry unexpectedly succeeded"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("is not an ancestor"));
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

fn write_create_block(
    layout: &RepositoryLayout,
    kind: BlockKind,
    parents: Vec<ObjectId>,
    path: &str,
    node_byte: u8,
) -> Result<ObjectId> {
    let mut store = FileObjectStore::new(layout.clone());
    let blob = BlobPayload::new(BlobKind::Text, format!("{path}\n").into_bytes());
    let blob_bytes = blob.to_canonical_bytes()?;
    let blob_id = ObjectId::from_canonical_payload(ObjectType::Blob, 1, &blob_bytes);
    let mut blob_env = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob_bytes);
    blob_env.add_signature(maintainer_signature())?;
    store.write_object(&blob_env)?;

    let patch = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: path.to_string(),
                node_id: NodeId::from_bytes([node_byte; 32]),
                blob_id,
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut patch_env = ObjectEnvelope::unsigned(ObjectType::Patch, 1, patch.to_canonical_bytes()?);
    patch_env.add_signature(dummy_signature())?;
    let patch_id = store.write_object(&patch_env)?;
    write_block(layout, kind, parents, vec![patch_id])
}

fn write_block(
    layout: &RepositoryLayout,
    kind: BlockKind,
    parents: Vec<ObjectId>,
    patches: Vec<ObjectId>,
) -> Result<ObjectId> {
    let mut store = FileObjectStore::new(layout.clone());
    let block = signed_block(kind, parents, patches, None);
    store.write_object(&block)
}

fn publish_ref(layout: &RepositoryLayout, ref_name: &str, block_id: ObjectId) -> Result<()> {
    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope(ref_name, None, block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope(ref_name, None, ref_state_id, block_id, 1);
    ref_store
        .publish(&RefPublication {
            ref_name: ref_name.to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        })
        .map(|_| ())
}
