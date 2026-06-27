//! Minimal patch replay planning for supported file-level operations.
//!
//! PR-020 introduces a deliberately narrow replay boundary. It can reconstruct an in-memory
//! snapshot manifest by walking a single-parent block chain and applying the file-level operations
//! emitted by `prikk commit --from-worktree`: `CreateFile`, `DeleteFile`, and `ReplaceBinary`.
//! Content-anchored text edits, renames, chmod, symlinks, merge algebra, and conflict handling
//! remain later increments.

use std::collections::{BTreeMap, HashSet};

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlobPayload, BlockPayload, CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType,
    RefStatePayload,
};

use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::path::RepoPath;
use crate::refs::RefStore;
use crate::snapshot::{SnapshotEntry, SnapshotManifest};

/// Read-only result of replaying supported patch operations to an in-memory snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchReplayPlan {
    /// Ref used as the checkout target.
    pub ref_name: String,
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Number of blocks replayed from oldest to newest.
    pub block_count: usize,
    /// Number of patch objects replayed.
    pub patch_count: usize,
    /// Number of supported operations applied.
    pub applied_operation_count: usize,
    /// Number of files in the resulting manifest.
    pub file_count: usize,
    /// Total content bytes in the resulting manifest.
    pub total_content_bytes: u64,
    /// Repository-relative paths in the resulting manifest.
    pub paths: Vec<String>,
}

/// Replay the supported operation subset for a ref without writing the worktree.
pub fn prepare_patch_replay_plan(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchReplayPlan> {
    let object_store = FileObjectStore::new(layout.clone());
    let target_block_id = current_target_block(layout, &object_store, ref_name)?;
    let block_ids = single_parent_chain(&object_store, target_block_id)?;
    let mut files = BTreeMap::new();
    let mut patch_count = 0_usize;
    let mut applied_operation_count = 0_usize;

    for block_id in &block_ids {
        let block = read_block(&object_store, *block_id)?;
        if let Some(snapshot_blob_ref) = block.snapshot_blob_ref {
            files = load_snapshot_files(&object_store, snapshot_blob_ref)?;
        }
        for patch_id in block.patch_ids {
            let patch = read_patch(&object_store, patch_id)?;
            let operations = decode_supported_patch_operations(&patch.canonical_payload)?;
            for operation in operations {
                apply_supported_operation(&object_store, &mut files, operation)?;
                applied_operation_count += 1;
            }
            patch_count += 1;
        }
    }

    let manifest = files_to_manifest(files)?;
    let paths = manifest
        .files
        .iter()
        .map(|entry| entry.path.as_str().to_string())
        .collect();
    Ok(PatchReplayPlan {
        ref_name: ref_name.to_string(),
        target_block_id,
        block_count: block_ids.len(),
        patch_count,
        applied_operation_count,
        file_count: manifest.files.len(),
        total_content_bytes: manifest.total_content_bytes(),
        paths,
    })
}

fn current_target_block(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    ref_name: &str,
) -> Result<ObjectId> {
    let ref_store = RefStore::new(layout.clone());
    let ref_state_id = ref_store.read_current_ref_state_id(ref_name)?.ok_or_else(|| {
        PrikkError::Integrity(format!("ref {ref_name} is not published"))
    })?;
    let envelope = object_store
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| {
            PrikkError::Integrity(format!(
                "ref {ref_name} points to missing RefState {ref_state_id}"
            ))
        })?;
    let ref_state = RefStatePayload::decode_canonical(&envelope.canonical_payload)?;
    if ref_state.ref_name != ref_name {
        return Err(PrikkError::Integrity(format!(
            "RefState name mismatch: expected {ref_name}, got {}",
            ref_state.ref_name
        )));
    }
    Ok(ref_state.target_object_id)
}

fn single_parent_chain(object_store: &FileObjectStore, target: ObjectId) -> Result<Vec<ObjectId>> {
    let mut newest_first = Vec::new();
    let mut seen = HashSet::new();
    let mut current = Some(target);
    while let Some(block_id) = current {
        if !seen.insert(block_id) {
            return Err(PrikkError::Integrity(format!(
                "block parent chain contains a cycle at {block_id}"
            )));
        }
        let block = read_block(object_store, block_id)?;
        if block.parent_block_ids.len() > 1 {
            return Err(PrikkError::UnsupportedObjectType(format!(
                "patch replay plan supports only single-parent block chains; block {block_id} has {} parents",
                block.parent_block_ids.len()
            )));
        }
        newest_first.push(block_id);
        current = block.parent_block_ids.first().copied();
    }
    newest_first.reverse();
    Ok(newest_first)
}

fn read_block(object_store: &FileObjectStore, block_id: ObjectId) -> Result<BlockPayload> {
    let envelope = object_store
        .read_typed(block_id, ObjectType::Block)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Block {block_id}")))?;
    BlockPayload::decode_canonical(&envelope.canonical_payload)
}

fn read_patch(object_store: &FileObjectStore, patch_id: ObjectId) -> Result<ObjectEnvelope> {
    object_store
        .read_typed(patch_id, ObjectType::Patch)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Patch {patch_id}")))
}

fn load_snapshot_files(
    object_store: &FileObjectStore,
    snapshot_blob_ref: ObjectId,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let envelope = object_store
        .read_typed(snapshot_blob_ref, ObjectType::Blob)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing snapshot Blob {snapshot_blob_ref}")))?;
    let blob = BlobPayload::decode_canonical(&envelope.canonical_payload)?;
    let manifest = SnapshotManifest::decode(&blob.bytes)?;
    let mut files = BTreeMap::new();
    for entry in manifest.files {
        files.insert(entry.path.as_str().to_string(), entry.bytes);
    }
    Ok(files)
}

fn files_to_manifest(files: BTreeMap<String, Vec<u8>>) -> Result<SnapshotManifest> {
    let mut entries = Vec::with_capacity(files.len());
    for (path, bytes) in files {
        entries.push(SnapshotEntry { path: RepoPath::parse(&path)?, bytes });
    }
    Ok(SnapshotManifest { files: entries })
}

fn apply_supported_operation(
    object_store: &FileObjectStore,
    files: &mut BTreeMap<String, Vec<u8>>,
    operation: SupportedPatchOperation,
) -> Result<()> {
    match operation {
        SupportedPatchOperation::CreateFile { path, blob_id } => {
            if files.contains_key(&path) {
                return Err(PrikkError::Integrity(format!(
                    "CreateFile would overwrite existing path {path}"
                )));
            }
            let bytes = read_blob_bytes(object_store, blob_id)?;
            files.insert(path, bytes);
        }
        SupportedPatchOperation::DeleteFile { path, old_blob_id } => {
            let old_bytes = files.get(&path).ok_or_else(|| {
                PrikkError::Integrity(format!("DeleteFile path is absent: {path}"))
            })?;
            ensure_blob_matches(old_bytes, old_blob_id)?;
            files.remove(&path);
        }
        SupportedPatchOperation::ReplaceBinary { path, old_blob_id, new_blob_id } => {
            let old_bytes = files.get(&path).ok_or_else(|| {
                PrikkError::Integrity(format!("ReplaceBinary path is absent: {path}"))
            })?;
            ensure_blob_matches(old_bytes, old_blob_id)?;
            let new_bytes = read_blob_bytes(object_store, new_blob_id)?;
            files.insert(path, new_bytes);
        }
    }
    Ok(())
}

fn read_blob_bytes(object_store: &FileObjectStore, blob_id: ObjectId) -> Result<Vec<u8>> {
    let envelope = object_store
        .read_typed(blob_id, ObjectType::Blob)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Blob {blob_id}")))?;
    let blob = BlobPayload::decode_canonical(&envelope.canonical_payload)?;
    Ok(blob.bytes)
}

fn ensure_blob_matches(bytes: &[u8], expected: ObjectId) -> Result<()> {
    let payload = BlobPayload { bytes: bytes.to_vec() };
    let id = ObjectId::from_canonical_payload(
        ObjectType::Blob,
        1,
        &payload.to_canonical_bytes()?,
    );
    if id == expected {
        return Ok(());
    }
    Err(PrikkError::Integrity(format!(
        "operation old_blob_id mismatch: expected {expected}, got {id}"
    )))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SupportedPatchOperation {
    CreateFile { path: String, blob_id: ObjectId },
    DeleteFile { path: String, old_blob_id: ObjectId },
    ReplaceBinary { path: String, old_blob_id: ObjectId, new_blob_id: ObjectId },
}

fn decode_supported_patch_operations(bytes: &[u8]) -> Result<Vec<SupportedPatchOperation>> {
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
                field.require_wire(prikk_object::canonical::WireType::Record)?;
                operation = Some(decode_create_file(field.value)?);
            }
            11 => {
                field.require_wire(prikk_object::canonical::WireType::Record)?;
                operation = Some(decode_delete_file(field.value)?);
            }
            12 => return Err(unsupported_operation("EditText")),
            13 => return Err(unsupported_operation("RenamePath")),
            14 => return Err(unsupported_operation("ChangePerm")),
            15 => return Err(unsupported_operation("CreateSymlink")),
            16 => {
                field.require_wire(prikk_object::canonical::WireType::Record)?;
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
        return Err(PrikkError::MalformedData("Operation missing op_seq".to_string()));
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
    let path = path.ok_or_else(|| PrikkError::MalformedData("CreateFile missing path".to_string()))?;
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
    let path = path.ok_or_else(|| PrikkError::MalformedData("DeleteFile missing path".to_string()))?;
    RepoPath::parse(&path)?;
    let old_blob_id = old_blob_id
        .ok_or_else(|| PrikkError::MalformedData("DeleteFile missing old_blob_id".to_string()))?;
    Ok(SupportedPatchOperation::DeleteFile { path, old_blob_id })
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
    let path = path
        .ok_or_else(|| PrikkError::MalformedData("ReplaceBinary missing path".to_string()))?;
    RepoPath::parse(&path)?;
    let old_blob_id = old_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing old_blob_id".to_string())
    })?;
    let new_blob_id = new_blob_id.ok_or_else(|| {
        PrikkError::MalformedData("ReplaceBinary missing new_blob_id".to_string())
    })?;
    Ok(SupportedPatchOperation::ReplaceBinary { path, old_blob_id, new_blob_id })
}

struct TlvCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> TlvCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0, last_tag: None }
    }

    fn next_field(&mut self) -> Result<Option<TlvField<'a>>> {
        if self.pos == self.bytes.len() {
            return Ok(None);
        }
        let tag = u16::from_be_bytes(self.read_array::<2>()?);
        if tag == 0 {
            return Err(PrikkError::MalformedData("field tag 0 is reserved".to_string()));
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
        Ok(Some(TlvField { tag, wire_type, value }))
    }

    fn read_u8(&mut self) -> Result<u8> {
        let bytes = self.read_exact(1)?;
        let Some(byte) = bytes.first() else {
            return Err(PrikkError::MalformedData("unexpected empty byte".to_string()));
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
        self.require_wire(prikk_object::canonical::WireType::String)?;
        String::from_utf8(self.value.to_vec())
            .map_err(|err| PrikkError::MalformedData(format!("invalid UTF-8 string: {err}")))
    }

    fn read_u32(&self) -> Result<u32> {
        self.require_wire(prikk_object::canonical::WireType::U32)?;
        Ok(u32::from_be_bytes(self.read_array::<4>()?))
    }

    fn read_object_id(&self) -> Result<ObjectId> {
        self.require_wire(prikk_object::canonical::WireType::Bytes)?;
        Ok(ObjectId::from_bytes(self.read_array::<32>()?))
    }

    fn require_wire(&self, expected: prikk_object::canonical::WireType) -> Result<()> {
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
