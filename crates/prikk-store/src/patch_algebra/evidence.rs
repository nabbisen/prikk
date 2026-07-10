use prikk_object::{BlobKind, BlobPayload, NodeId, ObjectId, ObjectType};

use super::evidence_types::{
    Evidence, EvidenceError, EvidenceFact, EvidenceScope, PatchAlgebraEvidence,
};
use crate::lifecycle_cache::ReplayDerivedLifecycleState;
#[cfg(test)]
use crate::lifecycle_cache::replay_derived_state;
use crate::node_lifecycle::{NodeContent, NodeLifecycleState};
use crate::object_store::ObjectReader;

#[derive(Debug)]
pub(crate) struct StorePatchAlgebraEvidence<'a, R: ObjectReader> {
    reader: &'a R,
    #[cfg(test)]
    baseline_block_id: ObjectId,
    #[cfg(test)]
    lineage_horizon_id: ObjectId,
    baseline_state: NodeLifecycleState,
}

impl<'a, R: ObjectReader> StorePatchAlgebraEvidence<'a, R> {
    #[cfg(test)]
    pub(crate) fn from_store(
        reader: &'a R,
        baseline_block_id: ObjectId,
        lineage_horizon_id: ObjectId,
    ) -> Result<Self, EvidenceError> {
        let replay =
            replay_derived_state(reader, baseline_block_id, lineage_horizon_id).map_err(|err| {
                EvidenceError::Unreadable {
                    scope: EvidenceScope::SealedBaselineRequired,
                    fact: EvidenceFact::BaselineState,
                    object_id: Some(baseline_block_id),
                    reason: err.to_string(),
                }
            })?;
        Self::from_replay_derived(reader, lineage_horizon_id, replay)
    }

    pub(crate) fn from_replay_derived(
        reader: &'a R,
        lineage_horizon_id: ObjectId,
        replay: ReplayDerivedLifecycleState,
    ) -> Result<Self, EvidenceError> {
        #[cfg(not(test))]
        let _ = lineage_horizon_id;
        let baseline_state = replay.state().clone();
        baseline_state
            .validate_internal_consistency()
            .map_err(|err| EvidenceError::Malformed {
                scope: EvidenceScope::SealedBaselineRequired,
                fact: EvidenceFact::BaselineState,
                object_id: Some(replay.baseline_block_id()),
                reason: err.to_string(),
            })?;
        Ok(Self {
            reader,
            #[cfg(test)]
            baseline_block_id: replay.baseline_block_id(),
            #[cfg(test)]
            lineage_horizon_id,
            baseline_state,
        })
    }

    #[cfg(test)]
    pub(crate) fn baseline_block_id(&self) -> ObjectId {
        self.baseline_block_id
    }

    #[cfg(test)]
    pub(crate) fn lineage_horizon_id(&self) -> ObjectId {
        self.lineage_horizon_id
    }

    pub(crate) fn baseline_state(&self) -> &NodeLifecycleState {
        &self.baseline_state
    }

    fn read_blob(
        &self,
        scope: EvidenceScope,
        fact: EvidenceFact,
        blob_id: ObjectId,
    ) -> Evidence<BlobPayload> {
        let envelope = match self.reader.read_object(blob_id) {
            Ok(Some(envelope)) => envelope,
            Ok(None) => {
                return Evidence::Missing {
                    scope,
                    fact,
                    object_id: Some(blob_id),
                    node_id: None,
                };
            }
            Err(err) => {
                return Evidence::Unreadable {
                    scope,
                    fact,
                    object_id: Some(blob_id),
                    reason: err.to_string(),
                };
            }
        };
        if envelope.object_type != ObjectType::Blob {
            return Evidence::WrongObjectType {
                scope,
                object_id: blob_id,
                expected: ObjectType::Blob,
                actual: envelope.object_type,
            };
        }
        match BlobPayload::decode_canonical(&envelope.canonical_payload) {
            Ok(blob) => Evidence::Known(blob),
            Err(err) => Evidence::Malformed {
                scope,
                fact,
                object_id: Some(blob_id),
                reason: err.to_string(),
            },
        }
    }
}

impl<R: ObjectReader> PatchAlgebraEvidence for StorePatchAlgebraEvidence<'_, R> {
    fn baseline_text(
        &self,
        scope: EvidenceScope,
        node_id: NodeId,
        blob_id: ObjectId,
    ) -> Evidence<Vec<u8>> {
        let Some(live) = self.baseline_state.live_node(&node_id) else {
            return Evidence::Missing {
                scope,
                fact: EvidenceFact::BaselineText,
                object_id: Some(blob_id),
                node_id: Some(node_id),
            };
        };
        let NodeContent::File {
            blob_id: live_blob_id,
            ..
        } = &live.content
        else {
            return Evidence::Malformed {
                scope,
                fact: EvidenceFact::BaselineText,
                object_id: Some(blob_id),
                reason: "baseline text node has symlink content".to_string(),
            };
        };
        if live.kind != prikk_object::NodeKind::TextFile || *live_blob_id != blob_id {
            return Evidence::Malformed {
                scope,
                fact: EvidenceFact::BaselineText,
                object_id: Some(blob_id),
                reason: "baseline text request does not match live text node".to_string(),
            };
        }
        match self.read_blob(scope, EvidenceFact::BaselineText, blob_id) {
            Evidence::Known(blob) if blob.blob_kind == BlobKind::Text => {
                Evidence::Known(blob.content)
            }
            Evidence::Known(blob) => Evidence::WrongBlobKind {
                scope,
                blob_id,
                expected: BlobKind::Text,
                actual: blob.blob_kind,
            },
            Evidence::Missing {
                scope,
                fact,
                object_id,
                ..
            } => Evidence::Missing {
                scope,
                fact,
                object_id,
                node_id: Some(node_id),
            },
            Evidence::WrongObjectType {
                scope,
                object_id,
                expected,
                actual,
            } => Evidence::WrongObjectType {
                scope,
                object_id,
                expected,
                actual,
            },
            Evidence::WrongBlobKind { .. } => unreachable!("read_blob returns raw blob payload"),
            Evidence::Malformed {
                scope,
                fact,
                object_id,
                reason,
            } => Evidence::Malformed {
                scope,
                fact,
                object_id,
                reason,
            },
            Evidence::Unreadable {
                scope,
                fact,
                object_id,
                reason,
            } => Evidence::Unreadable {
                scope,
                fact,
                object_id,
                reason,
            },
        }
    }

    fn blob_kind(&self, scope: EvidenceScope, blob_id: ObjectId) -> Evidence<BlobKind> {
        map_blob_payload_evidence(
            self.read_blob(scope, EvidenceFact::BlobKind, blob_id),
            |blob| Evidence::Known(blob.blob_kind),
        )
    }

    fn blob_content(
        &self,
        scope: EvidenceScope,
        blob_id: ObjectId,
    ) -> Evidence<(BlobKind, Vec<u8>)> {
        map_blob_payload_evidence(
            self.read_blob(scope, EvidenceFact::BlobBytes, blob_id),
            |blob| Evidence::Known((blob.blob_kind, blob.content)),
        )
    }
}

fn map_blob_payload_evidence<T>(
    evidence: Evidence<BlobPayload>,
    on_known: impl FnOnce(BlobPayload) -> Evidence<T>,
) -> Evidence<T> {
    match evidence {
        Evidence::Known(blob) => on_known(blob),
        Evidence::Missing {
            scope,
            fact,
            object_id,
            node_id,
        } => Evidence::Missing {
            scope,
            fact,
            object_id,
            node_id,
        },
        Evidence::WrongObjectType {
            scope,
            object_id,
            expected,
            actual,
        } => Evidence::WrongObjectType {
            scope,
            object_id,
            expected,
            actual,
        },
        Evidence::WrongBlobKind { .. } => unreachable!("read_blob returns raw blob payload"),
        Evidence::Malformed {
            scope,
            fact,
            object_id,
            reason,
        } => Evidence::Malformed {
            scope,
            fact,
            object_id,
            reason,
        },
        Evidence::Unreadable {
            scope,
            fact,
            object_id,
            reason,
        } => Evidence::Unreadable {
            scope,
            fact,
            object_id,
            reason,
        },
    }
}
