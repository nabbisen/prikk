//! Canonical patch-operation decoder used by supported replay.

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectId, TEXT_SPAN_HASH_BYTES, WireType, validate_text_anchor_id};

use crate::path::RepoPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SupportedPatchOperation {
    /// Create a file from a persisted Blob.
    CreateFile {
        /// Repository-relative path.
        path: String,
        /// Initial Blob object ID.
        blob_id: ObjectId,
    },
    /// Delete a file after verifying the old Blob precondition.
    DeleteFile {
        /// Repository-relative path.
        path: String,
        /// Expected old Blob object ID.
        old_blob_id: ObjectId,
    },
    /// Replace a file after verifying the old Blob precondition.
    ReplaceBinary {
        /// Repository-relative path.
        path: String,
        /// Expected old Blob object ID.
        old_blob_id: ObjectId,
        /// New Blob object ID.
        new_blob_id: ObjectId,
    },
    /// Replace a validated content-anchored text span.
    EditText {
        /// Repository-relative path.
        path: String,
        /// Stable text anchor ID.
        anchor_id: String,
        /// Expected old span hash.
        old_span_hash: [u8; TEXT_SPAN_HASH_BYTES],
        /// UTF-8 replacement text.
        replacement: String,
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
            1 => operations.push(decode_operation(field.value)?),
            2..=4 => {}
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown Patch field tag: {other}"
                )));
            }
        }
    }
    Ok(operations)
}

fn decode_operation(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut op_seq = None;
    let mut operation = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => op_seq = Some(field.read_u32()?),
            2 | 3 => {}
            10 => {
                field.require_wire(WireType::Record)?;
                operation = Some(decode_create_file(field.value)?);
            }
            11 => {
                field.require_wire(WireType::Record)?;
                operation = Some(decode_delete_file(field.value)?);
            }
            12 => {
                field.require_wire(WireType::Record)?;
                operation = Some(decode_edit_text(field.value)?);
            }
            13 => return Err(unsupported_operation("RenamePath")),
            14 => return Err(unsupported_operation("ChangePerm")),
            15 => return Err(unsupported_operation("CreateSymlink")),
            16 => {
                field.require_wire(WireType::Record)?;
                operation = Some(decode_replace_binary(field.value)?);
            }
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown Operation field tag: {other}"
                )));
            }
        }
    }
    let Some(_) = op_seq else {
        return Err(PrikkError::MalformedData(
            "Operation missing op_seq".to_string(),
        ));
    };
    operation.ok_or_else(|| PrikkError::MalformedData("Operation missing kind".to_string()))
}

fn unsupported_operation(name: &str) -> PrikkError {
    PrikkError::UnsupportedObjectType(format!(
        "patch replay plan does not yet support {name}; patch algebra remains a later increment"
    ))
}

fn decode_create_file(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut blob_id = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => path = Some(field.read_string()?),
            2 => blob_id = Some(field.read_object_id()?),
            3 => {
                let _ = field.read_u32()?;
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
    let blob_id = blob_id
        .ok_or_else(|| PrikkError::MalformedData("CreateFile missing blob_id".to_string()))?;
    Ok(SupportedPatchOperation::CreateFile { path, blob_id })
}

fn decode_delete_file(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut old_blob_id = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => path = Some(field.read_string()?),
            2 => old_blob_id = Some(field.read_object_id()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown DeleteFile field tag: {other}"
                )));
            }
        }
    }
    let path =
        path.ok_or_else(|| PrikkError::MalformedData("DeleteFile missing path".to_string()))?;
    RepoPath::parse(&path)?;
    let old_blob_id = old_blob_id
        .ok_or_else(|| PrikkError::MalformedData("DeleteFile missing old_blob_id".to_string()))?;
    Ok(SupportedPatchOperation::DeleteFile { path, old_blob_id })
}

fn decode_edit_text(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut anchor_id = None;
    let mut old_span_hash = None;
    let mut replacement = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => path = Some(field.read_string()?),
            2 => anchor_id = Some(field.read_string()?),
            3 => old_span_hash = Some(field.read_span_hash()?),
            4 => replacement = Some(field.read_string()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown EditText field tag: {other}"
                )));
            }
        }
    }
    let path =
        path.ok_or_else(|| PrikkError::MalformedData("EditText missing path".to_string()))?;
    RepoPath::parse(&path)?;
    let anchor_id = anchor_id
        .ok_or_else(|| PrikkError::MalformedData("EditText missing anchor_id".to_string()))?;
    validate_text_anchor_id(&anchor_id)?;
    let old_span_hash = old_span_hash
        .ok_or_else(|| PrikkError::MalformedData("EditText missing old_span_hash".to_string()))?;
    let replacement = replacement
        .ok_or_else(|| PrikkError::MalformedData("EditText missing replacement".to_string()))?;
    Ok(SupportedPatchOperation::EditText {
        path,
        anchor_id,
        old_span_hash,
        replacement,
    })
}

fn decode_replace_binary(bytes: &[u8]) -> Result<SupportedPatchOperation> {
    let mut cursor = TlvCursor::new(bytes);
    let mut path = None;
    let mut old_blob_id = None;
    let mut new_blob_id = None;
    while let Some(field) = cursor.next_field()? {
        match field.tag {
            1 => path = Some(field.read_string()?),
            2 => old_blob_id = Some(field.read_object_id()?),
            3 => new_blob_id = Some(field.read_object_id()?),
            other => {
                return Err(PrikkError::MalformedData(format!(
                    "unknown ReplaceBinary field tag: {other}"
                )));
            }
        }
    }
    let path =
        path.ok_or_else(|| PrikkError::MalformedData("ReplaceBinary missing path".to_string()))?;
    RepoPath::parse(&path)?;
    let old_blob_id = old_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing old_blob_id".to_string())
    })?;
    let new_blob_id = new_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing new_blob_id".to_string())
    })?;
    Ok(SupportedPatchOperation::ReplaceBinary {
        path,
        old_blob_id,
        new_blob_id,
    })
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

    fn read_object_id(&self) -> Result<ObjectId> {
        self.require_wire(WireType::Bytes)?;
        Ok(ObjectId::from_bytes(self.read_array::<32>()?))
    }

    fn read_span_hash(&self) -> Result<[u8; TEXT_SPAN_HASH_BYTES]> {
        self.require_wire(WireType::Bytes)?;
        self.read_array::<TEXT_SPAN_HASH_BYTES>()
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
