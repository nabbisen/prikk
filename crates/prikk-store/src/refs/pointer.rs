//! Ref pointer file codec.

use std::fs;
use std::path::Path;

use prikk_error::{PrikkError, Result};
use prikk_object::ObjectId;

use crate::byte_cursor::ByteCursor;
use crate::file_codec::push_bytes_u64;
use crate::fsutil::write_file_atomically;
use crate::layout::RepositoryLayout;

const REF_POINTER_MAGIC: &[u8; 8] = b"PREFPTR1";

/// Decoded ref pointer file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefPointer {
    /// Human-readable ref name stored inside the pointer.
    pub ref_name: String,
    /// RefState object ID currently selected by this ref.
    pub ref_state_id: ObjectId,
}

/// Read a ref pointer file.
pub(crate) fn read_ref_pointer(path: &Path) -> Result<RefPointer> {
    let bytes = fs::read(path)?;
    decode_ref_pointer(&bytes)
}

/// Write a candidate ref pointer file and fsync it.
pub(crate) fn write_ref_pointer_candidate(
    layout: &RepositoryLayout,
    ref_name: &str,
    ref_state_id: ObjectId,
) -> Result<()> {
    let candidate = layout.ref_tmp_path(ref_name);
    let bytes = encode_ref_pointer(ref_name, ref_state_id)?;
    write_file_atomically(&candidate, &bytes)
}

fn encode_ref_pointer(ref_name: &str, ref_state_id: ObjectId) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(REF_POINTER_MAGIC);
    push_bytes_u64(&mut out, ref_name.as_bytes())?;
    out.extend_from_slice(ref_state_id.as_bytes());
    Ok(out)
}

fn decode_ref_pointer(bytes: &[u8]) -> Result<RefPointer> {
    let mut cursor = ByteCursor::new(bytes);
    let magic = cursor.read_array::<8>()?;
    if &magic != REF_POINTER_MAGIC {
        return Err(PrikkError::MalformedData("invalid ref pointer magic".to_string()));
    }
    let ref_name_bytes = cursor.read_bytes_u64()?;
    let ref_name = String::from_utf8(ref_name_bytes)
        .map_err(|err| PrikkError::MalformedData(format!("invalid ref name utf-8: {err}")))?;
    let ref_state_id = ObjectId::from_bytes(cursor.read_array::<32>()?);
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData("trailing bytes in ref pointer".to_string()));
    }
    Ok(RefPointer { ref_name, ref_state_id })
}
