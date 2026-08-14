//! Canonical object-tree verification and temp-debris classification.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectId, ObjectType};

use super::{
    BlockSealVerification, ObjectVerification, PublicationTrustVerifier, verify_block_payload,
};
use crate::block_state::{BlockStateOutcome, LineageStateMemo, verify_blocks_topological};
use crate::file_codec::decode_envelope_file;
use crate::fsutil::{EntryKind, inspect_entry, list_directory, read_file_required};
use crate::layout::{RepositoryLayout, persisted_object_types};
use crate::object_store::FileObjectStore;
use crate::signature_diagnostics::{
    SignatureEnvelopeIssue, SignatureEnvelopeSource, classify_signature_envelope,
};

/// Outcome of attempting to verify one persisted object file (DC-95 Stage 2 Level 2, Phase A). No
/// `NotEvaluated` variant: Phase A's per-object checks (decode, id, schema, signature, trust,
/// reference existence) have no real dependency on any *other* object's own outcome (Step 0 §1.1) --
/// every object is independently attempted, so the only two possible resolutions are `Evaluated` or
/// `Failed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectItemStatus {
    /// The object's own checks all passed; carries the same data `verify_object_file` always
    /// produced on success.
    Evaluated(ObjectVerification),
    /// Some check for this specific object failed. Its signature-envelope findings and (for a
    /// `Block`) merge-baseline divergence and `pending_v2_blocks` contribution are *not* recorded --
    /// this object's own verification did not run to completion, so nothing derived partway through
    /// it is reported (the same rule Level 1 applied at stage granularity, one level in).
    Failed {
        /// The error the check raised.
        message: String,
    },
}

/// One object file's resolved outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectItemOutcome {
    /// The object-type directory this file was scanned under.
    pub object_type: ObjectType,
    /// The file's path.
    pub path: PathBuf,
    /// How this object's own verification resolved.
    pub status: ObjectItemStatus,
}

pub(super) struct ObjectSummary {
    /// Phase A: one outcome per object file scanned, in scan order.
    pub(super) item_outcomes: Vec<ObjectItemOutcome>,
    /// Phase B: one outcome per `CurrentV3` Block whose Phase A check succeeded, in the
    /// state-dependency order `verify_blocks_topological` resolved them (DC-92 §4.2) -- not scan
    /// order.
    pub(super) topological_outcomes: Vec<BlockStateOutcome>,
    pub(super) temp_paths: Vec<PathBuf>,
    pub(super) signature_issues: Vec<SignatureEnvelopeIssue>,
    pub(super) merge_baseline_divergences: Vec<super::MergeBaselineDivergence>,
    pub(super) block_seals: Vec<BlockSealVerification>,
}

impl ObjectSummary {
    fn empty() -> Self {
        Self {
            item_outcomes: Vec::new(),
            topological_outcomes: Vec::new(),
            temp_paths: Vec::new(),
            signature_issues: Vec::new(),
            merge_baseline_divergences: Vec::new(),
            block_seals: Vec::new(),
        }
    }

    fn add(&mut self, other: Self) {
        self.item_outcomes.extend(other.item_outcomes);
        self.topological_outcomes.extend(other.topological_outcomes);
        self.temp_paths.extend(other.temp_paths);
        self.signature_issues.extend(other.signature_issues);
        self.merge_baseline_divergences
            .extend(other.merge_baseline_divergences);
        self.block_seals.extend(other.block_seals);
    }
}

pub(super) fn verify_objects(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
) -> Result<ObjectSummary> {
    // DC-92 §4.2: Phase A (below) collects every CurrentV3 Block's already-decoded payload instead
    // of verifying its state inline, in whatever order the generic scan visits objects (ObjectId
    // order — unrelated to lineage). Phase B (`verify_blocks_topological`, after the loop) verifies
    // them in state-dependency order instead, against one shared memo constructed here and evicted
    // from as it goes — see that function's doc for why this bounds memory rather than merely
    // avoiding redundant re-derivation. Never persisted past this call.
    //
    // DC-95 Stage 2 Level 2: Phase A and Phase B are independently item-contained (Step 0 §1). A
    // single object's own Phase A failure does not prevent scanning every other object, structural
    // directory-shape errors excepted (they still abort this whole call, unchanged from Level 1 --
    // Step 0 §1.1's structural/semantic split). Phase B's own item containment lives in
    // `verify_blocks_topological` itself.
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
        )?);
    }
    let topological =
        verify_blocks_topological(object_store, &pending_v2_blocks, &mut lineage_memo)?;
    summary.topological_outcomes = topological.outcomes;
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
        )?);
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
        // DC-95 Stage 2 Level 2: this object's own failure is caught here, at the item boundary,
        // rather than propagated -- mirroring Level 1's stage-boundary catch, one level in. Nothing
        // this object's own call computed partway through (signature issues, merge-baseline
        // divergence, its `pending_v2_blocks` contribution) survives a `Failed` outcome; every
        // *other* object in this and every other prefix directory is still attempted.
        match verify_object_file(
            layout,
            object_store,
            object_type,
            &path,
            trust_verifier,
            pending_v2_blocks,
        ) {
            Ok((object, signature_issues, merge_baseline_divergence)) => {
                summary.signature_issues.extend(signature_issues);
                summary
                    .merge_baseline_divergences
                    .extend(merge_baseline_divergence);
                if object.object_type == ObjectType::Block {
                    if let Some(sealed_by_key_id) = object.sealed_by_key_id.clone() {
                        summary.block_seals.push(BlockSealVerification {
                            block_id: object.object_id,
                            sealed_by_key_id,
                        });
                    }
                }
                summary.item_outcomes.push(ObjectItemOutcome {
                    object_type,
                    path,
                    status: ObjectItemStatus::Evaluated(object),
                });
            }
            Err(err) => {
                summary.item_outcomes.push(ObjectItemOutcome {
                    object_type,
                    path,
                    status: ObjectItemStatus::Failed {
                        message: err.to_string(),
                    },
                });
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
