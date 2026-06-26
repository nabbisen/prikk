//! Canonical payload shapes for PRIKK object types.

use prikk_error::{PrikkError, Result};

use crate::canonical::{is_contiguous_op_seq, is_strictly_sorted};
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// A 32-byte Merkle root for a materialized tree state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MerkleRoot(pub [u8; 32]);

/// Advisory patch intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum Intent {
    /// Feature work.
    Feature = 1,
    /// Bug fix.
    Fix = 2,
    /// Refactoring.
    Refactor = 3,
    /// Documentation.
    Docs = 4,
    /// Test-only change.
    Test = 5,
}

impl Intent {
    /// Stable numeric code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

/// Operation-level condition entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationConditionEntry {
    /// Condition key.
    pub key: String,
    /// Condition value.
    pub value: OperationCondition,
}

impl CanonicalEncode for OperationConditionEntry {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.key)?;
        writer.field_record(2, &self.value)?;
        Ok(())
    }
}

/// Operation precondition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationCondition {
    /// Old content hash must match before the operation applies.
    OldContentHash(Vec<u8>),
    /// Named anchor must exist.
    AnchorExists(String),
    /// Path must exist.
    PathExists(String),
    /// Path must be absent.
    PathAbsent(String),
}

impl CanonicalEncode for OperationCondition {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        match self {
            Self::OldContentHash(hash) => {
                writer.field_u32(1, 1)?;
                writer.field_bytes(2, hash)?;
            }
            Self::AnchorExists(anchor) => {
                writer.field_u32(1, 2)?;
                writer.field_string(2, anchor)?;
            }
            Self::PathExists(path) => {
                writer.field_u32(1, 3)?;
                writer.field_string(2, path)?;
            }
            Self::PathAbsent(path) => {
                writer.field_u32(1, 4)?;
                writer.field_string(2, path)?;
            }
        }
        Ok(())
    }
}

/// Patch payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPayload {
    /// Operations in semantic order. `op_seq` must be contiguous from 1.
    pub operations: Vec<Operation>,
    /// Parent patch IDs. Sorted ascending.
    pub parent_patch_ids: Vec<ObjectId>,
    /// Advisory intent.
    pub intent: Option<Intent>,
    /// Patch-level preconditions, sorted by key.
    pub preconditions: Vec<OperationConditionEntry>,
}

impl PatchPayload {
    /// Validate ordering and duplicate constraints.
    pub fn validate(&self) -> Result<()> {
        let op_seq: Vec<u32> = self.operations.iter().map(|op| op.op_seq).collect();
        if !is_contiguous_op_seq(&op_seq) {
            return Err(PrikkError::CanonicalEncoding(
                "patch operations must have contiguous op_seq values starting at 1".to_string(),
            ));
        }
        if !is_strictly_sorted(&self.parent_patch_ids) {
            return Err(PrikkError::CanonicalEncoding(
                "parent_patch_ids must be sorted and unique".to_string(),
            ));
        }
        if !is_strictly_sorted(&self.preconditions) {
            return Err(PrikkError::CanonicalEncoding(
                "patch preconditions must be sorted and unique".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for PatchPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.repeated_record(1, &self.operations)?;
        writer.repeated_object_id(2, &self.parent_patch_ids)?;
        if let Some(intent) = self.intent {
            writer.field_u32(3, intent.code() as u32)?;
        }
        writer.repeated_record(4, &self.preconditions)?;
        Ok(())
    }
}

/// A single operation inside a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// Strict operation sequence, starting at 1 inside the patch.
    pub op_seq: u32,
    /// Optional stable label for UI/debugging.
    pub op_id: Option<String>,
    /// Inline operation preconditions.
    pub preconditions: Vec<OperationCondition>,
    /// Operation kind.
    pub kind: OperationKind,
}

impl CanonicalEncode for Operation {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_u32(1, self.op_seq)?;
        writer.field_string_opt(2, self.op_id.as_deref())?;
        writer.repeated_record(3, &self.preconditions)?;
        match &self.kind {
            OperationKind::CreateFile(value) => writer.field_record(10, value)?,
            OperationKind::DeleteFile(value) => writer.field_record(11, value)?,
            OperationKind::EditText(value) => writer.field_record(12, value)?,
            OperationKind::RenamePath(value) => writer.field_record(13, value)?,
            OperationKind::ChangePerm(value) => writer.field_record(14, value)?,
            OperationKind::CreateSymlink(value) => writer.field_record(15, value)?,
            OperationKind::ReplaceBinary(value) => writer.field_record(16, value)?,
        }
        Ok(())
    }
}

/// Operation variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationKind {
    /// Create a text or binary file.
    CreateFile(CreateFile),
    /// Delete a file.
    DeleteFile(DeleteFile),
    /// Edit text using content-anchored spans.
    EditText(EditText),
    /// Rename a path.
    RenamePath(RenamePath),
    /// Change Unix-like permissions.
    ChangePerm(ChangePerm),
    /// Create a symbolic link.
    CreateSymlink(CreateSymlink),
    /// Replace an opaque binary blob.
    ReplaceBinary(ReplaceBinary),
}

/// Create file payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateFile {
    /// Repo-relative UTF-8 path.
    pub path: String,
    /// Initial blob ID.
    pub blob_id: ObjectId,
    /// Mode bits.
    pub mode: u32,
}

impl CanonicalEncode for CreateFile {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.path)?;
        writer.field_bytes(2, self.blob_id.as_bytes())?;
        writer.field_u32(3, self.mode)?;
        Ok(())
    }
}

/// Delete file payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteFile {
    /// Repo-relative UTF-8 path.
    pub path: String,
    /// Previous blob ID needed for inverse/repair reachability.
    pub old_blob_id: ObjectId,
}

impl CanonicalEncode for DeleteFile {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.path)?;
        writer.field_bytes(2, self.old_blob_id.as_bytes())?;
        Ok(())
    }
}

/// Text edit payload using content anchor identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditText {
    /// Repo-relative UTF-8 path.
    pub path: String,
    /// Stable content-anchor identifier.
    pub anchor_id: String,
    /// Old content hash precondition for the edited span.
    pub old_span_hash: Vec<u8>,
    /// Replacement bytes; expected to be valid UTF-8 at higher validation layer.
    pub replacement: String,
}

impl CanonicalEncode for EditText {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.path)?;
        writer.field_string(2, &self.anchor_id)?;
        writer.field_bytes(3, &self.old_span_hash)?;
        writer.field_string(4, &self.replacement)?;
        Ok(())
    }
}

/// Rename path payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePath {
    /// Source path.
    pub src: String,
    /// Destination path.
    pub dst: String,
}

impl CanonicalEncode for RenamePath {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.src)?;
        writer.field_string(2, &self.dst)?;
        Ok(())
    }
}

/// Permission change payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePerm {
    /// Path.
    pub path: String,
    /// Old mode.
    pub old_mode: u32,
    /// New mode.
    pub new_mode: u32,
}

impl CanonicalEncode for ChangePerm {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.path)?;
        writer.field_u32(2, self.old_mode)?;
        writer.field_u32(3, self.new_mode)?;
        Ok(())
    }
}

/// Symlink creation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSymlink {
    /// Link path.
    pub path: String,
    /// Link target string.
    pub target: String,
}

impl CanonicalEncode for CreateSymlink {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.path)?;
        writer.field_string(2, &self.target)?;
        Ok(())
    }
}

/// Binary replacement payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceBinary {
    /// Path.
    pub path: String,
    /// Old blob ID.
    pub old_blob_id: ObjectId,
    /// New blob ID.
    pub new_blob_id: ObjectId,
}

impl CanonicalEncode for ReplaceBinary {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.path)?;
        writer.field_bytes(2, self.old_blob_id.as_bytes())?;
        writer.field_bytes(3, self.new_blob_id.as_bytes())?;
        Ok(())
    }
}

/// Block kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum BlockKind {
    /// Root block.
    Root = 1,
    /// Normal block.
    Normal = 2,
    /// Merge block.
    Merge = 3,
    /// Repair block.
    Repair = 4,
    /// Import block.
    Import = 5,
}

impl BlockKind {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

/// Block payload. Block summaries are intentionally not identity-bearing in PR-001.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPayload {
    /// Parent block IDs, sorted unless FDD later requires a semantic parent role ordering.
    pub parent_block_ids: Vec<ObjectId>,
    /// Block kind.
    pub kind: BlockKind,
    /// Patch IDs in canonical block patch order.
    pub patch_ids: Vec<ObjectId>,
    /// State Merkle root.
    pub state_merkle_root: MerkleRoot,
    /// Optional full snapshot blob reference.
    pub snapshot_blob_ref: Option<ObjectId>,
}

impl CanonicalEncode for BlockPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !is_strictly_sorted(&self.parent_block_ids) {
            return Err(PrikkError::CanonicalEncoding(
                "parent_block_ids must be sorted and unique".to_string(),
            ));
        }
        writer.repeated_object_id(1, &self.parent_block_ids)?;
        writer.field_u32(2, self.kind.code() as u32)?;
        writer.repeated_object_id(3, &self.patch_ids)?;
        writer.field_bytes(4, &self.state_merkle_root.0)?;
        if let Some(snapshot) = self.snapshot_blob_ref {
            writer.field_bytes(5, snapshot.as_bytes())?;
        }
        Ok(())
    }
}

/// Ref kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum RefKind {
    /// Branch ref.
    Branch = 1,
    /// Tag ref.
    Tag = 2,
}

impl RefKind {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

/// RefState payload stored as a content-addressed object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefStatePayload {
    /// Human-readable ref name.
    pub ref_name: String,
    /// Ref kind.
    pub kind: RefKind,
    /// Target object ID.
    pub target_object_id: ObjectId,
    /// Monotonic sequence number.
    pub update_seq: u64,
    /// Previous ref-state object ID.
    pub previous_ref_state_id: Option<ObjectId>,
    /// Required attestation IDs that justified this state.
    pub required_attestation_ids: Vec<ObjectId>,
}

impl CanonicalEncode for RefStatePayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !is_strictly_sorted(&self.required_attestation_ids) {
            return Err(PrikkError::CanonicalEncoding(
                "required_attestation_ids must be sorted and unique".to_string(),
            ));
        }
        writer.field_string(1, &self.ref_name)?;
        writer.field_u32(2, self.kind.code() as u32)?;
        writer.field_bytes(3, self.target_object_id.as_bytes())?;
        writer.field_u64(4, self.update_seq)?;
        if let Some(previous) = self.previous_ref_state_id {
            writer.field_bytes(5, previous.as_bytes())?;
        }
        writer.repeated_object_id(6, &self.required_attestation_ids)?;
        Ok(())
    }
}

/// Ref-update event payload stored inline in ref logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdatePayload {
    /// Ref name.
    pub ref_name: String,
    /// Previous RefState ID.
    pub old_ref_state_id: Option<ObjectId>,
    /// New RefState ID.
    pub new_ref_state_id: ObjectId,
    /// New target object ID.
    pub new_target_object_id: ObjectId,
    /// Update sequence.
    pub update_seq: u64,
    /// Authoritative event creation timestamp.
    pub created_at: u64,
    /// Author key ID.
    pub author_key_id: String,
}

impl CanonicalEncode for RefUpdatePayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.ref_name)?;
        if let Some(old) = self.old_ref_state_id {
            writer.field_bytes(2, old.as_bytes())?;
        }
        writer.field_bytes(3, self.new_ref_state_id.as_bytes())?;
        writer.field_bytes(4, self.new_target_object_id.as_bytes())?;
        writer.field_u64(5, self.update_seq)?;
        writer.field_u64(6, self.created_at)?;
        writer.field_string(7, &self.author_key_id)?;
        Ok(())
    }
}

/// Tag payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPayload {
    /// Tag name.
    pub name: String,
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Tag message.
    pub message: String,
    /// Authoritative creation timestamp.
    pub created_at: u64,
    /// Author key ID.
    pub author_key_id: String,
}

impl CanonicalEncode for TagPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.name)?;
        writer.field_bytes(2, self.target_block_id.as_bytes())?;
        writer.field_string(3, &self.message)?;
        writer.field_u64(4, self.created_at)?;
        writer.field_string(5, &self.author_key_id)?;
        Ok(())
    }
}

/// Attestation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum AttestationStatus {
    /// Passed policy.
    Pass = 1,
    /// Warning.
    Warn = 2,
    /// Failed policy.
    Fail = 3,
    /// Locally quarantined.
    Quarantine = 4,
}

impl AttestationStatus {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

/// Plugin result entry, sorted by plugin ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginResultEntry {
    /// Plugin ID.
    pub plugin_id: String,
    /// Status.
    pub status: AttestationStatus,
    /// Report object ID.
    pub report_blob_id: Option<ObjectId>,
}

impl CanonicalEncode for PluginResultEntry {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.plugin_id)?;
        writer.field_u32(2, self.status.code() as u32)?;
        if let Some(report) = self.report_blob_id {
            writer.field_bytes(3, report.as_bytes())?;
        }
        Ok(())
    }
}

/// Attestation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationPayload {
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Policy version string.
    pub policy_version: String,
    /// Policy hash.
    pub policy_hash: Vec<u8>,
    /// Plugin-set hash.
    pub plugin_set_hash: Vec<u8>,
    /// Results sorted by plugin ID.
    pub results: Vec<PluginResultEntry>,
    /// Overall status.
    pub status: AttestationStatus,
    /// Authoritative attestation creation timestamp.
    pub created_at: u64,
    /// True if this result can be reproduced offline from stored inputs.
    pub is_reproducible_offline: bool,
}

impl CanonicalEncode for AttestationPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !is_strictly_sorted(&self.results) {
            return Err(PrikkError::CanonicalEncoding(
                "plugin results must be sorted and unique by plugin_id".to_string(),
            ));
        }
        writer.field_bytes(1, self.target_block_id.as_bytes())?;
        writer.field_string(2, &self.policy_version)?;
        writer.field_bytes(3, &self.policy_hash)?;
        writer.field_bytes(4, &self.plugin_set_hash)?;
        writer.repeated_record(5, &self.results)?;
        writer.field_u32(6, self.status.code() as u32)?;
        writer.field_u64(7, self.created_at)?;
        writer.field_bool(8, self.is_reproducible_offline)?;
        Ok(())
    }
}

/// Blob payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobPayload {
    /// Blob bytes.
    pub bytes: Vec<u8>,
}

impl CanonicalEncode for BlobPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_bytes(1, &self.bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BlobPayload, EditText, Operation, OperationKind, PatchPayload};
    use crate::{CanonicalEncode, ObjectId, ObjectType};

    #[test]
    fn patch_operations_must_be_contiguous() {
        let patch = PatchPayload {
            operations: vec![Operation {
                op_seq: 2,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::EditText(EditText {
                    path: "a.txt".to_string(),
                    anchor_id: "anchor".to_string(),
                    old_span_hash: vec![1],
                    replacement: "hello".to_string(),
                }),
            }],
            parent_patch_ids: Vec::new(),
            intent: None,
            preconditions: Vec::new(),
        };
        assert!(patch.to_canonical_bytes().is_err());
    }

    #[test]
    fn blob_payload_has_stable_object_id() {
        let payload = BlobPayload {
            bytes: b"hello".to_vec(),
        };
        let bytes_a = payload.to_canonical_bytes();
        let bytes_b = payload.to_canonical_bytes();
        assert_eq!(bytes_a, bytes_b);
        if let Ok(bytes) = bytes_a {
            let id_a = ObjectId::from_canonical_payload(ObjectType::Blob, 1, &bytes);
            let id_b = ObjectId::from_canonical_payload(ObjectType::Blob, 1, &bytes);
            assert_eq!(id_a, id_b);
        }
    }
}
