//! Root-relative worktree enumeration for node authoring.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use prikk_error::PrikkError;

use super::{AuthorError, EXECUTABLE_FILE_MODE, REGULAR_FILE_MODE, RepoPath, RepositoryLayout};
use crate::fsutil::{EntryKind, list_directory, read_file_state_if_exists};

pub(super) struct WorktreeFile {
    pub(super) bytes: Vec<u8>,
    pub(super) mode: u32,
}

pub(super) fn enumerate_worktree_files(
    layout: &RepositoryLayout,
) -> std::result::Result<BTreeMap<String, WorktreeFile>, AuthorError> {
    let mut out = BTreeMap::new();
    walk_dir(layout, Path::new(""), &mut out)?;
    Ok(out)
}

fn walk_dir(
    layout: &RepositoryLayout,
    dir: &Path,
    out: &mut BTreeMap<String, WorktreeFile>,
) -> std::result::Result<(), AuthorError> {
    let entries =
        list_directory(layout.worktree_mutation_root(), dir).map_err(AuthorError::Store)?;
    for entry in entries {
        let file_name = entry.name;
        if file_name == ".prikk" {
            continue;
        }
        let path = join_relative(dir, &file_name);
        match entry.kind {
            EntryKind::Symlink => {
                return Err(AuthorError::UnsupportedSymlinkAuthoring(format!(
                    "{}: worktree symlink authoring is out of scope",
                    path.to_string_lossy()
                )));
            }
            EntryKind::Directory => {
                walk_dir(layout, &path, out)?;
                continue;
            }
            EntryKind::Regular => {}
            EntryKind::Other => {
                return Err(AuthorError::Store(PrikkError::InvalidName(format!(
                    "{}: worktree entry is not a regular file",
                    path.to_string_lossy()
                ))));
            }
        }
        insert_regular_file(layout, &path, out)?;
    }
    Ok(())
}

fn insert_regular_file(
    layout: &RepositoryLayout,
    path: &Path,
    out: &mut BTreeMap<String, WorktreeFile>,
) -> std::result::Result<(), AuthorError> {
    let rel = path.to_str().ok_or_else(|| {
        AuthorError::Store(PrikkError::InvalidName(format!(
            "worktree path is not valid UTF-8: {}",
            path.to_string_lossy()
        )))
    })?;
    let repo_path = RepoPath::parse(rel).map_err(AuthorError::Store)?;
    let file = read_file_state_if_exists(layout.worktree_mutation_root(), path)
        .map_err(AuthorError::Store)?
        .ok_or_else(|| {
            AuthorError::Store(PrikkError::Io(format!(
                "worktree entry disappeared: {}",
                path.display()
            )))
        })?;
    out.insert(
        repo_path.as_str().to_string(),
        WorktreeFile {
            bytes: file.bytes,
            mode: normalize_file_mode(file.mode),
        },
    );
    Ok(())
}

fn normalize_file_mode(mode: u32) -> u32 {
    if mode & 0o111 != 0 {
        EXECUTABLE_FILE_MODE
    } else {
        REGULAR_FILE_MODE
    }
}

fn join_relative(parent: &Path, name: &std::ffi::OsStr) -> PathBuf {
    if parent.as_os_str().is_empty() {
        PathBuf::from(name)
    } else {
        parent.join(name)
    }
}
