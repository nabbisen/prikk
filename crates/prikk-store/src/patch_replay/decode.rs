//! Canonical patch-operation decoder: decodes every FDD-03 §9.3 operation kind
//! into a typed [`DecodedPatchOperation`]. Decoding is structural only; whether an
//! operation can be applied/replayed is a separate decision gated by
//! [`ensure_apply_supported`] (review erratum P1).
//!
//! Split across three files (DC-58): this file keeps the types and top-level dispatch;
//! `operations.rs` holds the seven per-operation-kind decoders; `tlv.rs` holds the low-level
//! canonical TLV cursor/field reading. No behaviour change.

use prikk_error::{PrikkError, Result};
use prikk_object::{NodeId, NodeKind, ObjectId, PatchPurpose, TEXT_SPAN_HASH_BYTES, WireType};

mod operations;
mod tlv;

use operations::{
    decode_change_perm, decode_create_file, decode_create_symlink, decode_delete_node,
    decode_edit_text, decode_rename_path, decode_replace_binary,
};
use tlv::TlvCursor;

/// One decoded operation: the validated `op_seq` envelope plus the typed body.
///
/// FDD-03 §9.2.1 already validated `op_seq == physical position + 1` during decode,
/// but the value is retained here (review erratum P2) for diagnostics, inverse
/// planning, and validator messages, so promoting the body into typed variants does
/// not discard the operation envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecodedPatchOperation {
    pub(crate) op_seq: u32,
    pub(crate) kind: DecodedOperationKind,
}

/// The decoded body of every FDD-03 §9.3 operation kind.
///
/// Decoding into a variant is a *structural* fact, not an *applicability* fact:
/// review erratum P1 requires that "decoded successfully" never be read as
/// "supported by replay/apply". The apply-supported subset is the single gate
/// [`ensure_apply_supported`]; the not-yet-wired kinds carry their decoded fields
/// for the node-model application increment (4.4) and are `dead_code`-allowed until
/// then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DecodedOperationKind {
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
    /// Delete a node, carrying its discriminated deletion preimage (§9.3).
    DeleteNode {
        /// Repository-relative path.
        path: String,
        /// Node identity.
        node_id: NodeId,
        /// Discriminated old-state preimage (file or symlink).
        preimage: DecodedDeletePreimage,
    },
    /// Span-anchored text edit (node-addressed; apply/inverse is FDD-01 §7.2.1, 4.4).
    EditText {
        node_id: NodeId,
        span_id: [u8; TEXT_SPAN_HASH_BYTES],
        old_span_hash: [u8; TEXT_SPAN_HASH_BYTES],
        left_anchor_hash: [u8; TEXT_SPAN_HASH_BYTES],
        right_anchor_hash: [u8; TEXT_SPAN_HASH_BYTES],
        replacement_text: Vec<u8>,
        old_span_text: Vec<u8>,
    },
    /// Replace a binary node's blob (node-addressed; apply is 4.4).
    ReplaceBinary {
        node_id: NodeId,
        old_blob_id: ObjectId,
        new_blob_id: ObjectId,
    },
    /// Rename a node (node-addressed; apply is 4.4).
    RenamePath {
        node_id: NodeId,
        old_path: String,
        new_path: String,
    },
    /// Change a node's mode (node-addressed; apply is 4.4).
    ChangePerm {
        node_id: NodeId,
        old_mode: u32,
        new_mode: u32,
    },
    /// Create a symlink node (apply is 4.4; static target validation FDD-04 §5.4a).
    CreateSymlink {
        path: String,
        node_id: NodeId,
        target: String,
    },
}

/// Discriminated `DeleteNode` deletion preimage (§9.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DecodedDeletePreimage {
    /// Text/binary file: old blob + old mode.
    File {
        old_node_kind: NodeKind,
        old_blob_id: ObjectId,
        old_mode: u32,
    },
    /// Symlink: old target (apply/inverse is 4.4).
    Symlink { old_target: String },
}

/// Apply-time support gate (review erratum P1). Decoding a kind says nothing about
/// whether replay/apply can execute it; this is the *single* source of truth for the
/// apply-supported subset. Returns `Ok(())` only for the kinds whose application is
/// wired today (`CreateFile`, file-`DeleteNode`, and `EditText`); node-addressed kinds whose
/// application is still deferred return
/// `UnsupportedObjectType`. Per review erratum P4, Phase 4 cannot be marked
/// implementation-reconciled while any kind still returns unsupported here.
pub(crate) fn ensure_apply_supported(operation: &DecodedPatchOperation) -> Result<()> {
    match &operation.kind {
        DecodedOperationKind::CreateFile { .. }
        | DecodedOperationKind::DeleteNode {
            preimage: DecodedDeletePreimage::File { .. },
            ..
        }
        | DecodedOperationKind::EditText { .. } => Ok(()),
        DecodedOperationKind::DeleteNode {
            preimage: DecodedDeletePreimage::Symlink { .. },
            ..
        } => Err(unsupported_operation("DeleteNode(symlink)")),
        DecodedOperationKind::ReplaceBinary { .. } => Err(unsupported_operation(
            "ReplaceBinary (node-addressed apply pending node model, increment 4.4)",
        )),
        DecodedOperationKind::RenamePath { .. } => Err(unsupported_operation(
            "RenamePath (node-addressed apply pending node model, increment 4.4)",
        )),
        DecodedOperationKind::ChangePerm { .. } => Err(unsupported_operation(
            "ChangePerm (node-addressed apply pending node model, increment 4.4)",
        )),
        DecodedOperationKind::CreateSymlink { .. } => Err(unsupported_operation(
            "CreateSymlink (apply pending node model, increment 4.4)",
        )),
    }
}

/// Decode every FDD-03 §9.3 operation kind from canonical patch payload bytes into
/// typed [`DecodedPatchOperation`]s. Decoding validates structure/identity only;
/// applicability is gated separately by [`ensure_apply_supported`] (erratum P1).
pub(crate) fn decode_patch_operations(bytes: &[u8]) -> Result<Vec<DecodedPatchOperation>> {
    PatchPurpose::decode_from_patch_payload(bytes).map_err(|err| {
        PrikkError::MalformedData(format!("invalid PatchPurpose canonical form: {err}"))
    })?;
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
            5 => {
                field.require_wire(WireType::EnumU16)?;
                let _ = field.read_u16()?;
            }
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

fn decode_operation(bytes: &[u8], index: usize) -> Result<DecodedPatchOperation> {
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
    let kind = match kind_tag {
        10 => decode_create_file(value),
        11 => decode_delete_node(value),
        12 => decode_edit_text(value),
        13 => decode_rename_path(value),
        14 => decode_change_perm(value),
        15 => decode_create_symlink(value),
        16 => decode_replace_binary(value),
        _ => unreachable!("kind tag is constrained to 10..=16 above"),
    }?;
    Ok(DecodedPatchOperation { op_seq, kind })
}

fn unsupported_operation(name: &str) -> PrikkError {
    PrikkError::UnsupportedObjectType(format!(
        "patch replay plan does not yet support {name}; patch algebra remains a later increment"
    ))
}
