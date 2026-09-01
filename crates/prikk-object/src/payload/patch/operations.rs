//! Per-operation-kind patch payload structs (FDD-03 §9.3): validation and canonical encoding.
//! Split out of `payload/patch.rs` (DC-58) — no behaviour change, all items moved verbatim.
//! Re-exported at `payload/patch.rs` so every existing public path (`payload::patch::CreateFile`
//! etc., and the crate-root re-export at `payload.rs`) is unchanged.

use prikk_error::{PrikkError, Result};

use crate::path::validate_repo_path;
use crate::payload::node::{NodeId, NodeKind};
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

use super::{TEXT_SPAN_HASH_BYTES, text_span_hash};

/// The only two mode values prikk ever authors for a file node (`prikk-store`'s
/// `worktree_patch/node_authoring.rs::REGULAR_FILE_MODE`/`EXECUTABLE_FILE_MODE`, and
/// `state_root.rs`'s own `REGULAR_MODE`/`EXECUTABLE_MODE` seal-time matcher -- all three must keep
/// agreeing on these two values). RFC 125 §2: any other `u32` a decoder would accept -- including
/// setuid/setgid/sticky bits `& 0o7777` admits at materialization
/// (`fsutil/anchored/linux.rs::fchmod`) -- is a mode the encoder never wrote, so refusing it here
/// closes the gap symmetrically rather than leaving decode more permissive than encode.
const CANONICAL_FILE_MODES: [u32; 2] = [0o100_644, 0o100_755];

/// Whether `mode` is one of the two values prikk ever authors for a file node. Exposed so
/// `prikk-store`'s patch-replay decoder can refuse the same non-canonical modes at decode that
/// this module's own `validate()` methods refuse at encode (RFC 125 §2, DC-54 symmetry).
#[must_use]
pub fn is_canonical_file_mode(mode: u32) -> bool {
    CANONICAL_FILE_MODES.contains(&mode)
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
    /// persisted node-bearing operation, and the encoder produces identity bytes. Also reject a
    /// `path` that violates the `RepoPath` grammar decode enforces (DC-54): encode must reject
    /// exactly what decode rejects, using the same validator, so the two sides cannot drift.
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "CreateFile node_id must be nonzero".to_string(),
            ));
        }
        validate_repo_path(&self.path)?;
        if !is_canonical_file_mode(self.mode) {
            return Err(PrikkError::CanonicalEncoding(format!(
                "CreateFile mode {:#o} is not one of prikk's canonical file modes",
                self.mode
            )));
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
    /// Reject `old_node_kind` / preimage discriminator mismatches, an all-zero
    /// `node_id` (FDD-03 §9.3 forbids the reserved value in any node-bearing op), and a `path`
    /// that violates the `RepoPath` grammar decode enforces (DC-54).
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "DeleteNode node_id must be nonzero".to_string(),
            ));
        }
        validate_repo_path(&self.path)?;
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
        if let DeleteNodePreimage::File { old_mode, .. } = &self.preimage {
            if !is_canonical_file_mode(*old_mode) {
                return Err(PrikkError::CanonicalEncoding(format!(
                    "DeleteNode old_mode {old_mode:#o} is not one of prikk's canonical file modes"
                )));
            }
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
    /// persisted node-bearing operation, and the encoder produces identity bytes. Also reject an
    /// `old_path` or `new_path` that violates the `RepoPath` grammar decode enforces (DC-54) —
    /// both fields are checked independently, since a path-safe `new_path` must not mask an
    /// unsafe `old_path`.
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "RenamePath node_id must be nonzero".to_string(),
            ));
        }
        validate_repo_path(&self.old_path)?;
        validate_repo_path(&self.new_path)?;
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
        if !is_canonical_file_mode(self.old_mode) {
            return Err(PrikkError::CanonicalEncoding(format!(
                "ChangePerm old_mode {:#o} is not one of prikk's canonical file modes",
                self.old_mode
            )));
        }
        if !is_canonical_file_mode(self.new_mode) {
            return Err(PrikkError::CanonicalEncoding(format!(
                "ChangePerm new_mode {:#o} is not one of prikk's canonical file modes",
                self.new_mode
            )));
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
    /// Reject an all-zero `node_id` (FDD-03 §9.3) and a `path` that violates the `RepoPath`
    /// grammar decode enforces (DC-54). `target` is deliberately left untouched: it is an opaque
    /// symlink target by accepted DC-40 design, not a repository-relative path.
    pub fn validate(&self) -> Result<()> {
        if self.node_id.is_zero() {
            return Err(PrikkError::CanonicalEncoding(
                "CreateSymlink node_id must be nonzero".to_string(),
            ));
        }
        validate_repo_path(&self.path)?;
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
