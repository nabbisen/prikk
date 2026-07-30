//! Ref-state pointer and ref-log publication primitives.
//!
//! PR-007 introduced the storage mechanics needed before a full seal command exists: a RefState is
//! stored as a normal content-addressed object, the ref file is a durable pointer to that object,
//! and RefUpdate entries are stored inline in an append-only log. The module does not yet perform
//! publication-policy evaluation or patch/block sealing.

mod evidence;
mod log;
mod pointer;
mod publication;
mod verify;

#[cfg(test)]
pub(crate) use log::{
    append_log_record as append_log_record_for_signature_test, encode_log_record_for_test,
};

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType, RefStatePayload, RefUpdatePayload};

use crate::fsutil::{EntryKind, ensure_directory_required, list_directory, promote_file_required};
use crate::layout::RepositoryLayout;
use crate::lock::ActiveLock;
use crate::object_store::{FileObjectStore, ObjectReader};

pub use log::{RefLogRecord, RefLogReplay};
pub use verify::RefPublicationIssue;
pub(crate) use verify::verify_refs;

pub(crate) fn ensure_no_incomplete_publication(layout: &RepositoryLayout) -> Result<()> {
    let verification = verify_refs(layout)?;
    if verification.publication_issues.is_empty()
        && !evidence::has_incomplete_active_cleanup(layout)?
    {
        return Ok(());
    }
    Err(PrikkError::LockConflict(
        "repository mutation is blocked by incomplete ref publication; run verify/doctor and use signer-backed seal retry"
            .to_string(),
    ))
}

/// Diagnostic ref candidate derived from an append-only format-1 ref log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRecoveryCandidate {
    /// Human-readable ref name.
    pub ref_name: String,
    /// RefState ID selected by the latest valid ref-log record.
    pub ref_state_id: ObjectId,
    /// Target Block ID selected by the RefState.
    pub target_object_id: ObjectId,
    /// Update sequence of the latest ref-log record.
    pub update_seq: u64,
}

/// One enumerated ref pointer, for deterministic listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefPointerSummary {
    /// Human-readable ref name recovered from the pointer file body.
    pub ref_name: String,
    /// Current RefState object ID selected by this pointer.
    pub ref_state_id: ObjectId,
}

/// Compatibility result type retained for the now-refused format-1 reconstruction API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRecoveryRepair {
    /// Human-readable ref name.
    pub ref_name: String,
    /// RefState ID reconstructed into the pointer file.
    pub ref_state_id: ObjectId,
    /// Whether a pointer file was written.
    pub wrote_pointer: bool,
}

/// Inputs for a single ref publication primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefPublication {
    /// Human-readable ref name, such as `heads/main`.
    pub ref_name: String,
    /// Expected current RefState ID for CAS. Use `None` to create a new ref.
    pub expected_previous_ref_state_id: Option<ObjectId>,
    /// Signed RefState object envelope to persist before publishing the pointer.
    pub ref_state: ObjectEnvelope,
    /// Signed RefUpdate envelope to append after the ref pointer is durable.
    pub ref_update: ObjectEnvelope,
}

/// File-backed ref-state and ref-log store.
#[derive(Debug, Clone)]
pub struct RefStore {
    layout: RepositoryLayout,
}

impl RefStore {
    /// Create a ref store for a repository layout.
    #[must_use]
    pub fn new(layout: RepositoryLayout) -> Self {
        Self { layout }
    }

    /// Return the repository layout.
    #[must_use]
    pub fn layout(&self) -> &RepositoryLayout {
        &self.layout
    }

    /// Publish a signed RefState with ref-specific locking and CAS.
    pub fn publish(&self, publication: &RefPublication) -> Result<ObjectId> {
        self.layout.require_current_format()?;
        crate::format::validate_object_envelope(self.layout.format(), &publication.ref_state)?;
        crate::format::validate_object_envelope(self.layout.format(), &publication.ref_update)?;
        publication::publish(self, publication)
    }

    /// Finish an exact signer-backed interrupted publication, including a framing-incomplete tail.
    pub fn finish_interrupted_publication(
        &self,
        active_lock: &ActiveLock,
        publication: &RefPublication,
    ) -> Result<ObjectId> {
        self.finish_interrupted_publication_with_cleanup_authorization(active_lock, publication)
            .map(|(ref_state_id, _)| ref_state_id)
    }

    /// Finish an interrupted publication and return legacy cleanup authority when applicable.
    pub fn finish_interrupted_publication_with_cleanup_authorization(
        &self,
        active_lock: &ActiveLock,
        publication: &RefPublication,
    ) -> Result<(
        ObjectId,
        Option<crate::active::LegacyActiveCleanupAuthorization>,
    )> {
        self.layout.validate_format()?;
        active_lock.require_layout(&self.layout)?;
        crate::format::validate_read_schema(self.layout.format(), &publication.ref_state)?;
        crate::format::validate_read_schema(self.layout.format(), &publication.ref_update)?;
        evidence::validate_signer_backed_recovery(&self.layout, publication)?;
        let ref_state_id = publication::finish_interrupted(self, publication)?;
        let authorization = (self.layout.format() == crate::layout::RepositoryFormat::LegacyV1)
            .then(|| crate::active::authorize_legacy_active_cleanup(&self.layout));
        Ok((ref_state_id, authorization))
    }

    #[cfg(test)]
    pub(crate) fn finish_interrupted_publication_for_test(
        &self,
        publication: &RefPublication,
    ) -> Result<ObjectId> {
        publication::finish_interrupted(self, publication)
    }

    /// Read the current RefState object ID for a ref name.
    pub fn read_current_ref_state_id(&self, ref_name: &str) -> Result<Option<ObjectId>> {
        let path = self.layout.ref_pointer_path(ref_name);
        let Some(pointer) = pointer::read_ref_pointer(&self.layout, &path)? else {
            return Ok(None);
        };
        if pointer.ref_name != ref_name {
            return Err(PrikkError::Integrity(format!(
                "ref pointer name mismatch: expected {ref_name}, got {}",
                pointer.ref_name
            )));
        }
        Ok(Some(pointer.ref_state_id))
    }

    /// Replay the inline ref-update log for a ref name.
    pub fn replay_log(&self, ref_name: &str) -> Result<RefLogReplay> {
        log::replay_log(&self.layout, ref_name)
    }

    /// Enumerate every ref pointer under `by-id/`, sorted by name. `by-id/` is the complete set of
    /// ref pointers; logs, locks, and tmp files live elsewhere and are not pointers.
    pub fn list_ref_pointers(&self) -> Result<Vec<RefPointerSummary>> {
        let by_id_dir = self.layout.refs_dir().join("by-id");
        let relative = self.layout.repository_relative(&by_id_dir)?;
        let entries = list_directory(self.layout.repository_mutation_root(), &relative)?;
        let mut summaries = Vec::with_capacity(entries.len());
        for entry in entries {
            if entry.kind != EntryKind::Regular {
                continue;
            }
            let Some(name) = entry.name.to_str() else {
                return Err(PrikkError::Integrity(
                    "ref pointer file name is not valid UTF-8".to_string(),
                ));
            };
            if !name.ends_with(".ref") {
                continue;
            }
            let path = by_id_dir.join(name);
            let Some(pointer) = pointer::read_ref_pointer(&self.layout, &path)? else {
                return Err(PrikkError::Integrity(format!(
                    "ref pointer file disappeared during listing: {name}"
                )));
            };
            summaries.push(RefPointerSummary {
                ref_name: pointer.ref_name,
                ref_state_id: pointer.ref_state_id,
            });
        }
        summaries.sort_by(|left, right| left.ref_name.cmp(&right.ref_name));
        Ok(summaries)
    }

    /// Return a diagnostic candidate when the pointer is missing but the format-1 log is valid.
    pub fn recoverable_missing_ref(&self, ref_name: &str) -> Result<Option<RefRecoveryCandidate>> {
        if self.read_current_ref_state_id(ref_name)?.is_some() {
            return Ok(None);
        }
        let replay = self.replay_log(ref_name)?;
        if replay.records.is_empty() {
            return Ok(None);
        }
        if replay.trailing_partial_bytes != 0 {
            return Err(PrikkError::Integrity(format!(
                "ref log for {ref_name} has trailing partial bytes"
            )));
        }
        let object_store = FileObjectStore::new(self.layout.clone());
        let mut previous_ref_state_id = None;
        let mut latest = None;
        for record in &replay.records {
            let update = RefUpdatePayload::decode_canonical(&record.envelope.canonical_payload)?;
            if update.ref_name != ref_name {
                return Err(PrikkError::Integrity(format!(
                    "ref-log record name mismatch: expected {ref_name}, got {}",
                    update.ref_name
                )));
            }
            if update.old_ref_state_id != previous_ref_state_id {
                return Err(PrikkError::Integrity(format!(
                    "ref-log chain mismatch for {ref_name} at update {}",
                    update.update_seq
                )));
            }
            let ref_state = verified_ref_state_payload(
                &object_store,
                update.new_ref_state_id,
                ref_name,
                update.new_target_object_id,
            )?;
            if ref_state.previous_ref_state_id != update.old_ref_state_id {
                return Err(PrikkError::Integrity(format!(
                    "RefState previous link disagrees with RefUpdate for {ref_name}"
                )));
            }
            if ref_state.update_seq != update.update_seq {
                return Err(PrikkError::Integrity(format!(
                    "RefState update sequence disagrees with RefUpdate for {ref_name}"
                )));
            }
            previous_ref_state_id = Some(update.new_ref_state_id);
            latest = Some(update);
        }
        let Some(update) = latest else {
            return Ok(None);
        };
        Ok(Some(RefRecoveryCandidate {
            ref_name: ref_name.to_string(),
            ref_state_id: update.new_ref_state_id,
            target_object_id: update.new_target_object_id,
            update_seq: update.update_seq,
        }))
    }

    /// Refuse unsigned reconstruction of a missing format-1 ref pointer.
    pub fn reconstruct_missing_ref_from_log(&self, ref_name: &str) -> Result<RefRecoveryRepair> {
        let _ = ref_name;
        Err(PrikkError::Integrity(
            "format-1 missing-pointer reconstruction is unsupported in 0.18.0".to_string(),
        ))
    }

    fn ensure_current_matches(&self, ref_name: &str, expected: Option<ObjectId>) -> Result<()> {
        let current = self.read_current_ref_state_id(ref_name)?;
        if current != expected {
            return Err(PrikkError::LockConflict(format!(
                "ref CAS mismatch for {ref_name}: expected {:?}, got {:?}",
                expected, current
            )));
        }
        Ok(())
    }

    fn write_ref_pointer_candidate(&self, ref_name: &str, ref_state_id: ObjectId) -> Result<()> {
        pointer::write_ref_pointer_candidate(&self.layout, ref_name, ref_state_id)
    }

    fn promote_ref_pointer_candidate(&self, ref_name: &str) -> Result<()> {
        let candidate = self
            .layout
            .repository_relative(&self.layout.ref_tmp_path(ref_name))?;
        let pointer = self
            .layout
            .repository_relative(&self.layout.ref_pointer_path(ref_name))?;
        let Some(parent) = pointer.parent() else {
            return Err(PrikkError::Io(
                "ref pointer path has no parent directory".to_string(),
            ));
        };
        ensure_directory_required(self.layout.repository_mutation_root(), parent)?;
        promote_file_required(self.layout.repository_mutation_root(), &candidate, &pointer)
    }
}

fn verified_ref_state_payload(
    object_store: &FileObjectStore,
    ref_state_id: ObjectId,
    ref_name: &str,
    target_object_id: ObjectId,
) -> Result<RefStatePayload> {
    let Some(envelope) = object_store.read_typed(ref_state_id, ObjectType::RefState)? else {
        return Err(PrikkError::Integrity(format!(
            "missing RefState object for ref recovery: {ref_state_id}"
        )));
    };
    if envelope.signatures.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} is unsigned"
        )));
    }
    let payload =
        RefStatePayload::decode_canonical(&envelope.canonical_payload, envelope.schema_version)?;
    if payload.ref_name != ref_name {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} name mismatch: expected {ref_name}, got {}",
            payload.ref_name
        )));
    }
    if payload.target_object_id != target_object_id {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} target disagrees with ref log for {ref_name}"
        )));
    }
    let Some(target) = object_store.read_object(target_object_id)? else {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} targets missing block {target_object_id}"
        )));
    };
    if target.object_type != ObjectType::Block {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} targets {}, expected block",
            target.object_type
        )));
    }
    Ok(payload)
}

pub(crate) fn validate_publication(publication: &RefPublication) -> Result<()> {
    require_signed_type(&publication.ref_state, ObjectType::RefState)?;
    require_signed_type(&publication.ref_update, ObjectType::RefUpdate)?;
    publication.ref_state.validate_strict()?;
    publication.ref_update.validate_strict()?;
    Ok(())
}

pub(crate) fn require_signed_type(
    envelope: &ObjectEnvelope,
    object_type: ObjectType,
) -> Result<()> {
    if envelope.object_type != object_type {
        return Err(PrikkError::ObjectTypeMismatch {
            expected: object_type.to_string(),
            actual: envelope.object_type.to_string(),
        });
    }
    if envelope.signatures.is_empty() {
        return Err(PrikkError::InvalidSignature(format!(
            "{object_type} publication envelope must be signed"
        )));
    }
    envelope.validate()
}

/// Validate a local branch ref name and return its canonical identity string.
pub fn validate_local_branch_ref(ref_name: &str) -> Result<String> {
    if ref_name.is_empty() {
        return Err(PrikkError::InvalidName(
            "ref name must not be empty".to_string(),
        ));
    }
    if ref_name.starts_with("tags/")
        || ref_name.starts_with("remotes/")
        || ref_name.starts_with("rollback/")
    {
        return Err(PrikkError::InvalidName(format!(
            "ref namespace is reserved: {ref_name}"
        )));
    }
    if !ref_name.starts_with("heads/") {
        return Err(PrikkError::InvalidName(format!(
            "ref {ref_name} is not a local branch ref; expected heads/<name>"
        )));
    }
    let branch = &ref_name["heads/".len()..];
    if branch.is_empty() {
        return Err(PrikkError::InvalidName(
            "branch ref must include a name after heads/".to_string(),
        ));
    }
    if ref_name.chars().any(|ch| ch == '\0' || ch.is_control()) {
        return Err(PrikkError::InvalidName(format!(
            "ref {ref_name} contains a forbidden control character"
        )));
    }
    if branch.starts_with('/') || branch.ends_with('/') || branch.contains("//") {
        return Err(PrikkError::InvalidName(format!(
            "branch ref {ref_name} contains an empty path component"
        )));
    }
    if branch
        .split('/')
        .any(|component| component == "." || component == "..")
    {
        return Err(PrikkError::InvalidName(format!(
            "branch ref {ref_name} contains a traversal component"
        )));
    }
    Ok(ref_name.to_string())
}

/// Validate a local tag ref name and return its canonical identity string.
///
/// Mirrors `validate_local_branch_ref` with the prefix requirement inverted: `tags/` required,
/// `heads/`/`remotes/`/`rollback/` reserved. Deliberately carries no case-collision rule —
/// `validate_local_branch_ref` does not have one either (`tags/V1` and `tags/v1` both pass and
/// coexist as distinct refs, same as branches), and a stricter rule for tags alone than branches
/// would be arbitrary. That gap is real but is NFR-SEC-03's, unmet for both namespaces, and tracked
/// separately rather than closed asymmetrically here.
pub fn validate_local_tag_ref(ref_name: &str) -> Result<String> {
    if ref_name.is_empty() {
        return Err(PrikkError::InvalidName(
            "ref name must not be empty".to_string(),
        ));
    }
    if ref_name.starts_with("heads/")
        || ref_name.starts_with("remotes/")
        || ref_name.starts_with("rollback/")
    {
        return Err(PrikkError::InvalidName(format!(
            "ref namespace is reserved: {ref_name}"
        )));
    }
    if !ref_name.starts_with("tags/") {
        return Err(PrikkError::InvalidName(format!(
            "ref {ref_name} is not a local tag ref; expected tags/<name>"
        )));
    }
    let tag = &ref_name["tags/".len()..];
    if tag.is_empty() {
        return Err(PrikkError::InvalidName(
            "tag ref must include a name after tags/".to_string(),
        ));
    }
    if ref_name.chars().any(|ch| ch == '\0' || ch.is_control()) {
        return Err(PrikkError::InvalidName(format!(
            "ref {ref_name} contains a forbidden control character"
        )));
    }
    if tag.starts_with('/') || tag.ends_with('/') || tag.contains("//") {
        return Err(PrikkError::InvalidName(format!(
            "tag ref {ref_name} contains an empty path component"
        )));
    }
    if tag
        .split('/')
        .any(|component| component == "." || component == "..")
    {
        return Err(PrikkError::InvalidName(format!(
            "tag ref {ref_name} contains a traversal component"
        )));
    }
    Ok(ref_name.to_string())
}

#[cfg(test)]
mod tests;
