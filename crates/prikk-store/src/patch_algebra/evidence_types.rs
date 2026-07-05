use prikk_object::{BlobKind, NodeId, ObjectId, ObjectType};

use super::types::PairClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceScope {
    SealedBaselineRequired,
    SealedCandidateRequired,
    UnsealedCandidateOptional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceFact {
    BaselineState,
    BlobKind,
    BlobBytes,
    BaselineText,
    Operation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Evidence<T> {
    Known(T),
    Missing {
        scope: EvidenceScope,
        fact: EvidenceFact,
        object_id: Option<ObjectId>,
        node_id: Option<NodeId>,
    },
    WrongObjectType {
        scope: EvidenceScope,
        object_id: ObjectId,
        expected: ObjectType,
        actual: ObjectType,
    },
    WrongBlobKind {
        scope: EvidenceScope,
        blob_id: ObjectId,
        expected: BlobKind,
        actual: BlobKind,
    },
    Malformed {
        scope: EvidenceScope,
        fact: EvidenceFact,
        object_id: Option<ObjectId>,
        reason: String,
    },
    Unreadable {
        scope: EvidenceScope,
        fact: EvidenceFact,
        object_id: Option<ObjectId>,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvidenceError {
    Missing {
        scope: EvidenceScope,
        fact: EvidenceFact,
        object_id: Option<ObjectId>,
        node_id: Option<NodeId>,
    },
    WrongObjectType {
        scope: EvidenceScope,
        object_id: ObjectId,
        expected: ObjectType,
        actual: ObjectType,
    },
    WrongBlobKind {
        scope: EvidenceScope,
        blob_id: ObjectId,
        expected: BlobKind,
        actual: BlobKind,
    },
    Malformed {
        scope: EvidenceScope,
        fact: EvidenceFact,
        object_id: Option<ObjectId>,
        reason: String,
    },
    Unreadable {
        scope: EvidenceScope,
        fact: EvidenceFact,
        object_id: Option<ObjectId>,
        reason: String,
    },
}

pub(crate) type ClassificationResult = std::result::Result<PairClass, EvidenceError>;

impl<T> Evidence<T> {
    pub(crate) fn into_error(self) -> EvidenceError {
        match self {
            Self::Known(_) => EvidenceError::Malformed {
                scope: EvidenceScope::UnsealedCandidateOptional,
                fact: EvidenceFact::BlobBytes,
                object_id: None,
                reason: "internal evidence conversion received known evidence".to_string(),
            },
            Self::Missing {
                scope,
                fact,
                object_id,
                node_id,
            } => EvidenceError::Missing {
                scope,
                fact,
                object_id,
                node_id,
            },
            Self::WrongObjectType {
                scope,
                object_id,
                expected,
                actual,
            } => EvidenceError::WrongObjectType {
                scope,
                object_id,
                expected,
                actual,
            },
            Self::WrongBlobKind {
                scope,
                blob_id,
                expected,
                actual,
            } => EvidenceError::WrongBlobKind {
                scope,
                blob_id,
                expected,
                actual,
            },
            Self::Malformed {
                scope,
                fact,
                object_id,
                reason,
            } => EvidenceError::Malformed {
                scope,
                fact,
                object_id,
                reason,
            },
            Self::Unreadable {
                scope,
                fact,
                object_id,
                reason,
            } => EvidenceError::Unreadable {
                scope,
                fact,
                object_id,
                reason,
            },
        }
    }
}

pub(crate) trait PatchAlgebraEvidence {
    fn baseline_text(
        &self,
        scope: EvidenceScope,
        node_id: NodeId,
        blob_id: ObjectId,
    ) -> Evidence<Vec<u8>>;

    fn blob_kind(&self, scope: EvidenceScope, blob_id: ObjectId) -> Evidence<BlobKind>;

    fn blob_content(
        &self,
        scope: EvidenceScope,
        blob_id: ObjectId,
    ) -> Evidence<(BlobKind, Vec<u8>)>;
}

pub(super) struct NoPatchAlgebraEvidence;

impl PatchAlgebraEvidence for NoPatchAlgebraEvidence {
    fn baseline_text(
        &self,
        scope: EvidenceScope,
        node_id: NodeId,
        blob_id: ObjectId,
    ) -> Evidence<Vec<u8>> {
        Evidence::Missing {
            scope,
            fact: EvidenceFact::BaselineText,
            object_id: Some(blob_id),
            node_id: Some(node_id),
        }
    }

    fn blob_kind(&self, scope: EvidenceScope, blob_id: ObjectId) -> Evidence<BlobKind> {
        Evidence::Missing {
            scope,
            fact: EvidenceFact::BlobKind,
            object_id: Some(blob_id),
            node_id: None,
        }
    }

    fn blob_content(
        &self,
        scope: EvidenceScope,
        blob_id: ObjectId,
    ) -> Evidence<(BlobKind, Vec<u8>)> {
        Evidence::Missing {
            scope,
            fact: EvidenceFact::BlobBytes,
            object_id: Some(blob_id),
            node_id: None,
        }
    }
}
