//! Release-facing retired-format rejection proof: RFC 103 (format 1) and RFC 102 Stage 3 (format 2,
//! design-v1.md §12.1) each retire a repository format by refusing it at open, not reading it in a
//! bounded legacy mode -- there is no dual-path behavior left to exercise across a command matrix,
//! for either format. `build_legacy_fixture` remains load-bearing here: design-v1.md §5 acceptance
//! criterion 2 requires the rejection proven against a real fixture, not a hand-built one, for both
//! retired formats.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod format_transition_support;

use format_transition_support::{
    ActiveFixture, MAINTAINER_KEY_ID, MAINTAINER_SEED_HEX, StrictFailure,
    build_current_format_strict_wal_fixture, build_legacy_fixture,
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

fn assert_rejection_contract(args: &[&str], output: &Output, detected_format: &str) {
    assert!(
        !output.status.success(),
        "{args:?} unexpectedly succeeded: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for expected in [
        detected_format,
        "requires format 3",
        "removed after 0.19.0",
        "bundle export",
        "bundle import",
    ] {
        assert!(
            stderr.contains(expected),
            "{args:?}: rejection message missing {expected:?}: {stderr}"
        );
    }
}

/// RFC 103 §4/design-v1.md §2 (format 1) and RFC 102 Stage 3, design-v1.md §12.1 (format 2, the same
/// proof re-run one format later): a retired-format repository is rejected at
/// `RepositoryLayout::open`, with a message naming the detected format, the required format, the
/// last supporting version, and the bundle export/import remedy — for every command, not a
/// command-specific subset, since rejection now happens before any command-specific logic runs.
/// Proven against real fixtures (`build_legacy_fixture`, differing only in what kind of active-session
/// state they carry and which format byte was flipped), not hand-built ones.
#[test]
fn retired_format_repository_is_rejected_at_open_for_every_command() -> TestResult {
    for (target_format, detected_format) in [
        (b"1\n".as_slice(), "this repository uses format 1"),
        (b"2\n".as_slice(), "this repository uses format 2"),
    ] {
        for active in [
            ActiveFixture::RollbackDraft,
            ActiveFixture::InterruptedPublication,
        ] {
            let root = unique_root()?;
            build_legacy_fixture(&root, active, target_format)?;
            let before = snapshot_tree(&root)?;

            for args in [
                vec!["status"],
                vec!["log"],
                vec!["worktree-status"],
                vec!["verify"],
                vec!["doctor"],
                vec!["checkout", "--plan-only"],
                vec!["rollback-preview"],
                vec!["commit", "-m", "must refuse"],
                vec!["seal", "--allow-no-audit"],
                vec![
                    "trust",
                    "maintainer",
                    "add",
                    "--key-id",
                    "legacy-refused",
                    "--public-key",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                ],
            ] {
                // `run_owned`, not `run`: `seal` in particular constructs its signer from these env
                // vars before it ever opens the repository, so without them it fails at signer
                // construction and never reaches (or proves) the rejection this test is checking.
                let owned_args = args.iter().map(ToString::to_string).collect::<Vec<_>>();
                let output = run_owned(&root, &owned_args)?;
                assert_rejection_contract(&args, &output, detected_format);
                assert_eq!(
                    snapshot_tree(&root)?,
                    before,
                    "{args:?} must not mutate a rejected repository"
                );
            }

            let _ = std::fs::remove_dir_all(root);
        }
    }
    Ok(())
}

/// `RepositoryLayout::init` refuses to initialize over an existing non-format-3 repository through
/// its own, separate check (`layout.rs::init`, not `read_repository_format`) — out of RFC 103's scope
/// since it never opens the repository for use, but still a real safety property worth keeping under
/// regression coverage: it must not silently reformat or clobber a format-1 repository in place.
#[test]
fn reinit_over_a_format1_repository_refuses_and_preserves_it() -> TestResult {
    let root = unique_root()?;
    build_legacy_fixture(&root, ActiveFixture::InterruptedPublication, b"1\n")?;
    let before = snapshot_tree(&root)?;

    let reinit = run(&root, &["init"])?;
    assert!(!reinit.status.success());
    assert_eq!(std::fs::read(root.join(".prikk/FORMAT"))?, b"1\n");
    assert_eq!(snapshot_tree(&root)?, before);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[path = "format_transition/matrix.rs"]
mod matrix;
