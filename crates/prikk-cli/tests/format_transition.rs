//! Release-facing format-1/format-2 compatibility matrix.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod format_transition_support;

use format_transition_support::{
    ActiveFixture, MAINTAINER_KEY_ID, MAINTAINER_SEED_HEX, StrictFailure,
    build_format2_strict_wal_fixture, build_legacy_fixture,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn prikk(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prikk"));
    command.current_dir(root);
    command
}

fn run(root: &Path, args: &[&str]) -> TestResult<Output> {
    Ok(prikk(root).args(args).output()?)
}

/// DC-83: the nanosecond timestamp alone was the defect — two threads of this same test binary can
/// observe the identical value (confirmed empirically: a 64-thread synchronized-barrier stress test
/// against bare `SystemTime::now()` nanoseconds produced real collisions, not a theoretical risk).
/// Adding `process::id()` alone would not have closed it: every thread in this one binary shares one
/// PID, so it cannot distinguish two racing threads — only a per-process atomic counter can. Combines
/// the process id (matching `prikk-store::test_support::unique_temp_dir`'s shape, so a collision
/// against a *different* binary's temp directory is still avoided) with a `fetch_add` counter that is
/// unique for the life of this process regardless of clock resolution or thread scheduling.
fn unique_root() -> TestResult<PathBuf> {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "prikk-format-transition-{}-{nonce}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

fn assert_legacy_warning(output: &Output) {
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("format-1 repository opened"),
        "missing legacy warning: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_owned(root: &Path, args: &[String]) -> TestResult<Output> {
    Ok(prikk(root)
        .env("PRIKK_AUTHOR_KEY_ID", "legacy-author")
        .env(
            "PRIKK_AUTHOR_SEED",
            "3636363636363636363636363636363636363636363636363636363636363636",
        )
        .env("PRIKK_MAINTAINER_KEY_ID", MAINTAINER_KEY_ID)
        .env("PRIKK_MAINTAINER_SEED", MAINTAINER_SEED_HEX)
        .args(args)
        .output()?)
}

fn snapshot_tree(root: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    fn walk(root: &Path, current: &Path, snapshot: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
        let mut entries = std::fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root)?.to_path_buf();
            let metadata = std::fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                snapshot.insert(relative.clone(), b"directory".to_vec());
                walk(root, &path, snapshot)?;
            } else if metadata.is_file() {
                snapshot.insert(relative, std::fs::read(path)?);
            } else {
                snapshot.insert(
                    relative,
                    std::fs::read_link(path)?
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec(),
                );
            }
        }
        Ok(())
    }

    let mut snapshot = BTreeMap::new();
    walk(root, root, &mut snapshot)?;
    Ok(snapshot)
}

#[test]
fn format1_empty_repository_is_bounded_read_only() -> TestResult {
    let root = unique_root()?;
    let init = run(&root, &["init"])?;
    assert!(init.status.success());
    assert_eq!(std::fs::read(root.join(".prikk/FORMAT"))?, b"2\n");
    std::fs::write(root.join(".prikk/FORMAT"), b"1\n")?;

    let status = run(&root, &["status"])?;
    assert!(status.status.success());
    assert_legacy_warning(&status);

    let verify = run(&root, &["verify"])?;
    assert!(!verify.status.success());
    assert_legacy_warning(&verify);
    assert!(String::from_utf8_lossy(&verify.stderr).contains("not verifiable"));

    let doctor = run(&root, &["doctor"])?;
    assert!(doctor.status.success());
    assert_legacy_warning(&doctor);
    assert!(String::from_utf8_lossy(&doctor.stdout).contains("PRIKK-DOCTOR-LEGACY-FORMAT"));

    let commit = run(&root, &["commit", "-m", "must refuse"])?;
    assert!(!commit.status.success());
    assert!(String::from_utf8_lossy(&commit.stderr).contains("unsupported format version: 1"));

    let trust = run(
        &root,
        &[
            "trust",
            "maintainer",
            "add",
            "--key-id",
            "legacy-refused",
            "--public-key",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ],
    )?;
    assert!(!trust.status.success());
    assert!(String::from_utf8_lossy(&trust.stderr).contains("unsupported format version: 1"));

    let reinit = run(&root, &["init"])?;
    assert!(!reinit.status.success());
    assert_eq!(std::fs::read(root.join(".prikk/FORMAT"))?, b"1\n");
    assert!(!root.join(".prikk/active/default/queue.wal").exists());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[path = "format_transition/matrix.rs"]
mod matrix;
