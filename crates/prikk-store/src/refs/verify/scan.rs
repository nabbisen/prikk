//! Ref pointer/log scanning and structural chain validation.

use std::collections::BTreeMap;
use std::path::Path;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType, RefStatePayload, RefUpdatePayload};

use crate::fsutil::{EntryKind, list_directory, read_file_required};
use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::refs::log::decode_log_file_bytes;
use crate::refs::pointer::read_ref_pointer;

#[derive(Debug, Clone)]
pub(super) struct PointerState {
    pub(super) id: ObjectId,
    pub(super) payload: RefStatePayload,
}

#[derive(Debug, Clone)]
pub(super) struct LogState {
    pub(super) tip: Option<ObjectId>,
    pub(super) previous_tip: Option<ObjectId>,
    pub(super) record_count: usize,
    pub(super) trailing_partial_bytes: usize,
    pub(super) has_legacy_timestamp: bool,
}

pub(super) struct RefLogEnvelope {
    pub(super) ref_name: String,
    pub(super) sequence: u64,
    pub(super) envelope: ObjectEnvelope,
}

pub(super) fn read_pointers(
    layout: &RepositoryLayout,
    objects: &FileObjectStore,
) -> Result<BTreeMap<String, PointerState>> {
    let dir = layout.refs_dir().join("by-id");
    let mut pointers = BTreeMap::new();
    let relative_dir = layout.repository_relative(&dir)?;
    for entry in list_directory(layout.repository_mutation_root(), &relative_dir)? {
        let path = dir.join(&entry.name);
        if entry.kind != EntryKind::Regular {
            return Err(unexpected_path("directory in ref pointer directory", &path));
        }
        if is_temporary_path(&path) {
            continue;
        }
        ensure_ref_path_shape(&path, ".ref")?;
        let pointer = read_ref_pointer(layout, &path)?.ok_or_else(|| {
            PrikkError::Integrity("ref pointer disappeared during verification".to_string())
        })?;
        if path != layout.ref_pointer_path(&pointer.ref_name) {
            return Err(unexpected_path("non-canonical ref pointer", &path));
        }
        let payload = verified_ref_state_payload(objects, pointer.ref_state_id)?;
        if payload.ref_name != pointer.ref_name {
            return Err(PrikkError::Integrity(format!(
                "RefState {} name differs from pointer ref {}",
                pointer.ref_state_id, pointer.ref_name
            )));
        }
        ensure_block_exists(objects, payload.target_object_id, pointer.ref_state_id)?;
        if pointers
            .insert(
                pointer.ref_name.clone(),
                PointerState {
                    id: pointer.ref_state_id,
                    payload,
                },
            )
            .is_some()
        {
            return Err(PrikkError::Integrity(format!(
                "duplicate pointer identity for {}",
                pointer.ref_name
            )));
        }
    }
    Ok(pointers)
}

pub(super) fn read_logs(
    layout: &RepositoryLayout,
    objects: &FileObjectStore,
    pointers: &BTreeMap<String, PointerState>,
) -> Result<(BTreeMap<String, LogState>, usize, Vec<RefLogEnvelope>)> {
    let dir = layout.refs_dir().join("logs");
    let mut logs = BTreeMap::new();
    let mut total = 0_usize;
    let mut envelopes = Vec::new();
    let relative_dir = layout.repository_relative(&dir)?;
    let pointer_names: Vec<_> = pointers.keys().cloned().collect();
    for entry in list_directory(layout.repository_mutation_root(), &relative_dir)? {
        let path = dir.join(&entry.name);
        if entry.kind != EntryKind::Regular {
            return Err(unexpected_path("directory in ref-log directory", &path));
        }
        if is_temporary_path(&path) {
            continue;
        }
        ensure_ref_path_shape(&path, ".log")?;
        let relative = layout.repository_relative(&path)?;
        let bytes = read_file_required(layout.repository_mutation_root(), &relative)?;
        let replay = decode_log_file_bytes(layout.format(), &bytes)?;
        if replay.records.is_empty() {
            if replay.trailing_partial_bytes == 0 {
                continue;
            }
            let name = pointer_names
                .iter()
                .find(|name| layout.ref_log_path(name) == path)
                .cloned()
                .ok_or_else(|| unexpected_path("unowned partial ref log", &path))?;
            logs.insert(
                name,
                LogState {
                    tip: None,
                    previous_tip: None,
                    record_count: 0,
                    trailing_partial_bytes: replay.trailing_partial_bytes,
                    has_legacy_timestamp: false,
                },
            );
            continue;
        }
        let (name, state) = validate_log(layout, objects, &path, &replay)?;
        for (index, record) in replay.records.iter().enumerate() {
            let sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| PrikkError::Integrity("ref-log sequence overflow".to_string()))?;
            envelopes.push(RefLogEnvelope {
                ref_name: name.clone(),
                sequence,
                envelope: record.envelope.clone(),
            });
        }
        total = total
            .checked_add(replay.records.len())
            .ok_or_else(|| PrikkError::Integrity("ref-log count overflow".to_string()))?;
        if logs.insert(name.clone(), state).is_some() {
            return Err(PrikkError::Integrity(format!(
                "duplicate ref-log identity for {name}"
            )));
        }
    }
    envelopes.sort_by(|left, right| {
        left.ref_name
            .as_bytes()
            .cmp(right.ref_name.as_bytes())
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    Ok((logs, total, envelopes))
}

fn validate_log(
    layout: &RepositoryLayout,
    objects: &FileObjectStore,
    path: &Path,
    replay: &super::super::log::RefLogReplay,
) -> Result<(String, LogState)> {
    let mut previous = None;
    let mut previous_tip = None;
    let mut ref_name = None;
    let mut has_legacy_timestamp = false;
    for (index, record) in replay.records.iter().enumerate() {
        let update = RefUpdatePayload::decode_canonical(&record.envelope.canonical_payload)?;
        let expected_seq = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| PrikkError::Integrity("ref-log sequence overflow".to_string()))?;
        if ref_name
            .as_ref()
            .is_some_and(|name| name != &update.ref_name)
            || update.old_ref_state_id != previous
            || update.update_seq != expected_seq
        {
            return Err(PrikkError::Integrity(format!(
                "ref-log chain or sequence diverges for {}",
                update.ref_name
            )));
        }
        verify_update(objects, &update)?;
        has_legacy_timestamp |= update.created_at != 0;
        ref_name.get_or_insert_with(|| update.ref_name.clone());
        previous_tip = previous;
        previous = Some(update.new_ref_state_id);
    }
    let name = ref_name
        .ok_or_else(|| PrikkError::Integrity("non-empty ref log has no identity".to_string()))?;
    if path != layout.ref_log_path(&name) {
        return Err(unexpected_path("non-canonical ref log", path));
    }
    Ok((
        name,
        LogState {
            tip: previous,
            previous_tip,
            record_count: replay.records.len(),
            trailing_partial_bytes: replay.trailing_partial_bytes,
            has_legacy_timestamp,
        },
    ))
}

fn verify_update(objects: &FileObjectStore, update: &RefUpdatePayload) -> Result<()> {
    let state = verified_ref_state_payload(objects, update.new_ref_state_id)?;
    if state.ref_name != update.ref_name
        || state.previous_ref_state_id != update.old_ref_state_id
        || state.target_object_id != update.new_target_object_id
        || state.update_seq != update.update_seq
    {
        return Err(PrikkError::Integrity(format!(
            "RefState disagrees with RefUpdate for {}",
            update.ref_name
        )));
    }
    ensure_block_exists(
        objects,
        update.new_target_object_id,
        update.new_ref_state_id,
    )
}

fn verified_ref_state_payload(
    objects: &FileObjectStore,
    ref_state_id: ObjectId,
) -> Result<RefStatePayload> {
    let envelope = objects
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing RefState object: {ref_state_id}")))?;
    if envelope.signatures.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} is unsigned"
        )));
    }
    RefStatePayload::decode_canonical(&envelope.canonical_payload)
}

fn ensure_block_exists(
    objects: &FileObjectStore,
    block_id: ObjectId,
    owner: ObjectId,
) -> Result<()> {
    if objects.read_typed(block_id, ObjectType::Block)?.is_some() {
        return Ok(());
    }
    Err(PrikkError::Integrity(format!(
        "ref object {owner} targets missing block {block_id}"
    )))
}

fn ensure_ref_path_shape(path: &Path, extension: &str) -> Result<()> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| PrikkError::Integrity("ref path is not valid UTF-8".to_string()))?;
    let expected_len = 64_usize
        .checked_add(extension.len())
        .ok_or_else(|| PrikkError::Integrity("ref extension length overflow".to_string()))?;
    if name.len() != expected_len || !name.ends_with(extension) {
        return Err(unexpected_path("ref path has invalid shape", path));
    }
    Ok(())
}

fn is_temporary_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.contains(".tmp"))
}

fn unexpected_path(label: &str, path: &Path) -> PrikkError {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("<non-UTF-8>");
    PrikkError::Integrity(format!("{label}: {name}"))
}
