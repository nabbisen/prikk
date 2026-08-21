//! RFC 116 stage 2 (design-v1.md N1; handoff §1, §2): the `PSYNCSU1` sync summary -- one message,
//! every local `heads/*` ref, each with its own [`PatchSetDigest`] and patch count. Answers "are we
//! the same?" without moving a single patch id (design §1.1): the steady-state case, two
//! repositories already in sync, costs a few hundred bytes rather than 32 bytes per patch.
//!
//! **Branches only.** `remotes/*` never appears: those pointers live in the received-index, not the
//! ordinary ref-pointer index [`RefStore::list_ref_pointers`] enumerates. `tags/*` **is** filtered
//! out explicitly and deliberately, not by oversight -- see the parent module doc.
//!
//! **Representational, not frozen** (RFC 114 §3): carries no identity of its own.
//!
//! **Constructs no `RecognitionClaimPayload`** (handoff §6) -- only ref names, counts, and digests.

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectType, RefKind, RefStatePayload};

use crate::byte_cursor::ByteCursor;
use crate::file_codec::{push_string_u16, push_u64};
use crate::fsutil::len_to_u64;
use crate::layout::RepositoryLayout;
use crate::object_store::{ObjectReadSnapshot, ObjectReader};
use crate::patch_set_digest::{
    PatchSetDigest, compute_patch_set_digest, patch_ids_reachable_from_block,
};
use crate::refs::RefStore;

const SYNC_SUMMARY_MAGIC: &[u8; 8] = b"PSYNCSU1";

/// DC-86 bound on the summary's declared ref count, checked before the section is allocated.
pub const DEFAULT_SYNC_SUMMARY_MAX_REF_COUNT: usize = 100_000;

/// DC-86 bound on the summary's total encoded byte length, checked before decoding starts.
pub const DEFAULT_SYNC_SUMMARY_MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;

/// One ref's own entry in a decoded sync summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSummaryRefEntry {
    /// The branch ref's name, e.g. `heads/main`.
    pub ref_name: String,
    /// That ref's own patch-set digest, over its full reachable closure.
    pub digest: PatchSetDigest,
    /// That ref's own reachable patch count -- informational only; the digest already commits to
    /// the exact set, not merely its size, so this is never re-verified against it.
    pub patch_count: u64,
}

/// Build a `PSYNCSU1` sync summary covering every local `heads/*` ref, in
/// [`RefStore::list_ref_pointers`]'s own sorted-by-name order. `remotes/*` and `tags/*` are
/// excluded -- see the module doc. A repository with no `heads/*` ref at all still encodes validly,
/// as a summary declaring zero refs.
pub fn build_sync_summary(layout: &RepositoryLayout) -> Result<Vec<u8>> {
    let ref_store = RefStore::new(layout.clone());
    let object_store = ObjectReadSnapshot::open(layout)?;
    let mut entries: Vec<(String, PatchSetDigest, u64)> = Vec::new();
    for pointer in ref_store.list_ref_pointers()? {
        if !pointer.ref_name.starts_with("heads/") {
            continue;
        }
        let envelope = object_store
            .read_typed(pointer.ref_state_id, ObjectType::RefState)?
            .ok_or_else(|| {
                PrikkError::Integrity(format!(
                    "ref {} names missing RefState {}",
                    pointer.ref_name, pointer.ref_state_id
                ))
            })?;
        let payload = RefStatePayload::decode_canonical(
            &envelope.canonical_payload,
            envelope.schema_version,
        )?;
        if payload.kind != RefKind::Branch {
            return Err(PrikkError::Integrity(format!(
                "ref {} is under heads/ but its RefState kind is not Branch",
                pointer.ref_name
            )));
        }
        let patch_ids = patch_ids_reachable_from_block(&object_store, payload.target_object_id)?;
        let digest = compute_patch_set_digest(&patch_ids)?;
        let patch_count = len_to_u64(patch_ids.len())?;
        entries.push((pointer.ref_name, digest, patch_count));
    }

    let mut out = Vec::new();
    out.extend_from_slice(SYNC_SUMMARY_MAGIC);
    push_u64(&mut out, len_to_u64(entries.len())?);
    for (ref_name, digest, patch_count) in &entries {
        push_string_u16(&mut out, ref_name)?;
        out.extend_from_slice(&digest.0);
        push_u64(&mut out, *patch_count);
    }
    Ok(out)
}

/// Decode a `PSYNCSU1` sync summary structurally. Bounds the total byte length before touching the
/// bytes at all, then the declared ref count before allocating, the same DC-86 shape
/// `decode_exchange_artifact` follows. Performs no cross-entry checks and no comparison against
/// this repository's own refs -- a caller compares the returned entries against its own local
/// digests itself; that comparison is not this format's job (the "one computation" this stage
/// builds is the delta, [`super::compute_sync_delta`], not a summary-comparison routine).
pub fn decode_sync_summary(
    bytes: &[u8],
    max_total_bytes: usize,
    max_ref_count: usize,
) -> Result<Vec<SyncSummaryRefEntry>> {
    if bytes.len() > max_total_bytes {
        return Err(PrikkError::MalformedData(format!(
            "sync summary is {} bytes, over the configured limit of {max_total_bytes} bytes",
            bytes.len()
        )));
    }
    let mut cursor = ByteCursor::new(bytes);
    let magic = cursor.read_array::<8>()?;
    if &magic != SYNC_SUMMARY_MAGIC {
        return Err(PrikkError::MalformedData(
            "invalid sync summary magic".to_string(),
        ));
    }
    let ref_count = cursor.read_u64()?;
    if ref_count > len_to_u64(max_ref_count)? {
        return Err(PrikkError::MalformedData(format!(
            "sync summary declares {ref_count} refs, over the configured limit of {max_ref_count}"
        )));
    }
    let mut entries = Vec::new();
    for _ in 0..ref_count {
        let ref_name = cursor.read_string_u16()?;
        let digest = PatchSetDigest(cursor.read_array::<32>()?);
        let patch_count = cursor.read_u64()?;
        entries.push(SyncSummaryRefEntry {
            ref_name,
            digest,
            patch_count,
        });
    }
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in sync summary".to_string(),
        ));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests;
