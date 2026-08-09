//! Received (imported) ref bookkeeping — DC-78 §D4's distinct `remotes/` namespace.
//!
//! A received ref is deliberately **not** a `refs/by-id/` pointer. `refs/by-id/`'s own consistency
//! check (a RefState object's embedded `ref_name` must equal its pointer's declared name — enforced
//! by `refs/verify/scan.rs`) cannot be satisfied by a renamed local identifier: the RefState object
//! keeps the *origin's own* embedded ref name (its content-addressed identity and signature would be
//! invalidated by editing it), so a pointer declaring `remotes/heads/main` could never agree with a
//! payload that says `heads/main`. Storing received refs in their own directory, under their own
//! small pointer format, sidesteps that conflict entirely rather than patching the check to allow it
//! — the latter would be new verification-path surface, which §D6 rules out.
//!
//! This file format is never read by `verify_repository`. Every object a received pointer leads to
//! (RefState, Block, Patch, Blob, Attestation) is an ordinary object-store entry, checked exactly
//! like any other by the existing type-based object scan — §D6's "no new verification path" holds
//! because there genuinely is none: this module only makes received tips *discoverable* by name.

use std::path::PathBuf;

use prikk_error::{PrikkError, Result};
use prikk_object::ObjectId;

use crate::byte_cursor::ByteCursor;
use crate::file_codec::push_bytes_u64;
use crate::fsutil::{
    EntryKind, ensure_directory_required, inspect_entry, list_directory, read_file_if_exists,
    write_file_atomically,
};
use crate::layout::RepositoryLayout;

const RECEIVED_POINTER_MAGIC: &[u8; 8] = b"PRECV001";

/// One received ref pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedPointer {
    /// Local logical name, always `remotes/<origin ref name>`.
    pub ref_name: String,
    /// The received tip's RefState object id.
    pub ref_state_id: ObjectId,
}

/// Validate a received ref name: the reserved `remotes/` namespace, required rather than rejected
/// (the mirror image of `validate_local_branch_ref`/`validate_local_tag_ref`, which reject it).
pub fn validate_received_ref(ref_name: &str) -> Result<()> {
    if ref_name.is_empty() {
        return Err(PrikkError::InvalidName(
            "ref name must not be empty".to_string(),
        ));
    }
    if !ref_name.starts_with("remotes/") {
        return Err(PrikkError::InvalidName(format!(
            "ref {ref_name} is not a received ref; expected remotes/<name>"
        )));
    }
    let rest = &ref_name["remotes/".len()..];
    if rest.is_empty() {
        return Err(PrikkError::InvalidName(
            "received ref must include a name after remotes/".to_string(),
        ));
    }
    if ref_name.chars().any(|ch| ch == '\0' || ch.is_control()) {
        return Err(PrikkError::InvalidName(format!(
            "ref {ref_name} contains a forbidden control character"
        )));
    }
    if rest.starts_with('/') || rest.ends_with('/') || rest.contains("//") {
        return Err(PrikkError::InvalidName(format!(
            "received ref {ref_name} contains an empty path component"
        )));
    }
    if rest
        .split('/')
        .any(|component| component == "." || component == "..")
    {
        return Err(PrikkError::InvalidName(format!(
            "received ref {ref_name} contains a traversal component"
        )));
    }
    Ok(())
}

fn received_dir(layout: &RepositoryLayout) -> PathBuf {
    layout.refs_dir().join("received")
}

fn received_pointer_path(layout: &RepositoryLayout, ref_name: &str) -> PathBuf {
    received_dir(layout).join(format!(
        "{}.ref",
        crate::layout::ref_name_storage_key(ref_name)
    ))
}

/// Write (or overwrite) a received ref's pointer. Each import replaces the prior received state for
/// that name outright — there is no CAS and no merge between two received histories under one name;
/// D4 explicitly leaves remote-tracking-ref semantics out of scope, so a re-import is simply "this is
/// what I have now," and turning that into real local history remains the operator's own deliberate
/// merge, using machinery that already exists.
pub(crate) fn write_received_pointer(
    layout: &RepositoryLayout,
    ref_name: &str,
    ref_state_id: ObjectId,
) -> Result<()> {
    validate_received_ref(ref_name)?;
    let dir = layout.repository_relative(&received_dir(layout))?;
    ensure_directory_required(layout.repository_mutation_root(), &dir)?;
    let path = layout.repository_relative(&received_pointer_path(layout, ref_name))?;
    let bytes = encode_received_pointer(ref_name, ref_state_id)?;
    write_file_atomically(layout.repository_mutation_root(), &path, &bytes)
}

/// Read a received ref's current pointer, if one has been imported.
pub fn read_received_pointer(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<Option<ReceivedPointer>> {
    validate_received_ref(ref_name)?;
    let path = layout.repository_relative(&received_pointer_path(layout, ref_name))?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &path)? else {
        return Ok(None);
    };
    decode_received_pointer(&bytes).map(Some)
}

/// Enumerate every received ref pointer, sorted by name — the received-namespace counterpart of
/// `RefStore::list_ref_pointers`.
pub fn list_received_pointers(layout: &RepositoryLayout) -> Result<Vec<ReceivedPointer>> {
    let dir = layout.repository_relative(&received_dir(layout))?;
    if inspect_entry(layout.repository_mutation_root(), &dir)?.is_none() {
        return Ok(Vec::new());
    }
    let entries = list_directory(layout.repository_mutation_root(), &dir)?;
    let mut pointers = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.kind != EntryKind::Regular {
            continue;
        }
        let Some(name) = entry.name.to_str() else {
            return Err(PrikkError::Integrity(
                "received ref pointer file name is not valid UTF-8".to_string(),
            ));
        };
        if !name.ends_with(".ref") {
            continue;
        }
        let path = dir.join(name);
        let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &path)? else {
            return Err(PrikkError::Integrity(format!(
                "received ref pointer file disappeared during listing: {name}"
            )));
        };
        pointers.push(decode_received_pointer(&bytes)?);
    }
    pointers.sort_by(|left, right| left.ref_name.cmp(&right.ref_name));
    Ok(pointers)
}

fn encode_received_pointer(ref_name: &str, ref_state_id: ObjectId) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(RECEIVED_POINTER_MAGIC);
    push_bytes_u64(&mut out, ref_name.as_bytes())?;
    out.extend_from_slice(ref_state_id.as_bytes());
    Ok(out)
}

fn decode_received_pointer(bytes: &[u8]) -> Result<ReceivedPointer> {
    let mut cursor = ByteCursor::new(bytes);
    let magic = cursor.read_array::<8>()?;
    if &magic != RECEIVED_POINTER_MAGIC {
        return Err(PrikkError::MalformedData(
            "invalid received ref pointer magic".to_string(),
        ));
    }
    let ref_name_bytes = cursor.read_bytes_u64()?;
    let ref_name = String::from_utf8(ref_name_bytes).map_err(|err| {
        PrikkError::MalformedData(format!("invalid received ref name utf-8: {err}"))
    })?;
    let ref_state_id = ObjectId::from_bytes(cursor.read_array::<32>()?);
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in received ref pointer".to_string(),
        ));
    }
    Ok(ReceivedPointer {
        ref_name,
        ref_state_id,
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
