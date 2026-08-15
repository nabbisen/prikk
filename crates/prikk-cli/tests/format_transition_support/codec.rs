use prikk_hash::sha256;
use prikk_object::{ObjectEnvelope, ObjectId};
use prikk_store::RepositoryLayout;

use super::TestResult;

const WAL_MAGIC: &[u8; 8] = b"PWALR001";
const REF_LOG_MAGIC: &[u8; 8] = b"PREFLOG1";

pub(super) fn write_object(
    layout: &RepositoryLayout,
    envelope: &ObjectEnvelope,
) -> TestResult<ObjectId> {
    let object_id = envelope.object_id();
    let path = layout.object_path(envelope.object_type, object_id);
    let parent = path
        .parent()
        .ok_or("legacy fixture object path has no parent")?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, encode_envelope(envelope)?)?;
    Ok(object_id)
}

pub(super) fn write_ref_log(
    layout: &RepositoryLayout,
    ref_name: &str,
    envelope: &ObjectEnvelope,
) -> TestResult {
    let body = encode_envelope(envelope)?;
    let body_len = u64::try_from(body.len())?;
    let mut checksum_input = Vec::new();
    checksum_input.extend_from_slice(REF_LOG_MAGIC);
    checksum_input.extend_from_slice(&1_u16.to_be_bytes());
    checksum_input.extend_from_slice(&body_len.to_be_bytes());
    checksum_input.extend_from_slice(&body);

    let mut record = Vec::new();
    record.extend_from_slice(REF_LOG_MAGIC);
    record.extend_from_slice(&1_u16.to_be_bytes());
    record.extend_from_slice(&body_len.to_be_bytes());
    record.extend_from_slice(&sha256(&checksum_input));
    record.extend_from_slice(&body);
    let path = layout.ref_log_path(ref_name);
    let parent = path
        .parent()
        .ok_or("legacy fixture ref log has no parent")?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, record)?;
    Ok(())
}

pub(super) fn write_ref_pointer(
    layout: &RepositoryLayout,
    ref_name: &str,
    state_id: ObjectId,
) -> TestResult {
    let name = ref_name.as_bytes();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PREFPTR1");
    bytes.extend_from_slice(&u64::try_from(name.len())?.to_be_bytes());
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(state_id.as_bytes());
    let path = layout.ref_pointer_path(ref_name);
    let parent = path
        .parent()
        .ok_or("legacy fixture ref pointer has no parent")?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

pub(super) fn write_wal(layout: &RepositoryLayout, envelope: &ObjectEnvelope) -> TestResult {
    let body = encode_envelope(envelope)?;
    let body_len = u64::try_from(body.len())?;
    let mut checksum_input = Vec::new();
    checksum_input.extend_from_slice(WAL_MAGIC);
    checksum_input.extend_from_slice(&1_u16.to_be_bytes());
    checksum_input.extend_from_slice(&1_u64.to_be_bytes());
    checksum_input.extend_from_slice(&body_len.to_be_bytes());
    checksum_input.extend_from_slice(&body);

    let mut record = Vec::new();
    record.extend_from_slice(WAL_MAGIC);
    record.extend_from_slice(&1_u16.to_be_bytes());
    record.extend_from_slice(&1_u64.to_be_bytes());
    record.extend_from_slice(&body_len.to_be_bytes());
    record.extend_from_slice(&sha256(&checksum_input));
    record.extend_from_slice(&body);
    std::fs::write(layout.default_queue_wal_path(), record)?;
    Ok(())
}

fn encode_envelope(envelope: &ObjectEnvelope) -> TestResult<Vec<u8>> {
    envelope.validate()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"POBJ0001");
    bytes.extend_from_slice(&envelope.object_type.code().to_be_bytes());
    bytes.extend_from_slice(&envelope.schema_version.to_be_bytes());
    push_bytes_u64(&mut bytes, &envelope.canonical_payload)?;
    bytes.extend_from_slice(&u32::try_from(envelope.signatures.len())?.to_be_bytes());
    for signature in &envelope.signatures {
        bytes.extend_from_slice(&signature.algorithm.code().to_be_bytes());
        bytes.extend_from_slice(&signature.signer_role.code().to_be_bytes());
        push_bytes_u16(&mut bytes, signature.key_id.as_bytes())?;
        bytes.extend_from_slice(&signature.created_at.to_be_bytes());
        push_bytes_u32(&mut bytes, &signature.signature_bytes)?;
    }
    Ok(bytes)
}

fn push_bytes_u16(output: &mut Vec<u8>, value: &[u8]) -> TestResult {
    output.extend_from_slice(&u16::try_from(value.len())?.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

fn push_bytes_u32(output: &mut Vec<u8>, value: &[u8]) -> TestResult {
    output.extend_from_slice(&u32::try_from(value.len())?.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

fn push_bytes_u64(output: &mut Vec<u8>, value: &[u8]) -> TestResult {
    output.extend_from_slice(&u64::try_from(value.len())?.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

pub(super) fn remove_pointer(layout: &RepositoryLayout, ref_name: &str) -> TestResult {
    std::fs::remove_file(layout.ref_pointer_path(ref_name))?;
    Ok(())
}
