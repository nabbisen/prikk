use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::Result;

static CANDIDATE_ID: AtomicU64 = AtomicU64::new(0);

pub(super) fn create(root: &Path) -> Result<Candidate> {
    let scratch = root.join(".git-exclude/tmp");
    fs::create_dir_all(&scratch)?;
    let path = scratch.join(format!(
        "prikk-oracle-{}-{}",
        std::process::id(),
        CANDIDATE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    if path.exists() {
        fs::remove_dir_all(&path)?;
    }
    fs::create_dir(&path)?;
    let candidate = Candidate { path };
    copy_tree(&root.join("release"), &candidate.path().join("release"))?;
    fs::copy(
        root.join("release-signers.toml"),
        candidate.path().join("release-signers.toml"),
    )?;
    Ok(candidate)
}

pub(super) struct Candidate {
    path: PathBuf,
}

impl Candidate {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Candidate {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}
