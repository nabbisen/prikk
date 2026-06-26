//! Patch payload types.

use prikk_error::{PrikkError, Result};

use crate::canonical::{is_contiguous_op_seq, is_strictly_sorted};
use crate::payload::common::{Intent, OperationCondition, OperationConditionEntry};
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

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
            writer.field_u32(3, u32::from(intent.code()))?;
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
    /// Replacement text.
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
