#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Storage crate for PRIKK repositories.
//!
//! PR-003 introduces only the persistent repository layout and file-backed object store. WAL,
//! refs, locking, and patch algebra remain separate implementation increments.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType, Signature, SignatureAlgorithm, SignerRole};

const REPO_DIR: &str = ".prikk";
const FORMAT_VERSION: &str = "1\n";
const ENVELOPE_FILE_MAGIC: &[u8; 8] = b"POBJ0001";

/// Read-only object access boundary.
pub trait ObjectReader {
    /// Read an object by ID.
    fn read_object(&self, id: ObjectId) -> Result<Option<ObjectEnvelope>>;
}

/// Write object boundary.
pub trait ObjectWriter {
    /// Write an object envelope after validation.
    fn write_object(&mut self, envelope: &ObjectEnvelope) -> Result<ObjectId>;
}

/// Repository layout paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryLayout {
    root: PathBuf,
    prikk_dir: PathBuf,
}

impl RepositoryLayout {
    /// Create a layout for a working tree root.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let prikk_dir = root.join(REPO_DIR);
        Self { root, prikk_dir }
    }

    /// Initialize a repository layout on disk.
    pub fn init(root: impl Into<PathBuf>) -> Result<Self> {
        let layout = Self::new(root);
        fs::create_dir_all(&layout.prikk_dir)?;
        for dir in layout.required_directories() {
            fs::create_dir_all(dir)?;
        }
        write_file_atomically(&layout.format_path(), FORMAT_VERSION.as_bytes())?;
        sync_directory_best_effort(&layout.prikk_dir)?;
        Ok(layout)
    }

    /// Open an existing repository layout.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let layout = Self::new(root);
        let mut version = String::new();
        File::open(layout.format_path())?.read_to_string(&mut version)?;
        if version != FORMAT_VERSION {
            return Err(PrikkError::UnsupportedFormatVersion(0));
        }
        Ok(layout)
    }

    /// Return the working tree root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the `.prikk` directory.
    #[must_use]
    pub fn prikk_dir(&self) -> &Path {
        &self.prikk_dir
    }

    /// Return the repository format marker path.
    #[must_use]
    pub fn format_path(&self) -> PathBuf {
        self.prikk_dir.join("FORMAT")
    }

    /// Return the object root directory.
    #[must_use]
    pub fn objects_dir(&self) -> PathBuf {
        self.prikk_dir.join("objects")
    }

    /// Return the active-session root directory.
    #[must_use]
    pub fn active_dir(&self) -> PathBuf {
        self.prikk_dir.join("active")
    }

    /// Return the ref root directory.
    #[must_use]
    pub fn refs_dir(&self) -> PathBuf {
        self.prikk_dir.join("refs")
    }

    /// Return the cache directory.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.prikk_dir.join("cache")
    }

    /// Return the quarantine directory.
    #[must_use]
    pub fn quarantine_dir(&self) -> PathBuf {
        self.prikk_dir.join("quarantine")
    }

    /// Return all required directories for PR-003 layout creation.
    #[must_use]
    pub fn required_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        dirs.push(self.objects_dir());
        dirs.push(self.objects_dir().join("patch"));
        dirs.push(self.objects_dir().join("block"));
        dirs.push(self.objects_dir().join("ref-state"));
        dirs.push(self.objects_dir().join("tag"));
        dirs.push(self.objects_dir().join("attestation"));
        dirs.push(self.objects_dir().join("blob"));
        dirs.push(self.objects_dir().join("ref-update"));
        dirs.push(self.active_dir());
        dirs.push(self.active_dir().join("default"));
        dirs.push(self.refs_dir());
        dirs.push(self.refs_dir().join("by-id"));
        dirs.push(self.refs_dir().join("logs"));
        dirs.push(self.refs_dir().join("locks"));
        dirs.push(self.refs_dir().join("tmp"));
        dirs.push(self.cache_dir());
        dirs.push(self.quarantine_dir());
        dirs
    }

    /// Return the object directory for a type.
    #[must_use]
    pub fn object_type_dir(&self, object_type: ObjectType) -> PathBuf {
        self.objects_dir().join(object_type_directory_name(object_type))
    }

    /// Return the storage path for an object ID and type.
    #[must_use]
    pub fn object_path(&self, object_type: ObjectType, id: ObjectId) -> PathBuf {
        let hex = id.to_hex();
        let prefix = hex_prefix(&hex);
        self.object_type_dir(object_type)
            .join(prefix)
            .join(format!("{hex}.pobj"))
    }
}

/// File-backed object store.
#[derive(Debug, Clone)]
pub struct FileObjectStore {
    layout: RepositoryLayout,
}

impl FileObjectStore {
    /// Create a file object store for a repository layout.
    #[must_use]
    pub fn new(layout: RepositoryLayout) -> Self {
        Self { layout }
    }

    /// Return the repository layout.
    #[must_use]
    pub fn layout(&self) -> &RepositoryLayout {
        &self.layout
    }

    /// Return true if an object path exists.
    #[must_use]
    pub fn contains_object(&self, object_type: ObjectType, id: ObjectId) -> bool {
        self.layout.object_path(object_type, id).is_file()
    }

    /// Read and require a specific object type.
    pub fn read_typed(
        &self,
        id: ObjectId,
        object_type: ObjectType,
    ) -> Result<Option<ObjectEnvelope>> {
        let Some(envelope) = self.read_object(id)? else {
            return Ok(None);
        };
        if envelope.object_type != object_type {
            return Err(PrikkError::ObjectTypeMismatch {
                expected: object_type.to_string(),
                actual: envelope.object_type.to_string(),
            });
        }
        Ok(Some(envelope))
    }
}

impl ObjectReader for FileObjectStore {
    fn read_object(&self, id: ObjectId) -> Result<Option<ObjectEnvelope>> {
        for object_type in all_object_types() {
            let path = self.layout.object_path(object_type, id);
            if path.is_file() {
                let bytes = fs::read(&path)?;
                let envelope = decode_envelope_file(&bytes)?;
                let computed = envelope.object_id();
                if computed != id {
                    return Err(PrikkError::Integrity(format!(
                        "object path {id} contains envelope with computed id {computed}"
                    )));
                }
                if envelope.object_type != object_type {
                    return Err(PrikkError::Integrity(format!(
                        "object path type {object_type} contains envelope type {}",
                        envelope.object_type
                    )));
                }
                return Ok(Some(envelope));
            }
        }
        Ok(None)
    }
}

impl ObjectWriter for FileObjectStore {
    fn write_object(&mut self, envelope: &ObjectEnvelope) -> Result<ObjectId> {
        envelope.validate()?;
        let id = envelope.object_id();
        let path = self.layout.object_path(envelope.object_type, id);
        if path.is_file() {
            return Ok(id);
        }
        let Some(parent) = path.parent() else {
            return Err(PrikkError::Io("object path has no parent directory".to_string()));
        };
        fs::create_dir_all(parent)?;
        let bytes = encode_envelope_file(envelope)?;
        write_file_atomically(&path, &bytes)?;
        sync_directory_best_effort(parent)?;
        Ok(id)
    }
}

/// In-memory test object store for fixtures and early callers.
#[derive(Debug, Default)]
pub struct MemoryObjectStore {
    objects: std::collections::BTreeMap<ObjectId, ObjectEnvelope>,
}

impl MemoryObjectStore {
    /// Create an empty memory store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return number of stored objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Return true when no objects are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Read and require a specific object type.
    pub fn read_typed(
        &self,
        id: ObjectId,
        object_type: ObjectType,
    ) -> Result<Option<ObjectEnvelope>> {
        if let Some(object) = self.objects.get(&id) {
            if object.object_type != object_type {
                return Err(prikk_error::PrikkError::ObjectTypeMismatch {
                    expected: object_type.to_string(),
                    actual: object.object_type.to_string(),
                });
            }
            return Ok(Some(object.clone()));
        }
        Ok(None)
    }
}

impl ObjectReader for MemoryObjectStore {
    fn read_object(&self, id: ObjectId) -> Result<Option<ObjectEnvelope>> {
        Ok(self.objects.get(&id).cloned())
    }
}

impl ObjectWriter for MemoryObjectStore {
    fn write_object(&mut self, envelope: &ObjectEnvelope) -> Result<ObjectId> {
        envelope.validate()?;
        let id = envelope.object_id();
        self.objects.insert(id, envelope.clone());
        Ok(id)
    }
}

fn object_type_directory_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Patch => "patch",
        ObjectType::Block => "block",
        ObjectType::RefState => "ref-state",
        ObjectType::Tag => "tag",
        ObjectType::Attestation => "attestation",
        ObjectType::Blob => "blob",
        ObjectType::RefUpdate => "ref-update",
    }
}

fn all_object_types() -> [ObjectType; 7] {
    [
        ObjectType::Patch,
        ObjectType::Block,
        ObjectType::RefState,
        ObjectType::Tag,
        ObjectType::Attestation,
        ObjectType::Blob,
        ObjectType::RefUpdate,
    ]
}

fn hex_prefix(hex: &str) -> String {
    hex.chars().take(2).collect()
}

fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(PrikkError::Io("atomic write path has no parent directory".to_string()));
    };
    fs::create_dir_all(parent)?;
    let tmp_path = temporary_path(path);
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    sync_directory_best_effort(parent)?;
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(format!(".tmp.{}", std::process::id()));
    path.with_file_name(file_name)
}

fn sync_directory_best_effort(path: &Path) -> Result<()> {
    match File::open(path) {
        Ok(file) => {
            let _ = file.sync_all();
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn encode_envelope_file(envelope: &ObjectEnvelope) -> Result<Vec<u8>> {
    envelope.validate()?;
    let mut out = Vec::new();
    out.extend_from_slice(ENVELOPE_FILE_MAGIC);
    push_u16(&mut out, envelope.object_type.code());
    push_u32(&mut out, envelope.schema_version);
    push_bytes_u64(&mut out, &envelope.canonical_payload)?;
    push_u32(&mut out, len_to_u32(envelope.signatures.len())?);
    for signature in &envelope.signatures {
        push_u16(&mut out, signature.algorithm.code());
        push_u16(&mut out, signature.signer_role.code());
        push_string_u16(&mut out, &signature.key_id)?;
        push_u64(&mut out, signature.created_at);
        push_bytes_u32(&mut out, &signature.signature_bytes)?;
    }
    Ok(out)
}

fn decode_envelope_file(bytes: &[u8]) -> Result<ObjectEnvelope> {
    let mut cursor = ByteCursor::new(bytes);
    let magic = cursor.read_array::<8>()?;
    if &magic != ENVELOPE_FILE_MAGIC {
        return Err(PrikkError::MalformedData("invalid object file magic".to_string()));
    }
    let object_type = ObjectType::from_code(cursor.read_u16()?)?;
    let schema_version = cursor.read_u32()?;
    let canonical_payload = cursor.read_bytes_u64()?;
    let signature_count = cursor.read_u32()?;
    let mut signatures = Vec::new();
    for _ in 0..signature_count {
        let algorithm = SignatureAlgorithm::from_code(cursor.read_u16()?)?;
        let signer_role = SignerRole::from_code(cursor.read_u16()?)?;
        let key_id = cursor.read_string_u16()?;
        let created_at = cursor.read_u64()?;
        let signature_bytes = cursor.read_bytes_u32()?;
        signatures.push(Signature {
            algorithm,
            key_id,
            signature_bytes,
            created_at,
            signer_role,
        });
    }
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData("trailing bytes in object file".to_string()));
    }
    let envelope = ObjectEnvelope {
        object_type,
        schema_version,
        canonical_payload,
        signatures,
    };
    envelope.validate()?;
    Ok(envelope)
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_string_u16(out: &mut Vec<u8>, value: &str) -> Result<()> {
    let len = len_to_u16(value.len())?;
    push_u16(out, len);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn push_bytes_u32(out: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let len = len_to_u32(value.len())?;
    push_u32(out, len);
    out.extend_from_slice(value);
    Ok(())
}

fn push_bytes_u64(out: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let len = len_to_u64(value.len())?;
    push_u64(out, len);
    out.extend_from_slice(value);
    Ok(())
}

fn len_to_u16(len: usize) -> Result<u16> {
    u16::try_from(len).map_err(|_| PrikkError::MalformedData("length exceeds u16".to_string()))
}

fn len_to_u32(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| PrikkError::MalformedData("length exceeds u32".to_string()))
}

fn len_to_u64(len: usize) -> Result<u64> {
    u64::try_from(len).map_err(|_| PrikkError::MalformedData("length exceeds u64".to_string()))
}

struct ByteCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn is_finished(&self) -> bool {
        self.pos == self.bytes.len()
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let slice = self.read_exact(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.read_array::<2>()?))
    }

    fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.read_array::<8>()?))
    }

    fn read_string_u16(&mut self) -> Result<String> {
        let len = usize::from(self.read_u16()?);
        let bytes = self.read_exact(len)?.to_vec();
        String::from_utf8(bytes)
            .map_err(|err| PrikkError::MalformedData(format!("invalid utf-8 string: {err}")))
    }

    fn read_bytes_u32(&mut self) -> Result<Vec<u8>> {
        let len = usize::try_from(self.read_u32()?)
            .map_err(|_| PrikkError::MalformedData("u32 length does not fit usize".to_string()))?;
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_bytes_u64(&mut self) -> Result<Vec<u8>> {
        let len = usize::try_from(self.read_u64()?)
            .map_err(|_| PrikkError::MalformedData("u64 length does not fit usize".to_string()))?;
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| PrikkError::MalformedData("record length overflow".to_string()))?;
        let Some(slice) = self.bytes.get(self.pos..end) else {
            return Err(PrikkError::MalformedData("unexpected end of record".to_string()));
        };
        self.pos = end;
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::{FileObjectStore, MemoryObjectStore, ObjectReader, ObjectWriter, RepositoryLayout};
    use prikk_object::{ObjectEnvelope, ObjectType, Signature, SignatureAlgorithm, SignerRole};

    #[test]
    fn memory_store_roundtrips_object() {
        let mut store = MemoryObjectStore::new();
        let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        let id = store.write_object(&envelope);
        assert!(id.is_ok());
        if let Ok(id) = id {
            let read = store.read_object(id);
            assert_eq!(read, Ok(Some(envelope)));
        }
    }

    #[test]
    fn repository_init_creates_required_directories() {
        let root = unique_temp_dir("layout");
        let layout = RepositoryLayout::init(root.clone());
        assert!(layout.is_ok());
        if let Ok(layout) = layout {
            for dir in layout.required_directories() {
                assert!(dir.is_dir(), "missing directory: {}", dir.display());
            }
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn file_store_roundtrips_signed_object() {
        let root = unique_temp_dir("filestore");
        let layout = RepositoryLayout::init(root.clone());
        assert!(layout.is_ok());
        if let Ok(layout) = layout {
            let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
            let signature = Signature {
                algorithm: SignatureAlgorithm::Ed25519,
                key_id: "author-key".to_string(),
                signature_bytes: vec![1, 2, 3, 4],
                created_at: 7,
                signer_role: SignerRole::Author,
            };
            assert!(envelope.add_signature(signature).is_ok());
            let mut store = FileObjectStore::new(layout);
            let id = store.write_object(&envelope);
            assert!(id.is_ok());
            if let Ok(id) = id {
                let read = store.read_object(id);
                assert_eq!(read, Ok(Some(envelope)));
            }
        }
        let _ = std::fs::remove_dir_all(root);
    }

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "prikk-pr003-{name}-{}-{}",
            std::process::id(),
            monotonic_suffix()
        ));
        path
    }

    fn monotonic_suffix() -> u128 {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_nanos(),
            Err(_) => 0,
        }
    }
}
