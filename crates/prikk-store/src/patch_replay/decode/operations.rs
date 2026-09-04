//! Per-operation-kind decoders for FDD-03 §9.3 records. Split out of `decode.rs` (DC-58) — no
//! behaviour change, all items moved verbatim.

use prikk_error::{PrikkError, Result};
use prikk_object::{
    NodeKind, PATCH_TEXT_SPAN_V2_SCHEMA, TEXT_SPAN_ANCHOR_MIN_LEN, is_canonical_file_mode,
    text_span_hash,
};

use crate::path::RepoPath;

use super::DecodedOperationKind;
use super::tlv::TlvCursor;

pub(super) fn decode_create_file(bytes: &[u8]) -> Result<DecodedOperationKind> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut node_id = None;
    let mut blob_id = None;
    let mut mode = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => {
                if path.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate CreateFile path field".to_string(),
                    ));
                }
                path = Some(field.read_repo_path()?);
            }
            2 => {
                if node_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate CreateFile node_id field".to_string(),
                    ));
                }
                node_id = Some(field.read_node_id()?);
            }
            3 => {
                if blob_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate CreateFile blob_id field".to_string(),
                    ));
                }
                blob_id = Some(field.read_object_id_typed()?);
            }
            4 => {
                if mode.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate CreateFile mode field".to_string(),
                    ));
                }
                mode = Some(field.read_u32()?);
            }
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
    if !is_canonical_file_mode(mode) {
        return Err(PrikkError::MalformedData(format!(
            "CreateFile mode {mode:#o} is not one of prikk's canonical file modes"
        )));
    }
    Ok(DecodedOperationKind::CreateFile {
        path,
        node_id,
        blob_id,
        mode,
    })
}

pub(super) fn decode_delete_node(bytes: &[u8]) -> Result<DecodedOperationKind> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut node_id = None;
    let mut old_node_kind = None;
    let mut old_blob_id = None;
    let mut old_target = None;
    let mut old_mode = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => {
                if path.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate DeleteNode path field".to_string(),
                    ));
                }
                path = Some(field.read_repo_path()?);
            }
            2 => {
                if node_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate DeleteNode node_id field".to_string(),
                    ));
                }
                node_id = Some(field.read_node_id()?);
            }
            3 => {
                if old_node_kind.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate DeleteNode old_node_kind field".to_string(),
                    ));
                }
                old_node_kind = Some(field.read_node_kind()?);
            }
            4 => {
                if old_blob_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate DeleteNode old_blob_id field".to_string(),
                    ));
                }
                old_blob_id = Some(field.read_object_id_typed()?);
            }
            5 => {
                if old_target.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate DeleteNode old_target field".to_string(),
                    ));
                }
                old_target = Some(field.read_string()?);
            }
            6 => {
                if old_mode.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate DeleteNode old_mode field".to_string(),
                    ));
                }
                old_mode = Some(field.read_u32()?);
            }
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
            if !is_canonical_file_mode(old_mode) {
                return Err(PrikkError::MalformedData(format!(
                    "DeleteNode old_mode {old_mode:#o} is not one of prikk's canonical file modes"
                )));
            }
            Ok(DecodedOperationKind::DeleteNode {
                path,
                node_id,
                preimage: super::DecodedDeletePreimage::File {
                    old_node_kind,
                    old_blob_id,
                    old_mode,
                },
            })
        }
        NodeKind::Symlink => {
            if old_blob_id.is_some() || old_mode.is_some() {
                return Err(PrikkError::MalformedData(
                    "DeleteNode symlink kind must not carry old_blob_id/old_mode".to_string(),
                ));
            }
            let old_target = old_target.ok_or_else(|| {
                PrikkError::MalformedData("DeleteNode symlink kind missing old_target".to_string())
            })?;
            Ok(DecodedOperationKind::DeleteNode {
                path,
                node_id,
                preimage: super::DecodedDeletePreimage::Symlink { old_target },
            })
        }
    }
}

pub(super) fn decode_edit_text(bytes: &[u8], schema_version: u32) -> Result<DecodedOperationKind> {
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
    let mut left_anchor_len = None;
    let mut right_anchor_len = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => {
                if node_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText node_id field".to_string(),
                    ));
                }
                node_id = Some(field.read_node_id()?);
            }
            2 => {
                if span_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText span_id field".to_string(),
                    ));
                }
                span_id = Some(field.read_span_hash()?);
            }
            3 => {
                if old_span_hash.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText old_span_hash field".to_string(),
                    ));
                }
                old_span_hash = Some(field.read_span_hash()?);
            }
            4 => {
                if left_anchor_hash.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText left_anchor_hash field".to_string(),
                    ));
                }
                left_anchor_hash = Some(field.read_span_hash()?);
            }
            5 => {
                if right_anchor_hash.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText right_anchor_hash field".to_string(),
                    ));
                }
                right_anchor_hash = Some(field.read_span_hash()?);
            }
            6 => {
                if replacement_text.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText replacement_text field".to_string(),
                    ));
                }
                replacement_text = Some(field.read_bytes_vec()?);
            }
            7 | 8 => {
                // Presentation hints (line/column); not algebraic, deliberately not stored, so
                // there is nothing here for a duplicate to overwrite.
                let _ = field.read_u32()?;
            }
            9 => {
                if old_span_text.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText old_span_text field".to_string(),
                    ));
                }
                old_span_text = Some(field.read_bytes_vec()?);
            }
            10 => {
                if left_anchor_len.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText left_anchor_len field".to_string(),
                    ));
                }
                if schema_version < PATCH_TEXT_SPAN_V2_SCHEMA {
                    return Err(PrikkError::MalformedData(format!(
                        "Patch schema {schema_version} must not carry EditText left_anchor_len \
                         (tag 10); requires schema {PATCH_TEXT_SPAN_V2_SCHEMA}"
                    )));
                }
                left_anchor_len = Some(field.read_u32()?);
            }
            11 => {
                if right_anchor_len.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate EditText right_anchor_len field".to_string(),
                    ));
                }
                if schema_version < PATCH_TEXT_SPAN_V2_SCHEMA {
                    return Err(PrikkError::MalformedData(format!(
                        "Patch schema {schema_version} must not carry EditText right_anchor_len \
                         (tag 11); requires schema {PATCH_TEXT_SPAN_V2_SCHEMA}"
                    )));
                }
                right_anchor_len = Some(field.read_u32()?);
            }
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown EditText field tag: {other}"
                )));
            }
        }
    }
    let node_id =
        node_id.ok_or_else(|| PrikkError::MalformedData("EditText missing node_id".to_string()))?;
    let span_id =
        span_id.ok_or_else(|| PrikkError::MalformedData("EditText missing span_id".to_string()))?;
    let left_anchor_hash = left_anchor_hash.ok_or_else(|| {
        PrikkError::MalformedData("EditText missing left_anchor_hash".to_string())
    })?;
    let right_anchor_hash = right_anchor_hash.ok_or_else(|| {
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
    // RFC 134 §8: left_anchor_len/right_anchor_len must be both present or both absent, and each
    // at least TEXT_SPAN_ANCHOR_MIN_LEN when present -- mirrors EditText::validate() (prikk-object),
    // re-asserted here as defense-in-depth against raw/imported bytes (review erratum P1 pattern).
    match (left_anchor_len, right_anchor_len) {
        (None, None) => {}
        (Some(left), Some(right)) => {
            if left < TEXT_SPAN_ANCHOR_MIN_LEN || right < TEXT_SPAN_ANCHOR_MIN_LEN {
                return Err(PrikkError::MalformedData(format!(
                    "EditText anchor lengths must each be at least {TEXT_SPAN_ANCHOR_MIN_LEN}"
                )));
            }
        }
        _ => {
            return Err(PrikkError::MalformedData(
                "EditText left_anchor_len and right_anchor_len must be both present or both absent"
                    .to_string(),
            ));
        }
    }
    Ok(DecodedOperationKind::EditText {
        node_id,
        span_id,
        old_span_hash,
        left_anchor_hash,
        right_anchor_hash,
        replacement_text,
        old_span_text,
        left_anchor_len,
        right_anchor_len,
    })
}

pub(super) fn decode_rename_path(bytes: &[u8]) -> Result<DecodedOperationKind> {
    // FDD-03 §9.3 RenamePath (node-addressed): node_id bytes, old_path repo_path,
    // new_path repo_path. Validate the record on read, then report unsupported
    // (application deferred to the node model, increment 4.4).
    let mut cursor = TlvCursor::new(bytes);
    let mut node_id = None;
    let mut old_path = None;
    let mut new_path = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => {
                if node_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate RenamePath node_id field".to_string(),
                    ));
                }
                node_id = Some(field.read_node_id()?);
            }
            2 => {
                if old_path.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate RenamePath old_path field".to_string(),
                    ));
                }
                old_path = Some(field.read_repo_path()?);
            }
            3 => {
                if new_path.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate RenamePath new_path field".to_string(),
                    ));
                }
                new_path = Some(field.read_repo_path()?);
            }
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown RenamePath field tag: {other}"
                )));
            }
        }
    }
    let node_id = node_id
        .ok_or_else(|| PrikkError::MalformedData("RenamePath missing node_id".to_string()))?;
    let old_path = old_path
        .ok_or_else(|| PrikkError::MalformedData("RenamePath missing old_path".to_string()))?;
    let new_path = new_path
        .ok_or_else(|| PrikkError::MalformedData("RenamePath missing new_path".to_string()))?;
    RepoPath::parse(&old_path)?;
    RepoPath::parse(&new_path)?;
    Ok(DecodedOperationKind::RenamePath {
        node_id,
        old_path,
        new_path,
    })
}

pub(super) fn decode_change_perm(bytes: &[u8]) -> Result<DecodedOperationKind> {
    // FDD-03 §9.3 ChangePerm (node-addressed): node_id bytes, old_mode u32,
    // new_mode u32. Validate the record on read, then report unsupported.
    let mut cursor = TlvCursor::new(bytes);
    let mut node_id = None;
    let mut old_mode = None;
    let mut new_mode = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => {
                if node_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate ChangePerm node_id field".to_string(),
                    ));
                }
                node_id = Some(field.read_node_id()?);
            }
            2 => {
                if old_mode.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate ChangePerm old_mode field".to_string(),
                    ));
                }
                old_mode = Some(field.read_u32()?);
            }
            3 => {
                if new_mode.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate ChangePerm new_mode field".to_string(),
                    ));
                }
                new_mode = Some(field.read_u32()?);
            }
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown ChangePerm field tag: {other}"
                )));
            }
        }
    }
    let node_id = node_id
        .ok_or_else(|| PrikkError::MalformedData("ChangePerm missing node_id".to_string()))?;
    let old_mode = old_mode
        .ok_or_else(|| PrikkError::MalformedData("ChangePerm missing old_mode".to_string()))?;
    let new_mode = new_mode
        .ok_or_else(|| PrikkError::MalformedData("ChangePerm missing new_mode".to_string()))?;
    if !is_canonical_file_mode(old_mode) {
        return Err(PrikkError::MalformedData(format!(
            "ChangePerm old_mode {old_mode:#o} is not one of prikk's canonical file modes"
        )));
    }
    if !is_canonical_file_mode(new_mode) {
        return Err(PrikkError::MalformedData(format!(
            "ChangePerm new_mode {new_mode:#o} is not one of prikk's canonical file modes"
        )));
    }
    Ok(DecodedOperationKind::ChangePerm {
        node_id,
        old_mode,
        new_mode,
    })
}

pub(super) fn decode_create_symlink(bytes: &[u8]) -> Result<DecodedOperationKind> {
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
            1 => {
                if path.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate CreateSymlink path field".to_string(),
                    ));
                }
                path = Some(field.read_repo_path()?);
            }
            2 => {
                if node_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate CreateSymlink node_id field".to_string(),
                    ));
                }
                node_id = Some(field.read_node_id()?);
            }
            3 => {
                if target.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate CreateSymlink target field".to_string(),
                    ));
                }
                target = Some(field.read_string()?);
            }
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown CreateSymlink field tag: {other}"
                )));
            }
        }
    }
    let path =
        path.ok_or_else(|| PrikkError::MalformedData("CreateSymlink missing path".to_string()))?;
    let node_id = node_id
        .ok_or_else(|| PrikkError::MalformedData("CreateSymlink missing node_id".to_string()))?;
    let target = target
        .ok_or_else(|| PrikkError::MalformedData("CreateSymlink missing target".to_string()))?;
    RepoPath::parse(&path)?;
    Ok(DecodedOperationKind::CreateSymlink {
        path,
        node_id,
        target,
    })
}

pub(super) fn decode_replace_binary(bytes: &[u8]) -> Result<DecodedOperationKind> {
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
            1 => {
                if node_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate ReplaceBinary node_id field".to_string(),
                    ));
                }
                node_id = Some(field.read_node_id()?);
            }
            2 => {
                if old_blob_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate ReplaceBinary old_blob_id field".to_string(),
                    ));
                }
                old_blob_id = Some(field.read_object_id_typed()?);
            }
            3 => {
                if new_blob_id.is_some() {
                    return Err(PrikkError::MalformedData(
                        "duplicate ReplaceBinary new_blob_id field".to_string(),
                    ));
                }
                new_blob_id = Some(field.read_object_id_typed()?);
            }
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown ReplaceBinary field tag: {other}"
                )));
            }
        }
    }
    let node_id = node_id
        .ok_or_else(|| PrikkError::MalformedData("ReplaceBinary missing node_id".to_string()))?;
    let old_blob_id = old_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing old_blob_id".to_string())
    })?;
    let new_blob_id = new_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing new_blob_id".to_string())
    })?;
    Ok(DecodedOperationKind::ReplaceBinary {
        node_id,
        old_blob_id,
        new_blob_id,
    })
}
