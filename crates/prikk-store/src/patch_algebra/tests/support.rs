pub(super) use super::super::classify::{
    classify_pair as classify_pair_result,
    classify_pair_with_text_resolver as classify_pair_with_text_resolver_result,
};
pub(super) use super::super::evidence::StorePatchAlgebraEvidence;
pub(super) use super::super::evidence_types::{
    Evidence, EvidenceError, EvidenceFact, EvidenceScope, PatchAlgebraEvidence,
};
pub(super) use super::super::facts::path_effects;
pub(super) use super::super::types::{
    ConflictWitnessKind, PairClass, RequiredOrder, UnknownReason,
};
pub(super) use crate::node_lifecycle::NodeLifecycleState;
pub(super) use crate::patch_replay::decode::DecodedOperationKind;
pub(super) use prikk_object::{BlobKind, NodeKind, ObjectId};
pub(super) use std::collections::BTreeMap;

use crate::patch_replay::decode::DecodedPatchOperation;
use prikk_object::NodeId;

pub(super) const MODE_REGULAR: u32 = 0o100644;
pub(super) const MODE_EXECUTABLE: u32 = 0o100755;

pub(super) struct TestTextResolver {
    pub(super) texts: BTreeMap<NodeId, Vec<u8>>,
    blobs: BTreeMap<ObjectId, (BlobKind, Vec<u8>)>,
}

impl TestTextResolver {
    pub(super) fn empty() -> Self {
        Self {
            texts: BTreeMap::new(),
            blobs: BTreeMap::new(),
        }
    }

    pub(super) fn new(entries: impl IntoIterator<Item = (NodeId, Vec<u8>)>) -> Self {
        Self {
            texts: entries.into_iter().collect(),
            blobs: BTreeMap::new(),
        }
    }

    pub(super) fn with_blob(mut self, blob_id: ObjectId, kind: BlobKind, content: Vec<u8>) -> Self {
        self.blobs.insert(blob_id, (kind, content));
        self
    }
}

impl PatchAlgebraEvidence for TestTextResolver {
    fn baseline_text(
        &self,
        scope: EvidenceScope,
        node_id: NodeId,
        blob_id: ObjectId,
    ) -> Evidence<Vec<u8>> {
        self.texts
            .get(&node_id)
            .cloned()
            .map(Evidence::Known)
            .unwrap_or(Evidence::Missing {
                scope,
                fact: EvidenceFact::BaselineText,
                object_id: Some(blob_id),
                node_id: Some(node_id),
            })
    }

    fn blob_kind(&self, scope: EvidenceScope, blob_id: ObjectId) -> Evidence<BlobKind> {
        self.blobs
            .get(&blob_id)
            .map(|(kind, _)| Evidence::Known(*kind))
            .unwrap_or(Evidence::Missing {
                scope,
                fact: EvidenceFact::BlobKind,
                object_id: Some(blob_id),
                node_id: None,
            })
    }

    fn blob_content(
        &self,
        scope: EvidenceScope,
        blob_id: ObjectId,
    ) -> Evidence<(BlobKind, Vec<u8>)> {
        self.blobs
            .get(&blob_id)
            .cloned()
            .map(Evidence::Known)
            .unwrap_or(Evidence::Missing {
                scope,
                fact: EvidenceFact::BlobBytes,
                object_id: Some(blob_id),
                node_id: None,
            })
    }
}

pub(super) fn classify_pair(
    baseline: &NodeLifecycleState,
    left: &DecodedPatchOperation,
    right: &DecodedPatchOperation,
) -> PairClass {
    classify_pair_result(baseline, left, right).expect("classification evidence")
}

pub(super) fn classify_pair_with_text_resolver(
    baseline: &NodeLifecycleState,
    evidence: &TestTextResolver,
    left: &DecodedPatchOperation,
    right: &DecodedPatchOperation,
) -> PairClass {
    classify_pair_with_text_resolver_result(baseline, evidence, left, right)
        .expect("classification evidence")
}
