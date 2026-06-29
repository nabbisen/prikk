//! Snapshot-manifest validation for future checkout materialization.
//!
//! Snapshot bytes are stored inside Blob objects. PR-017 validates snapshot content and feeds an
//! explicit snapshot materializer.

use prikk_error::{PrikkError, Result};

use crate::path::{RepoPath, validate_no_path_collisions};

const SNAPSHOT_MAGIC: &[u8] = b"PRIKK-SNAPSHOT-MANIFEST-v1\n";

/// A single file entry in a snapshot manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotEntry {
    /// Validated repository-relative path.
    pub path: RepoPath,
    /// File content bytes.
    pub bytes: Vec<u8>,
}

/// Decoded snapshot manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotManifest {
    /// File entries, sorted by path.
    pub files: Vec<SnapshotEntry>,
}

impl SnapshotManifest {
    /// Decode a snapshot manifest from Blob payload bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let Some(mut rest) = bytes.get(SNAPSHOT_MAGIC.len()..) else {
            return Err(PrikkError::MalformedData(
                "snapshot manifest is shorter than magic".to_string(),
            ));
        };
        if !bytes.starts_with(SNAPSHOT_MAGIC) {
            return Err(PrikkError::MalformedData(
                "snapshot manifest magic mismatch".to_string(),
            ));
        }
        let mut files = Vec::new();
        while !rest.is_empty() {
            let (path_len, after_path_len) = read_u32(rest)?;
            rest = after_path_len;
            let path_len = path_len as usize;
            if path_len == 0 {
                return Err(PrikkError::MalformedData(
                    "snapshot path must not be empty".to_string(),
                ));
            }
            let (path_bytes, after_path) = read_exact(rest, path_len)?;
            rest = after_path;
            let path_text = std::str::from_utf8(path_bytes)
                .map_err(|_| PrikkError::MalformedData("snapshot path is not UTF-8".to_string()))?;
            let path = RepoPath::parse(path_text)?;
            let (content_len, after_content_len) = read_u64(rest)?;
            rest = after_content_len;
            let content_len = usize::try_from(content_len).map_err(|_| {
                PrikkError::MalformedData("snapshot content length does not fit usize".to_string())
            })?;
            let (content, after_content) = read_exact(rest, content_len)?;
            rest = after_content;
            files.push(SnapshotEntry {
                path,
                bytes: content.to_vec(),
            });
        }
        let manifest = Self { files };
        manifest.validate_order_and_collisions()?;
        Ok(manifest)
    }

    /// Encode a snapshot manifest. This is used by tests and fixture generation.
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate_order_and_collisions()?;
        let mut out = Vec::new();
        out.extend_from_slice(SNAPSHOT_MAGIC);
        for file in &self.files {
            let path = file.path.as_str().as_bytes();
            let path_len = u32::try_from(path.len()).map_err(|_| {
                PrikkError::MalformedData("snapshot path length exceeds u32".to_string())
            })?;
            out.extend_from_slice(&path_len.to_be_bytes());
            out.extend_from_slice(path);
            out.extend_from_slice(&(file.bytes.len() as u64).to_be_bytes());
            out.extend_from_slice(&file.bytes);
        }
        Ok(out)
    }

    /// Return the total number of content bytes across entries.
    #[must_use]
    pub fn total_content_bytes(&self) -> u64 {
        self.files
            .iter()
            .map(|entry| entry.bytes.len() as u64)
            .sum()
    }

    fn validate_order_and_collisions(&self) -> Result<()> {
        let paths: Vec<RepoPath> = self.files.iter().map(|entry| entry.path.clone()).collect();
        validate_no_path_collisions(&paths)?;
        if !paths.windows(2).all(|pair| {
            let mut items = pair.iter();
            match (items.next(), items.next()) {
                (Some(left), Some(right)) => left < right,
                _ => true,
            }
        }) {
            return Err(PrikkError::MalformedData(
                "snapshot paths must be sorted by repository path".to_string(),
            ));
        }
        Ok(())
    }
}

fn read_u32(bytes: &[u8]) -> Result<(u32, &[u8])> {
    let (raw, rest) = read_exact(bytes, 4)?;
    let mut out = [0_u8; 4];
    out.copy_from_slice(raw);
    Ok((u32::from_be_bytes(out), rest))
}

fn read_u64(bytes: &[u8]) -> Result<(u64, &[u8])> {
    let (raw, rest) = read_exact(bytes, 8)?;
    let mut out = [0_u8; 8];
    out.copy_from_slice(raw);
    Ok((u64::from_be_bytes(out), rest))
}

fn read_exact(bytes: &[u8], len: usize) -> Result<(&[u8], &[u8])> {
    let Some(value) = bytes.get(..len) else {
        return Err(PrikkError::MalformedData(
            "unexpected end of snapshot manifest".to_string(),
        ));
    };
    let Some(rest) = bytes.get(len..) else {
        return Err(PrikkError::MalformedData(
            "snapshot manifest range overflow".to_string(),
        ));
    };
    Ok((value, rest))
}

#[cfg(test)]
mod tests;
