pub(super) use super::super::classify::{classify_pair, classify_pair_with_text_resolver};
pub(super) use super::super::facts::path_effects;
pub(super) use super::super::types::{
    BaselineTextResolver, ConflictWitnessKind, PairClass, RequiredOrder, UnknownReason,
};
pub(super) use crate::node_lifecycle::NodeLifecycleState;
pub(super) use crate::patch_replay::decode::DecodedOperationKind;
pub(super) use prikk_object::{BlobKind, NodeKind};
pub(super) use std::collections::BTreeMap;

use prikk_object::{NodeId, ObjectId};

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

impl BaselineTextResolver for TestTextResolver {
    fn text_content(&self, node_id: NodeId, _blob_id: ObjectId) -> Option<Vec<u8>> {
        self.texts.get(&node_id).cloned()
    }

    fn blob_kind(&self, blob_id: ObjectId) -> Option<BlobKind> {
        self.blobs.get(&blob_id).map(|(kind, _)| *kind)
    }

    fn blob_content(&self, blob_id: ObjectId) -> Option<(BlobKind, Vec<u8>)> {
        self.blobs.get(&blob_id).cloned()
    }
}
