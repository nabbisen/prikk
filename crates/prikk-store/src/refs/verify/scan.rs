//! Ref pointer/log scanning and structural chain validation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};
use prikk_object::{
    ObjectEnvelope, ObjectId, ObjectType, RefKind, RefStatePayload, RefUpdatePayload, TagPayload,
};

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

/// Outcome of attempting to read one pointer or log file (DC-95 Stage 2 Level 2). File identity is
/// a SHA-256 hash of the ref name (`ref_name_storage_key`) -- not reversible -- so a file whose
/// content never decoded far enough to reveal its own claimed ref name genuinely has no identity to
/// report beyond its path. A file that *did* reveal its name before some later check on it failed
/// (e.g. "non-canonical ref pointer path") is still reported this way, by its path, not its claimed
/// name: the claim comes from a file this check has already decided not to trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefFileStatus {
    /// The file was read and validated successfully.
    Evaluated {
        /// The ref name it resolved to.
        ref_name: String,
    },
    /// Some check for this specific file failed.
    Failed {
        /// The error the check raised.
        message: String,
    },
}

/// One pointer or log file's resolved outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefFileOutcome {
    /// The file's path.
    pub path: PathBuf,
    /// How this file's own read/validation resolved.
    pub status: RefFileStatus,
}

pub(super) fn read_pointers(
    layout: &RepositoryLayout,
    objects: &FileObjectStore,
) -> Result<(BTreeMap<String, PointerState>, Vec<RefFileOutcome>)> {
    let dir = layout.refs_dir().join("by-id");
    let mut pointers = BTreeMap::new();
    let mut outcomes = Vec::new();
    let relative_dir = layout.repository_relative(&dir)?;
    for entry in list_directory(layout.repository_mutation_root(), &relative_dir)? {
        let path = dir.join(&entry.name);
        if entry.kind != EntryKind::Regular {
            return Err(unexpected_path("directory in ref pointer directory", &path));
        }
        if is_temporary_path(&path) {
            continue;
        }
        // DC-95 Stage 2 Level 2: this pointer file's own failure is caught here, at the item
        // boundary, rather than propagated -- every *other* pointer file is still attempted.
        // "Duplicate pointer identity" stays a hard error: DC-95 Stage 1 round 6 ruled it provably
        // unreachable today (needs a genuine SHA-256 collision), so its containment shape is moot.
        match read_one_pointer(layout, objects, &path) {
            Ok((ref_name, state)) => {
                if pointers.insert(ref_name.clone(), state).is_some() {
                    return Err(PrikkError::Integrity(format!(
                        "duplicate pointer identity for {ref_name}"
                    )));
                }
                outcomes.push(RefFileOutcome {
                    path,
                    status: RefFileStatus::Evaluated { ref_name },
                });
            }
            Err(err) => {
                outcomes.push(RefFileOutcome {
                    path,
                    status: RefFileStatus::Failed {
                        message: err.to_string(),
                    },
                });
            }
        }
    }
    Ok((pointers, outcomes))
}

fn read_one_pointer(
    layout: &RepositoryLayout,
    objects: &FileObjectStore,
    path: &Path,
) -> Result<(String, PointerState)> {
    ensure_ref_path_shape(path, ".ref")?;
    let pointer = read_ref_pointer(layout, path)?.ok_or_else(|| {
        PrikkError::Integrity("ref pointer disappeared during verification".to_string())
    })?;
    if path != layout.ref_pointer_path(&pointer.ref_name) {
        return Err(unexpected_path("non-canonical ref pointer", path));
    }
    let payload = verified_ref_state_payload(objects, pointer.ref_state_id)?;
    if payload.ref_name != pointer.ref_name {
        return Err(PrikkError::Integrity(format!(
            "RefState {} name differs from pointer ref {}",
            pointer.ref_state_id, pointer.ref_name
        )));
    }
    ensure_ref_target_valid(
        objects,
        payload.kind,
        payload.target_object_id,
        pointer.ref_state_id,
    )?;
    Ok((
        pointer.ref_name.clone(),
        PointerState {
            id: pointer.ref_state_id,
            payload,
        },
    ))
}

#[allow(clippy::type_complexity)]
pub(super) fn read_logs(
    layout: &RepositoryLayout,
    objects: &FileObjectStore,
    pointers: &BTreeMap<String, PointerState>,
) -> Result<(
    BTreeMap<String, LogState>,
    usize,
    Vec<RefLogEnvelope>,
    Vec<RefFileOutcome>,
)> {
    let dir = layout.refs_dir().join("logs");
    let mut logs = BTreeMap::new();
    let mut total = 0_usize;
    let mut envelopes = Vec::new();
    let mut outcomes = Vec::new();
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
        // DC-95 Stage 2 Level 2: this log file's own failure is caught here, at the item boundary,
        // rather than propagated -- every *other* log file is still attempted. "Duplicate ref-log
        // identity" stays a hard error for the same reason as pointers above.
        match read_one_log(layout, objects, &path, &pointer_names) {
            Ok(None) => {
                // Legitimately empty, no trailing bytes -- not an item at all, matching the
                // pre-Level-2 `continue`.
            }
            Ok(Some((ref_name, state, record_envelopes))) => {
                total = match total.checked_add(record_envelopes.len()) {
                    Some(value) => value,
                    None => {
                        return Err(PrikkError::Integrity("ref-log count overflow".to_string()));
                    }
                };
                envelopes.extend(record_envelopes);
                if logs.insert(ref_name.clone(), state).is_some() {
                    return Err(PrikkError::Integrity(format!(
                        "duplicate ref-log identity for {ref_name}"
                    )));
                }
                outcomes.push(RefFileOutcome {
                    path,
                    status: RefFileStatus::Evaluated { ref_name },
                });
            }
            Err(err) => {
                outcomes.push(RefFileOutcome {
                    path,
                    status: RefFileStatus::Failed {
                        message: err.to_string(),
                    },
                });
            }
        }
    }
    envelopes.sort_by(|left, right| {
        left.ref_name
            .as_bytes()
            .cmp(right.ref_name.as_bytes())
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    Ok((logs, total, envelopes, outcomes))
}

#[allow(clippy::type_complexity)]
fn read_one_log(
    layout: &RepositoryLayout,
    objects: &FileObjectStore,
    path: &Path,
    pointer_names: &[String],
) -> Result<Option<(String, LogState, Vec<RefLogEnvelope>)>> {
    ensure_ref_path_shape(path, ".log")?;
    let relative = layout.repository_relative(path)?;
    let bytes = read_file_required(layout.repository_mutation_root(), &relative)?;
    let replay = decode_log_file_bytes(layout.format(), &bytes)?;
    // RFC 102 Stage 2: isolate-and-continue reading means a damaged record no longer makes
    // `decode_log_file_bytes` return `Err` -- this file's own item containment (the caller,
    // `read_logs`, already catches an `Err` here into `RefFileStatus::Failed`) must be preserved
    // explicitly, or `replay.records.is_empty()` below could read a log with only a damaged record
    // as legitimately empty rather than as a file whose content could not be fully trusted. Kept at
    // file granularity (not the WAL's per-record `wal_record_outcomes` exposure) deliberately --
    // see the review submission for the scope reasoning.
    if replay.has_item_failure() {
        let failed = replay
            .record_outcomes
            .iter()
            .filter_map(|outcome| match &outcome.status {
                crate::refs::log::RefLogRecordStatus::Failed { message } => {
                    Some(format!("offset {}: {message}", outcome.offset))
                }
                crate::refs::log::RefLogRecordStatus::Evaluated => None,
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(PrikkError::Integrity(format!(
            "ref log has damaged record(s): {failed}"
        )));
    }
    if replay.records.is_empty() {
        if replay.trailing_partial_bytes == 0 {
            return Ok(None);
        }
        let name = pointer_names
            .iter()
            .find(|name| layout.ref_log_path(name) == path)
            .cloned()
            .ok_or_else(|| unexpected_path("unowned partial ref log", path))?;
        return Ok(Some((
            name,
            LogState {
                tip: None,
                previous_tip: None,
                record_count: 0,
                trailing_partial_bytes: replay.trailing_partial_bytes,
                has_legacy_timestamp: false,
            },
            Vec::new(),
        )));
    }
    let (name, state) = validate_log(layout, objects, path, &replay)?;
    let mut record_envelopes = Vec::with_capacity(replay.records.len());
    for (index, record) in replay.records.iter().enumerate() {
        let sequence = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| PrikkError::Integrity("ref-log sequence overflow".to_string()))?;
        record_envelopes.push(RefLogEnvelope {
            ref_name: name.clone(),
            sequence,
            envelope: record.envelope.clone(),
        });
    }
    Ok(Some((name, state, record_envelopes)))
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
    ensure_ref_target_valid(
        objects,
        state.kind,
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
    RefStatePayload::decode_canonical(&envelope.canonical_payload, envelope.schema_version)
}

/// Kind-aware ref-target validation, shared by both the pointer scan (`read_pointers`) and the
/// ref-log scan (`verify_update`), which must agree: `publication.rs`'s coherence check requires
/// `RefUpdatePayload.new_target_object_id == RefStatePayload.target_object_id`, so a log record's
/// target and its pointer's target are the identical value for the identical kind. `RefKind::Branch`
/// must target a `Block` directly; `RefKind::Tag` must target a `Tag` object whose own
/// `target_block_id` is a `Block` — the two-hop indirection §6.6 requires.
fn ensure_ref_target_valid(
    objects: &FileObjectStore,
    kind: RefKind,
    target_object_id: ObjectId,
    owner: ObjectId,
) -> Result<()> {
    match kind {
        RefKind::Branch => ensure_block_exists(objects, target_object_id, owner),
        RefKind::Tag => {
            let tag_envelope = objects
                .read_typed(target_object_id, ObjectType::Tag)?
                .ok_or_else(|| {
                    PrikkError::Integrity(format!(
                        "ref object {owner} targets missing tag {target_object_id}"
                    ))
                })?;
            let tag_payload = TagPayload::decode_canonical(&tag_envelope.canonical_payload)?;
            ensure_block_exists(objects, tag_payload.target_block_id, owner)
        }
    }
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
