//! Canonical object-tree verification and temp-debris classification.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectId, ObjectType};

use super::{ObjectVerification, PublicationTrustVerifier, verify_block_payload};
use crate::file_codec::decode_envelope_file;
use crate::layout::{RepositoryLayout, persisted_object_types};
use crate::object_store::FileObjectStore;

pub(super) struct ObjectSummary {
    pub(super) object_count: usize,
    pub(super) block_count: usize,
    pub(super) rollback_block_count: usize,
    pub(super) rollback_patch_count: usize,
    pub(super) temp_paths: Vec<PathBuf>,
}

impl ObjectSummary {
    const fn empty() -> Self {
        Self {
            object_count: 0,
            block_count: 0,
            rollback_block_count: 0,
            rollback_patch_count: 0,
            temp_paths: Vec::new(),
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
        Ok(())
    }
}

pub(super) fn verify_objects(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
) -> Result<ObjectSummary> {
    let mut summary = ObjectSummary::empty();
    for object_type in persisted_object_types() {
        summary.add(verify_object_type(
            layout,
            object_store,
            object_type,
            trust_verifier,
        )?)?;
    }
    Ok(summary)
}

fn verify_object_type(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    object_type: ObjectType,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
) -> Result<ObjectSummary> {
    let dir = layout.object_type_dir(object_type);
    if !dir.exists() {
        return Ok(ObjectSummary::empty());
    }
    let mut summary = ObjectSummary::empty();
    for entry in fs::read_dir(&dir)? {
        let prefix_path = entry?.path();
        if !prefix_path.is_dir() {
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
) -> Result<ObjectSummary> {
    let mut summary = ObjectSummary::empty();
    for entry in fs::read_dir(prefix_path)? {
        let path = entry?.path();
        if path.is_dir() {
            return Err(PrikkError::Integrity(format!(
                "unexpected directory in object prefix directory: {}",
                path.display()
            )));
        }
        if is_object_temp_path(&path) {
            summary.temp_paths.push(path);
            continue;
        }
        let object = verify_object_file(layout, object_store, object_type, &path, trust_verifier)?;
        summary.object_count = checked_add(summary.object_count, 1, "object")?;
        if object.object_type == ObjectType::Block {
            summary.block_count = checked_add(summary.block_count, 1, "block")?;
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
) -> Result<ObjectVerification> {
    let object_id = object_id_from_path(path)?;
    let expected_path = layout.object_path(object_type, object_id);
    if path != expected_path {
        return Err(PrikkError::Integrity(format!(
            "object path {} does not match canonical path {}",
            path.display(),
            expected_path.display()
        )));
    }
    let envelope = decode_envelope_file(&fs::read(path)?)?;
    if envelope.object_type != object_type {
        return Err(PrikkError::Integrity(format!(
            "object file {} is under type {} but envelope type is {}",
            path.display(),
            object_type,
            envelope.object_type
        )));
    }
    let computed = envelope.object_id();
    if computed != object_id {
        return Err(PrikkError::Integrity(format!(
            "object file {} has id {} but computed id is {}",
            path.display(),
            object_id,
            computed
        )));
    }
    if matches!(object_type, ObjectType::Block | ObjectType::RefState) {
        trust_verifier.verify(&envelope)?;
    }
    let rollback_patch_count = if object_type == ObjectType::Block {
        verify_block_payload(object_store, object_id, &envelope.canonical_payload)?
    } else {
        0
    };
    Ok(ObjectVerification {
        object_id,
        object_type,
        path: path.to_path_buf(),
        rollback_patch_count,
    })
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
