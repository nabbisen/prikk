//! Canonical patch-operation decoder used by supported replay.

use prikk_error::{PrikkError, Result};
use prikk_object::{NodeId, NodeKind, ObjectId, TEXT_SPAN_HASH_BYTES, WireType, text_span_hash};

use crate::path::RepoPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SupportedPatchOperation {
    /// Create a file from a persisted Blob.
    CreateFile {
        /// Repository-relative path.
        path: String,
        /// Node identity.
        node_id: NodeId,
        /// Initial Blob object ID.
        blob_id: ObjectId,
        /// Mode bits.
        mode: u32,
    },
    /// Delete a file node after verifying the old Blob precondition. Symlink-kind
    /// deletions are unsupported by replay (CreateSymlink is likewise unsupported).
    DeleteNode {
        /// Repository-relative path.
        path: String,
        /// Node identity.
        node_id: NodeId,
        /// Previous node kind (text or binary file).
        old_node_kind: NodeKind,
        /// Expected old Blob object ID.
        old_blob_id: ObjectId,
        /// Previous mode bits.
        old_mode: u32,
    },
}

/// Decode the supported patch-operation subset from canonical patch payload bytes.
pub(crate) fn decode_supported_patch_operations(
    bytes: &[u8],
) -> Result<Vec<SupportedPatchOperation>> {
    let mut cursor = TlvCursor::new(bytes);
    let mut operations = Vec::new();
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => {
                field.require_wire(WireType::RecordListItem)?;
                // FDD-03 §9.2.1: the operation at physical position `i` (zero-based)
                // must carry `op_seq == i + 1`. Passing the position lets
                // decode_operation enforce one-based/contiguous/unique/ordered as a
                // single check, so raw persisted/imported bytes cannot carry an
                // alternate canonical operation order.
                let index = operations.len();
                operations.push(decode_operation(field.value, index)?);
            }
            2..=4 => {}
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown Patch field tag: {other}"
                )));
            }
        }
    }
    if operations.is_empty() {
        return Err(PrikkError::MalformedData(
            "Patch missing operations".to_string(),
        ));
    }
    Ok(operations)
}

fn decode_operation(bytes: &[u8], index: usize) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut op_seq = None;
    // FDD-03 §9.2: an Operation record carries exactly one operation-kind field
    // (the oneof group, tags 10..=16). The Rust `OperationKind` enum enforces this
    // structurally on the write side; raw/imported bytes must be checked here, or a
    // malformed multi-kind record would decode "last-wins" and different readers
    // could interpret the same bytes differently. We claim the kind in the field
    // loop (rejecting a second kind before decoding it) and dispatch once after, so
    // the oneof check is independent of per-kind decode order or errors.
    let mut kind: Option<(u16, &[u8])> = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => op_seq = Some(field.read_u32()?),
            2 | 3 => {}
            10..=16 => {
                if let Some((first, _)) = kind {
                    return Err(PrikkError::MalformedData(format!(
                        "Operation carries multiple kind records: tag {first} and tag {}",
                        field.tag
                    )));
                }
                field.require_wire(WireType::Record)?;
                kind = Some((field.tag, field.value));
            }
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown Operation field tag: {other}"
                )));
            }
        }
    }
    let op_seq =
        op_seq.ok_or_else(|| PrikkError::MalformedData("Operation missing op_seq".to_string()))?;
    // FDD-03 §9.2.1: op_seq is one-based and equals physical position + 1. This
    // single comparison enforces first==1, contiguous, unique, and physical order ==
    // ascending op_seq; any deviation is rejected, never normalized.
    if (op_seq as usize) != index + 1 {
        return Err(PrikkError::MalformedData(format!(
            "operation op_seq {op_seq} does not match physical position {} (expected {})",
            index,
            index + 1
        )));
    }
    let (kind_tag, value) =
        kind.ok_or_else(|| PrikkError::MalformedData("Operation missing kind".to_string()))?;
    match kind_tag {
        10 => decode_create_file(value),
        11 => decode_delete_node(value),
        12 => decode_edit_text(value),
        13 => decode_rename_path(value),
        14 => decode_change_perm(value),
        15 => decode_create_symlink(value),
        16 => decode_replace_binary(value),
        _ => unreachable!("kind tag is constrained to 10..=16 above"),
    }
}

fn unsupported_operation(name: &str) -> PrikkError {
    PrikkError::UnsupportedObjectType(format!(
        "patch replay plan does not yet support {name}; patch algebra remains a later increment"
    ))
}

fn decode_create_file(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut node_id = None;
    let mut blob_id = None;
    let mut mode = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => path = Some(field.read_repo_path()?),
            2 => node_id = Some(field.read_node_id()?),
            3 => blob_id = Some(field.read_object_id_typed()?),
            4 => mode = Some(field.read_u32()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown CreateFile field tag: {other}"
                )));
            }
        }
    }
    let path =
        path.ok_or_else(|| PrikkError::MalformedData("CreateFile missing path".to_string()))?;
    RepoPath::parse(&path)?;
    let node_id = node_id
        .ok_or_else(|| PrikkError::MalformedData("CreateFile missing node_id".to_string()))?;
    let blob_id = blob_id
        .ok_or_else(|| PrikkError::MalformedData("CreateFile missing blob_id".to_string()))?;
    let mode =
        mode.ok_or_else(|| PrikkError::MalformedData("CreateFile missing mode".to_string()))?;
    Ok(SupportedPatchOperation::CreateFile {
        path,
        node_id,
        blob_id,
        mode,
    })
}

fn decode_delete_node(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut node_id = None;
    let mut old_node_kind = None;
    let mut old_blob_id = None;
    let mut old_target = None;
    let mut old_mode = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => path = Some(field.read_repo_path()?),
            2 => node_id = Some(field.read_node_id()?),
            3 => old_node_kind = Some(field.read_node_kind()?),
            4 => old_blob_id = Some(field.read_object_id_typed()?),
            5 => old_target = Some(field.read_string()?),
            6 => old_mode = Some(field.read_u32()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown DeleteNode field tag: {other}"
                )));
            }
        }
    }
    let path =
        path.ok_or_else(|| PrikkError::MalformedData("DeleteNode missing path".to_string()))?;
    RepoPath::parse(&path)?;
    let node_id = node_id
        .ok_or_else(|| PrikkError::MalformedData("DeleteNode missing node_id".to_string()))?;
    let old_node_kind = old_node_kind
        .ok_or_else(|| PrikkError::MalformedData("DeleteNode missing old_node_kind".to_string()))?;
    // Discriminator: file kinds carry old_blob_id + old_mode and MUST NOT carry
    // old_target; symlink carries old_target only and is unsupported by replay.
    match old_node_kind {
        NodeKind::TextFile | NodeKind::BinaryFile => {
            if old_target.is_some() {
                return Err(PrikkError::MalformedData(
                    "DeleteNode file kind must not carry old_target".to_string(),
                ));
            }
            let old_blob_id = old_blob_id.ok_or_else(|| {
                PrikkError::MalformedData("DeleteNode file kind missing old_blob_id".to_string())
            })?;
            let old_mode = old_mode.ok_or_else(|| {
                PrikkError::MalformedData("DeleteNode file kind missing old_mode".to_string())
            })?;
            Ok(SupportedPatchOperation::DeleteNode {
                path,
                node_id,
                old_node_kind,
                old_blob_id,
                old_mode,
            })
        }
        NodeKind::Symlink => {
            if old_blob_id.is_some() || old_mode.is_some() {
                return Err(PrikkError::MalformedData(
                    "DeleteNode symlink kind must not carry old_blob_id/old_mode".to_string(),
                ));
            }
            if old_target.is_none() {
                return Err(PrikkError::MalformedData(
                    "DeleteNode symlink kind missing old_target".to_string(),
                ));
            }
            Err(unsupported_operation("DeleteNode(symlink)"))
        }
    }
}

fn decode_edit_text(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    // Reconcile the FDD-03 §9.3 EditText record on read (node-addressed, span-
    // anchored), then report unsupported: span-anchored application/inverse is
    // FDD-01 §7.2.1 algebra and requires node-model tracking, both later
    // increments. This validates the persisted record and fails closed, mirroring
    // the discriminator-then-unsupported handling of symlink DeleteNode.
    let mut cursor = TlvCursor::new(bytes);
    let mut node_id = None;
    let mut span_id = None;
    let mut old_span_hash = None;
    let mut left_anchor_hash = None;
    let mut right_anchor_hash = None;
    let mut replacement_text = None;
    let mut old_span_text = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => node_id = Some(field.read_node_id()?),
            2 => span_id = Some(field.read_span_hash()?),
            3 => old_span_hash = Some(field.read_span_hash()?),
            4 => left_anchor_hash = Some(field.read_span_hash()?),
            5 => right_anchor_hash = Some(field.read_span_hash()?),
            6 => replacement_text = Some(field.read_bytes_vec()?),
            7 | 8 => {
                let _ = field.read_u32()?; // presentation hints; not algebraic
            }
            9 => old_span_text = Some(field.read_bytes_vec()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown EditText field tag: {other}"
                )));
            }
        }
    }
    let _node_id =
        node_id.ok_or_else(|| PrikkError::MalformedData("EditText missing node_id".to_string()))?;
    span_id.ok_or_else(|| PrikkError::MalformedData("EditText missing span_id".to_string()))?;
    left_anchor_hash.ok_or_else(|| {
        PrikkError::MalformedData("EditText missing left_anchor_hash".to_string())
    })?;
    right_anchor_hash.ok_or_else(|| {
        PrikkError::MalformedData("EditText missing right_anchor_hash".to_string())
    })?;
    let old_span_hash = old_span_hash
        .ok_or_else(|| PrikkError::MalformedData("EditText missing old_span_hash".to_string()))?;
    let replacement_text = replacement_text.ok_or_else(|| {
        PrikkError::MalformedData("EditText missing replacement_text".to_string())
    })?;
    let old_span_text = old_span_text
        .ok_or_else(|| PrikkError::MalformedData("EditText missing old_span_text".to_string()))?;
    // §9.3 validator: old_span_hash == SHA-256(old_span_text).
    if old_span_hash != text_span_hash(&old_span_text) {
        return Err(PrikkError::MalformedData(
            "EditText old_span_hash != SHA-256(old_span_text)".to_string(),
        ));
    }
    // §9.3 validator: both span-text fields must be well-formed UTF-8.
    if core::str::from_utf8(&old_span_text).is_err() {
        return Err(PrikkError::MalformedData(
            "EditText old_span_text is not well-formed UTF-8".to_string(),
        ));
    }
    if core::str::from_utf8(&replacement_text).is_err() {
        return Err(PrikkError::MalformedData(
            "EditText replacement_text is not well-formed UTF-8".to_string(),
        ));
    }
    Err(unsupported_operation(
        "EditText (span-anchored apply pending FDD-01 §7.2.1 + node model)",
    ))
}

fn decode_rename_path(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    // FDD-03 §9.3 RenamePath (node-addressed): node_id bytes, old_path repo_path,
    // new_path repo_path. Validate the record on read, then report unsupported
    // (application deferred to the node model, increment 4.4).
    let mut cursor = TlvCursor::new(bytes);
    let mut node_id = None;
    let mut old_path = None;
    let mut new_path = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => node_id = Some(field.read_node_id()?),
            2 => old_path = Some(field.read_repo_path()?),
            3 => new_path = Some(field.read_repo_path()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown RenamePath field tag: {other}"
                )));
            }
        }
    }
    node_id.ok_or_else(|| PrikkError::MalformedData("RenamePath missing node_id".to_string()))?;
    let old_path = old_path
        .ok_or_else(|| PrikkError::MalformedData("RenamePath missing old_path".to_string()))?;
    let new_path = new_path
        .ok_or_else(|| PrikkError::MalformedData("RenamePath missing new_path".to_string()))?;
    RepoPath::parse(&old_path)?;
    RepoPath::parse(&new_path)?;
    Err(unsupported_operation(
        "RenamePath (node-addressed apply pending node model, increment 4.4)",
    ))
}

fn decode_change_perm(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    // FDD-03 §9.3 ChangePerm (node-addressed): node_id bytes, old_mode u32,
    // new_mode u32. Validate the record on read, then report unsupported.
    let mut cursor = TlvCursor::new(bytes);
    let mut node_id = None;
    let mut old_mode = None;
    let mut new_mode = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => node_id = Some(field.read_node_id()?),
            2 => old_mode = Some(field.read_u32()?),
            3 => new_mode = Some(field.read_u32()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown ChangePerm field tag: {other}"
                )));
            }
        }
    }
    node_id.ok_or_else(|| PrikkError::MalformedData("ChangePerm missing node_id".to_string()))?;
    old_mode.ok_or_else(|| PrikkError::MalformedData("ChangePerm missing old_mode".to_string()))?;
    new_mode.ok_or_else(|| PrikkError::MalformedData("ChangePerm missing new_mode".to_string()))?;
    Err(unsupported_operation(
        "ChangePerm (node-addressed apply pending node model, increment 4.4)",
    ))
}

fn decode_create_symlink(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    // FDD-03 §9.3 CreateSymlink: path repo_path (tag 1), node_id bytes (tag 2),
    // target utf8_string (tag 3). Validate the record on read, then report
    // unsupported. Static symlink-target escape validation (FDD-04 §5.4a / §13.1)
    // is a later increment; this enforces the field shape and node_id.
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut node_id = None;
    let mut target = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => path = Some(field.read_repo_path()?),
            2 => node_id = Some(field.read_node_id()?),
            3 => target = Some(field.read_string()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown CreateSymlink field tag: {other}"
                )));
            }
        }
    }
    let path =
        path.ok_or_else(|| PrikkError::MalformedData("CreateSymlink missing path".to_string()))?;
    node_id
        .ok_or_else(|| PrikkError::MalformedData("CreateSymlink missing node_id".to_string()))?;
    target.ok_or_else(|| PrikkError::MalformedData("CreateSymlink missing target".to_string()))?;
    RepoPath::parse(&path)?;
    Err(unsupported_operation(
        "CreateSymlink (apply pending node model, increment 4.4)",
    ))
}

fn decode_replace_binary(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    // Reconcile the FDD-03 §9.3 ReplaceBinary record on read (node-addressed:
    // node_id + old_blob_id + new_blob_id, both object_id), validate the node_id and
    // blob-id presence/typing, then report unsupported. Like EditText, application is
    // node-addressed and cannot run against the path-keyed replay manifest; node
    // lookup (path->node_id tracking) plus the binary-only blob-kind resolution are
    // wired in at the node model (increment 4.4). The binary-only blob-kind rule
    // itself is implemented and tested as `blob_access::ensure_blob_kind_is_binary`.
    let mut cursor = TlvCursor::new(bytes);
    let mut node_id = None;
    let mut old_blob_id = None;
    let mut new_blob_id = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => node_id = Some(field.read_node_id()?),
            2 => old_blob_id = Some(field.read_object_id_typed()?),
            3 => new_blob_id = Some(field.read_object_id_typed()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown ReplaceBinary field tag: {other}"
                )));
            }
        }
    }
    node_id
        .ok_or_else(|| PrikkError::MalformedData("ReplaceBinary missing node_id".to_string()))?;
    old_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing old_blob_id".to_string())
    })?;
    new_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing new_blob_id".to_string())
    })?;
    Err(unsupported_operation(
        "ReplaceBinary (node-addressed apply pending node model, increment 4.4)",
    ))
}

struct TlvCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> TlvCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            last_tag: None,
        }
    }

    fn next_field(&mut self) -> Result<Option<TlvField<'a>>> {
        if self.pos == self.bytes.len() {
            return Ok(None);
        }
        let tag = u16::from_be_bytes(self.read_array::<2>()?);
        if tag == 0 {
            return Err(PrikkError::MalformedData(
                "field tag 0 is reserved".to_string(),
            ));
        }
        if let Some(last) = self.last_tag {
            if tag < last {
                return Err(PrikkError::MalformedData(format!(
                    "field tag order violation: {tag} after {last}"
                )));
            }
        }
        self.last_tag = Some(tag);
        let wire_type = self.read_u8()?;
        let len = usize::try_from(u64::from_be_bytes(self.read_array::<8>()?)).map_err(|_| {
            PrikkError::MalformedData("canonical field length does not fit usize".to_string())
        })?;
        let value = self.read_exact(len)?;
        Ok(Some(TlvField {
            tag,
            wire_type,
            value,
        }))
    }

    fn read_u8(&mut self) -> Result<u8> {
        let bytes = self.read_exact(1)?;
        let Some(byte) = bytes.first() else {
            return Err(PrikkError::MalformedData(
                "unexpected empty byte".to_string(),
            ));
        };
        Ok(*byte)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.read_exact(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| PrikkError::MalformedData("canonical range overflow".to_string()))?;
        let Some(slice) = self.bytes.get(self.pos..end) else {
            return Err(PrikkError::MalformedData(
                "unexpected end of canonical payload".to_string(),
            ));
        };
        self.pos = end;
        Ok(slice)
    }
}

struct TlvField<'a> {
    tag: u16,
    wire_type: u8,
    value: &'a [u8],
}

impl<'a> TlvField<'a> {
    fn read_string(&self) -> Result<String> {
        self.require_wire(WireType::String)?;
        String::from_utf8(self.value.to_vec())
            .map_err(|err| PrikkError::MalformedData(format!("invalid UTF-8 string: {err}")))
    }

    fn read_u32(&self) -> Result<u32> {
        self.require_wire(WireType::U32)?;
        Ok(u32::from_be_bytes(self.read_array::<4>()?))
    }

    /// Read an `object_id` (0x12) field. §9.3 references use the `object_id` value
    /// type (not `bytes`).
    fn read_object_id_typed(&self) -> Result<ObjectId> {
        self.require_wire(WireType::ObjectId)?;
        Ok(ObjectId::from_bytes(self.read_array::<32>()?))
    }

    /// Read a `repo_path` (0x13) field as a UTF-8 string. Callers still parse it
    /// through `RepoPath` for path-safety validation.
    fn read_repo_path(&self) -> Result<String> {
        self.require_wire(WireType::RepoPath)?;
        String::from_utf8(self.value.to_vec())
            .map_err(|err| PrikkError::MalformedData(format!("invalid UTF-8 repo_path: {err}")))
    }

    /// Read a `bytes` (0x11) field as a validated 32-byte node identity; rejects
    /// the all-zero reserved value via `NodeId::try_from_bytes`.
    fn read_node_id(&self) -> Result<NodeId> {
        self.require_wire(WireType::Bytes)?;
        NodeId::try_from_bytes(self.read_array::<32>()?)
    }

    /// Read an `enum_u16` (0x05) field as a `NodeKind`; rejects 0x0000/unknown.
    fn read_node_kind(&self) -> Result<NodeKind> {
        self.require_wire(WireType::EnumU16)?;
        NodeKind::from_code(u16::from_be_bytes(self.read_array::<2>()?))
    }

    fn read_span_hash(&self) -> Result<[u8; TEXT_SPAN_HASH_BYTES]> {
        self.require_wire(WireType::Bytes)?;
        self.read_array::<TEXT_SPAN_HASH_BYTES>()
    }

    /// Read a variable-length `bytes` (0x11) field.
    fn read_bytes_vec(&self) -> Result<Vec<u8>> {
        self.require_wire(WireType::Bytes)?;
        Ok(self.value.to_vec())
    }

    fn require_wire(&self, expected: WireType) -> Result<()> {
        if self.wire_type == expected as u8 {
            return Ok(());
        }
        Err(PrikkError::MalformedData(format!(
            "field {} has wrong wire type: expected {}, got {}",
            self.tag, expected as u8, self.wire_type
        )))
    }

    fn read_array<const N: usize>(&self) -> Result<[u8; N]> {
        if self.value.len() != N {
            return Err(PrikkError::MalformedData(format!(
                "field {} expected {N} bytes, got {}",
                self.tag,
                self.value.len()
            )));
        }
        let mut out = [0_u8; N];
        out.copy_from_slice(self.value);
        Ok(out)
    }
}
