//! Pointer-first ref publication and bounded retry state classification.

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectId, RefKind, RefStatePayload, RefUpdatePayload, ascii_fold};

use super::{
    RefPublication, RefStore, log, validate_local_branch_ref, validate_local_tag_ref,
    validate_publication,
};
use crate::layout::RepositoryFormat;
use crate::lock::RefLock;
use crate::object_store::{FileObjectStore, ObjectReader, ObjectWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublicationState {
    Ready,
    PointerLeading,
    Complete,
    LegacyLogLeading,
}

pub(super) fn publish(store: &RefStore, publication: &RefPublication) -> Result<ObjectId> {
    publish_locked(store, publication, false)
}

pub(super) fn finish_interrupted(
    store: &RefStore,
    publication: &RefPublication,
) -> Result<ObjectId> {
    publish_locked(store, publication, true)
}

fn publish_locked(
    store: &RefStore,
    publication: &RefPublication,
    allow_partial_tail_repair: bool,
) -> Result<ObjectId> {
    let update = validate_coherent_publication(publication)?;
    // DC-72: `expected_previous_ref_state_id: None` is this publication's own CAS signal for "no
    // current state" — the same signal `branch create`/`tag create`/first `seal` already use to mean
    // "this ref does not exist yet" (branch.rs, tag.rs, seal.rs). Checked only here, not on every
    // ordinary pointer-update publication, so an existing ref's routine seal never re-scans every ref.
    if publication.expected_previous_ref_state_id.is_none() {
        validate_no_ref_name_collision(store, &publication.ref_name)?;
    }
    let ref_state_id = publication.ref_state.object_id();
    let ref_lock = RefLock::acquire(&store.layout, &publication.ref_name)?;
    super::pointer::remove_candidate_write_temps(&store.layout, &publication.ref_name)?;
    let mut object_store = FileObjectStore::new(store.layout.clone());
    match store.layout.format() {
        RepositoryFormat::CurrentV2 => {
            object_store.write_object(&publication.ref_state)?;
        }
        RepositoryFormat::LegacyV1 => {
            let existing = object_store.read_object(ref_state_id)?;
            if existing.as_ref() != Some(&publication.ref_state) {
                return Err(PrikkError::Integrity(
                    "legacy publication completion requires the exact persisted RefState"
                        .to_string(),
                ));
            }
        }
    }

    let (state, trailing_partial_bytes) = classify_state(store, publication, &update)?;
    match (store.layout.format(), state, allow_partial_tail_repair) {
        (RepositoryFormat::LegacyV1, PublicationState::LegacyLogLeading, true) => {}
        (RepositoryFormat::LegacyV1, _, _) => {
            return Err(PrikkError::UnsupportedFormatVersion(1));
        }
        (RepositoryFormat::CurrentV2, PublicationState::LegacyLogLeading, _) => {
            return Err(PrikkError::Integrity(
                "format-2 ref log must not lead its authoritative pointer".to_string(),
            ));
        }
        (RepositoryFormat::CurrentV2, _, _) => {}
    }
    if trailing_partial_bytes != 0 {
        if !allow_partial_tail_repair
            || state != PublicationState::PointerLeading
            || !log::incomplete_tail_matches(
                &store.layout,
                &publication.ref_name,
                &publication.ref_update,
            )?
        {
            return Err(PrikkError::Integrity(format!(
                "ref {} has an unauthorized incomplete log tail",
                publication.ref_name
            )));
        }
        log::truncate_incomplete_tail(&store.layout, &publication.ref_name)?;
    }
    match state {
        PublicationState::Ready => {
            store.write_ref_pointer_candidate(&publication.ref_name, ref_state_id)?;
            store.ensure_current_matches(
                &publication.ref_name,
                publication.expected_previous_ref_state_id,
            )?;
            store.promote_ref_pointer_candidate(&publication.ref_name)?;
            log::append_log_record(
                &store.layout,
                &publication.ref_name,
                &publication.ref_update,
            )?;
        }
        PublicationState::PointerLeading => {
            log::append_log_record(
                &store.layout,
                &publication.ref_name,
                &publication.ref_update,
            )?;
        }
        PublicationState::Complete => {
            log::append_log_record(
                &store.layout,
                &publication.ref_name,
                &publication.ref_update,
            )?;
        }
        PublicationState::LegacyLogLeading => {
            store.write_ref_pointer_candidate(&publication.ref_name, ref_state_id)?;
            store.ensure_current_matches(
                &publication.ref_name,
                publication.expected_previous_ref_state_id,
            )?;
            store.promote_ref_pointer_candidate(&publication.ref_name)?;
        }
    }
    ensure_agreement(store, publication, &update)?;
    drop(ref_lock);
    Ok(ref_state_id)
}

/// Reject a new ref whose ASCII-folded name collides with an existing ref other than itself
/// (DC-72). Branch and tag namespaces never collide with each other here: every valid ref name
/// begins with the exact literal `heads/` or `tags/` (`validate_local_branch_ref`/
/// `validate_local_tag_ref` require the case-sensitive prefix), so folding the full name keeps the
/// two prefixes apart. Folds through `prikk_object::ascii_fold`, the one shared folding definition
/// (DC-72 design ruling, `rfcs/accepted/DC-72-PATH-SAFETY-CONFORMANCE.md` §3.5) — see its doc comment
/// for the recorded NFC/NFD limitation this inherits.
fn validate_no_ref_name_collision(store: &RefStore, ref_name: &str) -> Result<()> {
    let folded = ascii_fold(ref_name);
    for existing in store.list_ref_pointers()? {
        if existing.ref_name != ref_name && ascii_fold(&existing.ref_name) == folded {
            return Err(PrikkError::InvalidName(format!(
                "case-insensitive ref-name collision involving: {}",
                existing.ref_name
            )));
        }
    }
    Ok(())
}

fn validate_coherent_publication(publication: &RefPublication) -> Result<RefUpdatePayload> {
    validate_publication(publication)?;
    let ref_state = RefStatePayload::decode_canonical(
        &publication.ref_state.canonical_payload,
        publication.ref_state.schema_version,
    )?;
    let update = RefUpdatePayload::decode_canonical(&publication.ref_update.canonical_payload)?;
    let ref_state_id = publication.ref_state.object_id();
    if ref_state.ref_name != publication.ref_name || update.ref_name != publication.ref_name {
        return Err(PrikkError::Integrity(
            "publication ref names do not agree".to_string(),
        ));
    }
    // Kind-aware, now that the ref-state payload is decoded and its name is confirmed to agree
    // with the publication. Makes namespace and kind mutually enforcing: a Tag-kind publication
    // for `heads/...` and a Branch-kind publication for `tags/...` are both rejected here, neither
    // of which the name-only check below did on its own.
    match ref_state.kind {
        RefKind::Branch => {
            validate_local_branch_ref(&publication.ref_name)?;
        }
        RefKind::Tag => {
            validate_local_tag_ref(&publication.ref_name)?;
        }
    }
    if ref_state.previous_ref_state_id != publication.expected_previous_ref_state_id
        || update.old_ref_state_id != publication.expected_previous_ref_state_id
        || update.new_ref_state_id != ref_state_id
        || update.new_target_object_id != ref_state.target_object_id
        || update.update_seq != ref_state.update_seq
    {
        return Err(PrikkError::Integrity(
            "RefState and RefUpdate publication fields do not agree".to_string(),
        ));
    }
    if update.created_at != 0 {
        return Err(PrikkError::Integrity(
            "schema-1 RefUpdate mutation requires created_at == 0".to_string(),
        ));
    }
    Ok(update)
}

fn classify_state(
    store: &RefStore,
    publication: &RefPublication,
    update: &RefUpdatePayload,
) -> Result<(PublicationState, usize)> {
    let current = store.read_current_ref_state_id(&publication.ref_name)?;
    let replay = store.replay_log(&publication.ref_name)?;
    let (log_tip, exact_last, previous_log_tip) = log_position(&replay, publication)?;
    let expected = publication.expected_previous_ref_state_id;
    let proposed = Some(update.new_ref_state_id);
    let state = match (current, log_tip, exact_last, previous_log_tip) {
        (current, tip, false, _) if current == expected && tip == expected => {
            PublicationState::Ready
        }
        (current, tip, false, _) if current == proposed && tip == expected => {
            PublicationState::PointerLeading
        }
        (current, tip, true, previous)
            if current == proposed && tip == proposed && previous == expected =>
        {
            PublicationState::Complete
        }
        (current, tip, true, previous)
            if current == expected && tip == proposed && previous == expected =>
        {
            PublicationState::LegacyLogLeading
        }
        _ => {
            return Err(PrikkError::Integrity(format!(
                "ref {} pointer/log state does not match the expected publication transition",
                publication.ref_name
            )));
        }
    };
    Ok((state, replay.trailing_partial_bytes))
}

fn log_position(
    replay: &super::RefLogReplay,
    publication: &RefPublication,
) -> Result<(Option<ObjectId>, bool, Option<ObjectId>)> {
    let mut previous = None;
    let mut before_last = None;
    for (index, record) in replay.records.iter().enumerate() {
        let update = RefUpdatePayload::decode_canonical(&record.envelope.canonical_payload)?;
        let expected_sequence = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| PrikkError::Integrity("ref-log sequence overflow".to_string()))?;
        if update.ref_name != publication.ref_name
            || update.old_ref_state_id != previous
            || update.update_seq != expected_sequence
            || update.created_at != 0
        {
            return Err(PrikkError::Integrity(format!(
                "ref-log chain diverges for {}",
                publication.ref_name
            )));
        }
        before_last = previous;
        previous = Some(update.new_ref_state_id);
    }
    let exact_last = replay
        .records
        .last()
        .is_some_and(|record| record.envelope == publication.ref_update);
    Ok((previous, exact_last, before_last))
}

fn ensure_agreement(
    store: &RefStore,
    publication: &RefPublication,
    update: &RefUpdatePayload,
) -> Result<()> {
    let current = store.read_current_ref_state_id(&publication.ref_name)?;
    let replay = store.replay_log(&publication.ref_name)?;
    let last = replay.records.last().map(|record| &record.envelope);
    if current != Some(update.new_ref_state_id)
        || replay.trailing_partial_bytes != 0
        || last != Some(&publication.ref_update)
    {
        return Err(PrikkError::Integrity(format!(
            "ref {} pointer/log agreement was not established",
            publication.ref_name
        )));
    }
    Ok(())
}
