//! Canonical object-tree verification and temp-debris classification.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectId, ObjectType};

use super::{
    BlockSealVerification, ObjectVerification, PublicationTrustVerifier, verify_block_payload,
};
use crate::block_state::{LineageStateMemo, verify_blocks_topological};
use crate::file_codec::decode_envelope_file;
use crate::fsutil::{EntryKind, inspect_entry, list_directory, read_file_required};
use crate::layout::{RepositoryLayout, persisted_object_types};
use crate::object_store::FileObjectStore;
use crate::signature_diagnostics::{
    SignatureEnvelopeIssue, SignatureEnvelopeSource, classify_signature_envelope,
};

pub(super) struct ObjectSummary {
    pub(super) object_count: usize,
    pub(super) block_count: usize,
    pub(super) rollback_block_count: usize,
    pub(super) rollback_patch_count: usize,
    pub(super) temp_paths: Vec<PathBuf>,
    pub(super) signature_issues: Vec<SignatureEnvelopeIssue>,
    pub(super) merge_baseline_divergences: Vec<super::MergeBaselineDivergence>,
    pub(super) block_seals: Vec<BlockSealVerification>,
}

impl ObjectSummary {
    const fn empty() -> Self {
        Self {
            object_count: 0,
            block_count: 0,
            rollback_block_count: 0,
            rollback_patch_count: 0,
            temp_paths: Vec::new(),
            signature_issues: Vec::new(),
            merge_baseline_divergences: Vec::new(),
            block_seals: Vec::new(),
        }
    }

    fn add(&mut self, other: Self) -> Result<()> {
        self.object_count = checked_add(self.object_count, other.object_count, "object")?;
        self.block_count = checked_add(self.block_count, other.block_count, "block")?;
        self.rollback_block_count = checked_add(
            self.rollback_block_count,
            other.rollback_block_count,
            "rollback block",
        )?;
        self.rollback_patch_count = checked_add(
            self.rollback_patch_count,
            other.rollback_patch_count,
            "rollback patch",
        )?;
        self.temp_paths.extend(other.temp_paths);
        self.signature_issues.extend(other.signature_issues);
        self.merge_baseline_divergences
            .extend(other.merge_baseline_divergences);
        self.block_seals.extend(other.block_seals);
        Ok(())
    }
}

pub(super) fn verify_objects(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
) -> Result<ObjectSummary> {
    // DC-92 §4.2: Phase A (below) collects every CurrentV2 Block's already-decoded payload instead
    // of verifying its state inline, in whatever order the generic scan visits objects (ObjectId
    // order — unrelated to lineage). Phase B (`verify_blocks_topological`, after the loop) verifies
    // them in state-dependency order instead, against one shared memo constructed here and evicted
    // from as it goes — see that function's doc for why this bounds memory rather than merely
    // avoiding redundant re-derivation. Never persisted past this call.
    let mut lineage_memo = LineageStateMemo::new();
    let mut pending_v2_blocks: Vec<(ObjectId, BlockPayload)> = Vec::new();
    let mut summary = ObjectSummary::empty();
    for object_type in persisted_object_types() {
        summary.add(verify_object_type(
            layout,
            object_store,
            object_type,
            trust_verifier,
            &mut pending_v2_blocks,
        )?)?;
    }
    verify_blocks_topological(object_store, &pending_v2_blocks, &mut lineage_memo)?;
    Ok(summary)
}

fn verify_object_type(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    object_type: ObjectType,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
    pending_v2_blocks: &mut Vec<(ObjectId, BlockPayload)>,
) -> Result<ObjectSummary> {
    let dir = layout.object_type_dir(object_type);
    let relative_dir = layout.repository_relative(&dir)?;
    match inspect_entry(layout.repository_mutation_root(), &relative_dir)? {
        None => return Ok(ObjectSummary::empty()),
        Some(EntryKind::Directory) => {}
        Some(_) => {
            return Err(PrikkError::Integrity(format!(
                "unexpected non-directory in object type directory: {}",
                dir.display()
            )));
        }
    }
    let mut summary = ObjectSummary::empty();
    let mut entries = list_directory(layout.repository_mutation_root(), &relative_dir)?;
    entries.sort_by(|left, right| {
        left.name
            .as_encoded_bytes()
            .cmp(right.name.as_encoded_bytes())
    });
    for entry in entries {
        let prefix_path = dir.join(&entry.name);
        if entry.kind != EntryKind::Directory {
            return Err(PrikkError::Integrity(format!(
                "unexpected non-directory in object type directory: {}",
                prefix_path.display()
            )));
        }
        summary.add(verify_prefix_dir(
            layout,
            object_store,
            object_type,
            &prefix_path,
            trust_verifier,
            pending_v2_blocks,
        )?)?;
    }
    Ok(summary)
}

fn verify_prefix_dir(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    object_type: ObjectType,
    prefix_path: &Path,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
    pending_v2_blocks: &mut Vec<(ObjectId, BlockPayload)>,
) -> Result<ObjectSummary> {
    let mut summary = ObjectSummary::empty();
    let relative_prefix = layout.repository_relative(prefix_path)?;
    let mut entries = list_directory(layout.repository_mutation_root(), &relative_prefix)?;
    entries.sort_by(|left, right| {
        left.name
            .as_encoded_bytes()
            .cmp(right.name.as_encoded_bytes())
    });
    for entry in entries {
        let path = prefix_path.join(&entry.name);
        if entry.kind != EntryKind::Regular {
            return Err(PrikkError::Integrity(format!(
                "unexpected non-file in object prefix directory: {}",
                path.display()
            )));
        }
        if is_object_temp_path(&path) {
            summary.temp_paths.push(path);
            continue;
        }
        let (object, signature_issues, merge_baseline_divergence) = verify_object_file(
            layout,
            object_store,
            object_type,
            &path,
            trust_verifier,
            pending_v2_blocks,
        )?;
        summary.signature_issues.extend(signature_issues);
        summary
            .merge_baseline_divergences
            .extend(merge_baseline_divergence);
        summary.object_count = checked_add(summary.object_count, 1, "object")?;
        if object.object_type == ObjectType::Block {
            summary.block_count = checked_add(summary.block_count, 1, "block")?;
            if let Some(sealed_by_key_id) = object.sealed_by_key_id.clone() {
                summary.block_seals.push(BlockSealVerification {
                    block_id: object.object_id,
                    sealed_by_key_id,
                });
            }
            if object.rollback_patch_count != 0 {
                summary.rollback_block_count =
                    checked_add(summary.rollback_block_count, 1, "rollback block")?;
                summary.rollback_patch_count = checked_add(
                    summary.rollback_patch_count,
                    object.rollback_patch_count,
                    "rollback patch",
                )?;
            }
        }
    }
    Ok(summary)
}

fn verify_object_file(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    object_type: ObjectType,
    path: &Path,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
    pending_v2_blocks: &mut Vec<(ObjectId, BlockPayload)>,
) -> Result<(
    ObjectVerification,
    Vec<SignatureEnvelopeIssue>,
    Option<super::MergeBaselineDivergence>,
)> {
    let object_id = object_id_from_path(path)?;
    let expected_path = layout.object_path(object_type, object_id);
    if path != expected_path {
        return Err(PrikkError::Integrity(format!(
            "object path {} does not match canonical path {}",
            path.display(),
            expected_path.display()
        )));
    }
    let relative = layout.repository_relative(path)?;
    let envelope = decode_envelope_file(&read_file_required(
        layout.repository_mutation_root(),
        &relative,
    )?)?;
    if envelope.object_type != object_type {
        return Err(PrikkError::Integrity(format!(
            "object file {} is under type {} but envelope type is {}",
            path.display(),
            object_type,
            envelope.object_type
        )));
    }
    crate::format::validate_read_schema(layout.format(), &envelope)?;
    let computed = envelope.object_id();
    if computed != object_id {
        return Err(PrikkError::Integrity(format!(
            "object file {} has id {} but computed id is {}",
            path.display(),
            object_id,
            computed
        )));
    }
    let signature_issues = classify_signature_envelope(
        &envelope,
        SignatureEnvelopeSource::Object {
            object_type,
            object_id,
        },
    )?;
    let sealed_by_key_id = if matches!(object_type, ObjectType::Block | ObjectType::RefState) {
        trust_verifier.verify(&envelope)?
    } else {
        None
    };
    let (rollback_patch_count, merge_baseline_divergence) = if object_type == ObjectType::Block {
        verify_block_payload(
            object_store,
            object_id,
            layout.format(),
            &envelope.canonical_payload,
            pending_v2_blocks,
        )?
    } else {
        (0, None)
    };
    Ok((
        ObjectVerification {
            object_id,
            object_type,
            path: path.to_path_buf(),
            rollback_patch_count,
            sealed_by_key_id,
        },
        signature_issues,
        merge_baseline_divergence,
    ))
}

fn object_id_from_path(path: &Path) -> Result<ObjectId> {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Err(PrikkError::Integrity(format!(
            "object file path is not valid UTF-8: {}",
            path.display()
        )));
    };
    let Some(hex) = file_name.strip_suffix(".pobj") else {
        return Err(PrikkError::Integrity(format!(
            "object file does not use .pobj extension: {}",
            path.display()
        )));
    };
    ObjectId::from_str(hex)
}

fn is_object_temp_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some((object_name, suffix)) = name.split_once(".pobj.tmp.") else {
        return false;
    };
    let Some((pid, random)) = suffix.split_once('.') else {
        return false;
    };
    object_name.len() == 64
        && object_name.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !pid.is_empty()
        && pid.bytes().all(|byte| byte.is_ascii_digit())
        && random.len() == 32
        && random.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn checked_add(left: usize, right: usize, label: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| PrikkError::Integrity(format!("{label} verification count overflow")))
}
