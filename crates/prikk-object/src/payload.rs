//! Canonical payload shapes for PRIKK object types.

pub mod attestation;
pub mod blob;
pub mod block;
pub mod common;
pub mod patch;
pub mod refs;
pub mod tag;

#[cfg(test)]
mod tests;

pub use attestation::{AttestationPayload, AttestationStatus, PluginResultEntry};
pub use blob::BlobPayload;
pub use block::{BlockKind, BlockPayload};
pub use common::{Intent, MerkleRoot, OperationCondition, OperationConditionEntry};
pub use patch::{
    text_span_hash, validate_text_anchor_id, ChangePerm, CreateFile, CreateSymlink, DeleteFile,
    EditText, Operation, OperationKind, PatchPayload, RenamePath, ReplaceBinary,
    TEXT_SPAN_HASH_BYTES,
};
pub use refs::{RefKind, RefStatePayload, RefUpdatePayload};
pub use tag::TagPayload;
