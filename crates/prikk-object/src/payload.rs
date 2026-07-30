//! Canonical payload shapes for Prikk object types.

pub mod attestation;
pub mod blob;
pub mod block;
pub mod common;
pub mod node;
pub mod patch;
pub mod refs;
pub mod tag;

#[cfg(test)]
mod tests;

pub use attestation::{AttestationPayload, AttestationStatus, PluginResultEntry};
pub use blob::{BlobKind, BlobPayload};
pub use block::{BlockKind, BlockPayload};
pub use common::{Intent, MerkleRoot, OperationCondition, OperationConditionEntry};
pub use node::{NODE_ID_BYTES, NodeId, NodeKind};
pub use patch::{
    ChangePerm, CreateFile, CreateSymlink, DeleteNode, DeleteNodePreimage, EditText, Operation,
    OperationKind, PatchPayload, PatchPurpose, RenamePath, ReplaceBinary, TEXT_SPAN_HASH_BYTES,
    text_span_hash, validate_text_anchor_id,
};
pub use refs::{REF_STATE_CLOSED_SCHEMA, RefKind, RefStatePayload, RefUpdatePayload};
pub use tag::TagPayload;
