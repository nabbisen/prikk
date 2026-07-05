use std::collections::BTreeSet;

use prikk_object::{BlobKind, NodeId, NodeKind, ObjectId};

use crate::path::RepoPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PairClass {
    Independent,
    OrderedDependency {
        required_order: RequiredOrder,
        witness: ConflictWitness,
    },
    Conflict {
        witness: ConflictWitness,
    },
    Unknown {
        reason: UnknownReason,
        witness: ConflictWitness,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequiredOrder {
    LeftBeforeRight,
    RightBeforeLeft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConflictWitness {
    pub(crate) kind: ConflictWitnessKind,
    pub(crate) left_op_seq: u32,
    pub(crate) right_op_seq: u32,
    pub(crate) node_id: Option<NodeId>,
    pub(crate) path: Option<RepoPath>,
    pub(crate) expected: Option<String>,
    pub(crate) actual: Option<String>,
    pub(crate) text_span: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConflictWitnessKind {
    SamePathCreate,
    NodeIdReuse,
    LiveStateMismatch,
    KindMismatch,
    ModeMismatch,
    BlobMismatch,
    TextSpanOverlap,
    TextAnchorStale,
    DeleteMutationConflict,
    UnsupportedOperation,
    MalformedOperation,
    UnknownRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnknownReason {
    MalformedOperation,
    SameNodeTextCommutationDeferred,
    RenameDeferred,
    SymlinkDeferred,
    FuturePreconditionDeferred,
    UnknownRelation,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct PathEffects {
    pub(crate) occupied_before: BTreeSet<RepoPath>,
    pub(crate) required_free: BTreeSet<RepoPath>,
    pub(crate) occupied_after: BTreeSet<RepoPath>,
    pub(crate) freed: BTreeSet<RepoPath>,
    pub(crate) newly_occupied: BTreeSet<RepoPath>,
}

pub(crate) trait BaselineTextResolver {
    fn text_content(&self, node_id: NodeId, blob_id: ObjectId) -> Option<Vec<u8>>;

    fn blob_kind(&self, _blob_id: ObjectId) -> Option<BlobKind> {
        None
    }

    fn blob_content(&self, _blob_id: ObjectId) -> Option<(BlobKind, Vec<u8>)> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OperationFacts {
    pub(super) op_seq: u32,
    pub(super) node_id: Option<NodeId>,
    pub(super) action: Action,
    pub(super) path_effects: PathEffects,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Action {
    CreateFile {
        path: RepoPath,
        node_id: NodeId,
        blob_id: ObjectId,
        mode: u32,
    },
    DeleteFile {
        path: RepoPath,
        node_id: NodeId,
        old_node_kind: NodeKind,
        old_blob_id: prikk_object::ObjectId,
        old_mode: u32,
    },
    DeleteSymlink {
        path: RepoPath,
        node_id: NodeId,
    },
    EditText {
        node_id: NodeId,
        span_id: [u8; 32],
        old_span_hash: [u8; 32],
        left_anchor_hash: [u8; 32],
        right_anchor_hash: [u8; 32],
        old_span_text: Vec<u8>,
    },
    ReplaceBinary {
        node_id: NodeId,
        old_blob_id: prikk_object::ObjectId,
        new_blob_id: prikk_object::ObjectId,
    },
    RenamePath {
        node_id: NodeId,
    },
    ChangePerm {
        node_id: NodeId,
        old_mode: u32,
        new_mode: u32,
    },
    CreateSymlink {
        path: RepoPath,
        node_id: NodeId,
    },
}

pub(super) struct NoBaselineTextResolver;

impl BaselineTextResolver for NoBaselineTextResolver {
    fn text_content(&self, _node_id: NodeId, _blob_id: ObjectId) -> Option<Vec<u8>> {
        None
    }
}
