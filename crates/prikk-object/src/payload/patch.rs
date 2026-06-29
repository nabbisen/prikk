//! Patch payload types.

use prikk_error::{PrikkError, Result};

use crate::canonical::{is_contiguous_op_seq, is_strictly_sorted};
use crate::payload::common::{Intent, OperationCondition, OperationConditionEntry};
use crate::payload::node::{NodeId, NodeKind};
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// Number of bytes in a content-anchored text span hash.
pub const TEXT_SPAN_HASH_BYTES: usize = 32;

/// Compute the stable hash used by content-anchored text edit preconditions.
#[must_use]
pub fn text_span_hash(bytes: &[u8]) -> [u8; TEXT_SPAN_HASH_BYTES] {
    prikk_hash::sha256(bytes)
}

/// Validate a stable content-anchor identifier.
pub fn validate_text_anchor_id(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(PrikkError::CanonicalEncoding(
            "text anchor id must not be empty".to_string(),
        ));
    }
    if !value.is_ascii() {
        return Err(PrikkError::CanonicalEncoding(
            "text anchor id must be ASCII in v1".to_string(),
        ));
    }
    if value.bytes().any(|byte| byte < 0x21 || byte == 0x7f) {
        return Err(PrikkError::CanonicalEncoding(
            "text anchor id must not contain whitespace or control characters".to_string(),
        ));
    }
    Ok(())
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
        if self.operations.is_empty() {
            return Err(PrikkError::CanonicalEncoding(
                "patch operations must contain at least one operation".to_string(),
            ));
        }
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
        writer.repeated_record_list(1, &self.operations)?;
        writer.repeated_object_id(2, &self.parent_patch_ids)?;
        if let Some(intent) = self.intent {
            writer.field_enum_u16(3, intent.code())?;
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
            OperationKind::DeleteNode(value) => writer.field_record(11, value)?,
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
    /// Delete a node.
    DeleteNode(DeleteNode),
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

/// Create file payload (FDD-03 §9.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateFile {
    /// Repo-relative UTF-8 path (`repo_path`).
    pub path: String,
    /// Node identity (`bytes`, 32).
    pub node_id: NodeId,
    /// Initial blob ID (`object_id`).
    pub blob_id: ObjectId,
    /// Mode bits (`u32`).
    pub mode: u32,
}

impl CreateFile {
    /// Reject an all-zero `node_id`; FDD-03 §9.3 forbids the reserved value in any
    /// persisted node-bearing operation, and the encoder produces identity bytes.
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "CreateFile node_id must be nonzero".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for CreateFile {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.field_repo_path(1, &self.path)?;
        writer.field_bytes(2, self.node_id.as_bytes())?;
        writer.field_object_id(3, &self.blob_id)?;
        writer.field_u32(4, self.mode)?;
        Ok(())
    }
}

/// Discriminated deletion preimage (FDD-03 §9.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteNodePreimage {
    /// File or binary node: blob + mode preimage.
    File {
        /// Previous blob ID (`object_id`).
        old_blob_id: ObjectId,
        /// Previous mode bits (`u32`).
        old_mode: u32,
    },
    /// Symlink node: target preimage.
    Symlink {
        /// Previous symlink target (`utf8`).
        old_target: String,
    },
}

/// Delete a node (FDD-03 §9.3; the wire tag is retained as `delete_file`). The
/// preimage is discriminated by `old_node_kind`: text/binary file nodes carry
/// `old_blob_id` + `old_mode`; symlink nodes carry `old_target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteNode {
    /// Repo-relative UTF-8 path (`repo_path`).
    pub path: String,
    /// Node identity (`bytes`, 32).
    pub node_id: NodeId,
    /// Previous node kind (`enum_u16`); must agree with the preimage.
    pub old_node_kind: NodeKind,
    /// Discriminated deletion preimage.
    pub preimage: DeleteNodePreimage,
}

impl DeleteNode {
    /// Reject `old_node_kind` / preimage discriminator mismatches and an all-zero
    /// `node_id` (FDD-03 §9.3 forbids the reserved value in any node-bearing op).
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "DeleteNode node_id must be nonzero".to_string(),
            ));
        }
        let consistent = matches!(
            (self.old_node_kind, &self.preimage),
            (
                NodeKind::TextFile | NodeKind::BinaryFile,
                DeleteNodePreimage::File { .. }
            ) | (NodeKind::Symlink, DeleteNodePreimage::Symlink { .. })
        );
        if !consistent {
            return Err(PrikkError::CanonicalEncoding(
                "DeleteNode old_node_kind does not match preimage discriminator".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for DeleteNode {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.field_repo_path(1, &self.path)?;
        writer.field_bytes(2, self.node_id.as_bytes())?;
        writer.field_enum_u16(3, self.old_node_kind.code())?;
        match &self.preimage {
            DeleteNodePreimage::File {
                old_blob_id,
                old_mode,
            } => {
                writer.field_object_id(4, old_blob_id)?;
                writer.field_u32(6, *old_mode)?;
            }
            DeleteNodePreimage::Symlink { old_target } => {
                writer.field_string(5, old_target)?;
            }
        }
        Ok(())
    }
}

/// Text edit payload using content-anchor identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditText {
    /// Node identity (`bytes`, 32). EditText is node-addressed, not path-addressed.
    pub node_id: NodeId,
    /// Content-anchor span identity (`bytes`, 32; FDD-01 §5.1).
    pub span_id: [u8; TEXT_SPAN_HASH_BYTES],
    /// SHA-256 of `old_span_text`; the validator binds the two.
    pub old_span_hash: [u8; TEXT_SPAN_HASH_BYTES],
    /// Bounded left-context hash (`bytes`, 32).
    pub left_anchor_hash: [u8; TEXT_SPAN_HASH_BYTES],
    /// Bounded right-context hash (`bytes`, 32).
    pub right_anchor_hash: [u8; TEXT_SPAN_HASH_BYTES],
    /// New span bytes (`bytes`); UTF-8 text for v1, stored verbatim (never NFC).
    pub replacement_text: Vec<u8>,
    /// Optional presentation hint (line); not part of algebraic identity.
    pub presentation_hint_line: Option<u32>,
    /// Optional presentation hint (column); not part of algebraic identity.
    pub presentation_hint_column: Option<u32>,
    /// Old span bytes (`bytes`); UTF-8 for v1, verbatim; inverse material.
    pub old_span_text: Vec<u8>,
}

impl EditText {
    /// Validate the FDD-03 §9.3 EditText record contract: nonzero `node_id`,
    /// `old_span_hash == SHA-256(old_span_text)`, and both span-text fields are
    /// well-formed UTF-8 (non-UTF-8 content must use `ReplaceBinary`).
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "EditText node_id must be nonzero".to_string(),
            ));
        }
        if self.old_span_hash != text_span_hash(&self.old_span_text) {
            return Err(PrikkError::CanonicalEncoding(
                "EditText old_span_hash must equal SHA-256(old_span_text)".to_string(),
            ));
        }
        if core::str::from_utf8(&self.old_span_text).is_err() {
            return Err(PrikkError::CanonicalEncoding(
                "EditText old_span_text must be well-formed UTF-8".to_string(),
            ));
        }
        if core::str::from_utf8(&self.replacement_text).is_err() {
            return Err(PrikkError::CanonicalEncoding(
                "EditText replacement_text must be well-formed UTF-8".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for EditText {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.field_bytes(1, self.node_id.as_bytes())?;
        writer.field_bytes(2, &self.span_id)?;
        writer.field_bytes(3, &self.old_span_hash)?;
        writer.field_bytes(4, &self.left_anchor_hash)?;
        writer.field_bytes(5, &self.right_anchor_hash)?;
        writer.field_bytes(6, &self.replacement_text)?;
        if let Some(line) = self.presentation_hint_line {
            writer.field_u32(7, line)?;
        }
        if let Some(column) = self.presentation_hint_column {
            writer.field_u32(8, column)?;
        }
        writer.field_bytes(9, &self.old_span_text)?;
        Ok(())
    }
}

/// Rename path payload (FDD-03 §9.3, node-addressed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePath {
    /// Node identity (`bytes`, 32).
    pub node_id: NodeId,
    /// Old repo-relative path (`repo_path`).
    pub old_path: String,
    /// New repo-relative path (`repo_path`).
    pub new_path: String,
}

impl RenamePath {
    /// Reject an all-zero `node_id`; FDD-03 §9.3 forbids the reserved value in any
    /// persisted node-bearing operation, and the encoder produces identity bytes.
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "RenamePath node_id must be nonzero".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for RenamePath {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.field_bytes(1, self.node_id.as_bytes())?;
        writer.field_repo_path(2, &self.old_path)?;
        writer.field_repo_path(3, &self.new_path)?;
        Ok(())
    }
}

/// Permission change payload (FDD-03 §9.3, node-addressed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePerm {
    /// Node identity (`bytes`, 32).
    pub node_id: NodeId,
    /// Old mode bits (`u32`).
    pub old_mode: u32,
    /// New mode bits (`u32`).
    pub new_mode: u32,
}

impl ChangePerm {
    /// Reject an all-zero `node_id` (FDD-03 §9.3).
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "ChangePerm node_id must be nonzero".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for ChangePerm {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.field_bytes(1, self.node_id.as_bytes())?;
        writer.field_u32(2, self.old_mode)?;
        writer.field_u32(3, self.new_mode)?;
        Ok(())
    }
}

/// Symlink creation payload (FDD-03 §9.3). Note tag order: `path` (1), then
/// `node_id` (2), then `target` (3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSymlink {
    /// Repo-relative UTF-8 path (`repo_path`).
    pub path: String,
    /// Node identity (`bytes`, 32).
    pub node_id: NodeId,
    /// Symlink target (`utf8_string`). Static escape/four-boundary validation
    /// (FDD-04 §5.4a / §13.1) is a later increment; this reconciles identity bytes.
    pub target: String,
}

impl CreateSymlink {
    /// Reject an all-zero `node_id` (FDD-03 §9.3).
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "CreateSymlink node_id must be nonzero".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for CreateSymlink {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.field_repo_path(1, &self.path)?;
        writer.field_bytes(2, self.node_id.as_bytes())?;
        writer.field_string(3, &self.target)?;
        Ok(())
    }
}

/// Binary replacement payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceBinary {
    /// Node identity (`bytes`, 32).
    pub node_id: NodeId,
    /// Old blob ID (`object_id`).
    pub old_blob_id: ObjectId,
    /// New blob ID (`object_id`).
    pub new_blob_id: ObjectId,
}

impl ReplaceBinary {
    /// Reject an all-zero `node_id`; FDD-03 §9.3 forbids the reserved value in any
    /// persisted node-bearing operation, and the encoder produces identity bytes.
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "ReplaceBinary node_id must be nonzero".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for ReplaceBinary {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate()?;
        writer.field_bytes(1, self.node_id.as_bytes())?;
        writer.field_object_id(2, &self.old_blob_id)?;
        writer.field_object_id(3, &self.new_blob_id)?;
        Ok(())
    }
}
