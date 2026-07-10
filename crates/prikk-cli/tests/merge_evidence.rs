//! CLI regression tests for the read-only `merge-evidence` command.

use std::collections::BTreeMap;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use prikk_store::{Ed25519MaintainerSigner, MaintainerSigner};

type TestResult<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn prikk(repo: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_prikk"));
    cmd.current_dir(repo);
    cmd
}

fn ok(output: &Output, what: &str) -> TestResult {
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{what} failed (status {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
    .into())
}

fn fail(output: &Output, what: &str) -> TestResult {
    if !output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{what} unexpectedly succeeded\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
    .into())
}

fn unique_repo(tag: &str) -> TestResult<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "prikk-cli-merge-evidence-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn maintainer_seed() -> &'static str {
    "111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
}

fn maintainer_signer() -> TestResult<Ed25519MaintainerSigner> {
    Ok(Ed25519MaintainerSigner::from_seed(
        "merge-evidence-maintainer",
        &[
            0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44, 0x55, 0x55, 0x66, 0x66, 0x77, 0x77,
            0x88, 0x88, 0x99, 0x99, 0xaa, 0xaa, 0xbb, 0xbb, 0xcc, 0xcc, 0xdd, 0xdd, 0xee, 0xee,
            0xff, 0xff, 0x00, 0x00,
        ],
    )?)
}

fn public_key_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn add_trusted_maintainer(repo: &Path) -> TestResult {
    let signer = maintainer_signer()?;
    let out = prikk(repo)
        .args([
            "trust",
            "maintainer",
            "add",
            "--key-id",
            "merge-evidence-maintainer",
            "--public-key",
            &public_key_hex(&signer.public_key_bytes()),
        ])
        .output()?;
    ok(&out, "trust maintainer add")
}

fn commit_worktree(repo: &Path, message: &str) -> TestResult {
    let out = prikk(repo)
        .env("PRIKK_AUTHOR_KEY_ID", "merge-evidence-author")
        .env(
            "PRIKK_AUTHOR_SEED",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        )
        .args(["commit", "-m", message])
        .output()?;
    ok(&out, "commit")
}

fn seal_current(repo: &Path) -> TestResult<String> {
    let out = prikk(repo)
        .env("PRIKK_MAINTAINER_KEY_ID", "merge-evidence-maintainer")
        .env("PRIKK_MAINTAINER_SEED", maintainer_seed())
        .args(["seal", "--allow-no-audit"])
        .output()?;
    ok(&out, "seal")?;
    seal_block_id(&String::from_utf8_lossy(&out.stdout))
}

fn init_with_sealed_genesis(repo: &Path) -> TestResult<String> {
    let out = prikk(repo).arg("init").output()?;
    ok(&out, "init")?;
    std::fs::write(repo.join("readme.txt"), b"hello prikk\n")?;
    commit_worktree(repo, "genesis")?;
    add_trusted_maintainer(repo)?;
    seal_current(repo)
}

#[test]
fn merge_evidence_ref_targets_are_success_and_read_only() -> TestResult {
    let repo = unique_repo("ref-targets")?;
    let baseline = init_with_sealed_genesis(&repo)?;
    let before = snapshot_files(&repo)?;

    let out = prikk(&repo)
        .args([
            "merge-evidence",
            "--baseline-block",
            &baseline,
            "--left-ref",
            "heads/main",
            "--right-ref",
            "heads/main",
        ])
        .output()?;
    ok(&out, "merge-evidence")?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("outcome: Confluent"), "stdout: {stdout}");
    assert!(
        stdout.contains("reason: proven_confluent"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("left selector: ref heads/main"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("right selector: ref heads/main"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("no merge commit, ref update, WAL write, or worktree change"),
        "stdout: {stdout}"
    );
    assert_eq!(snapshot_files(&repo)?, before);
    let _ = std::fs::remove_dir_all(repo);
    Ok(())
}

#[test]
fn merge_evidence_output_does_not_leak_file_content_or_host_paths() -> TestResult {
    let repo = unique_repo("privacy")?;
    let baseline = init_with_sealed_genesis(&repo)?;
    let secret = "MERGE_EVIDENCE_SECRET_PAYLOAD_DO_NOT_PRINT";
    std::fs::write(repo.join("secret.txt"), format!("{secret}\n"))?;
    commit_worktree(&repo, "secret candidate")?;
    let _target = seal_current(&repo)?;

    let out = prikk(&repo)
        .args([
            "merge-evidence",
            "--baseline-block",
            &baseline,
            "--left-ref",
            "heads/main",
            "--right-ref",
            "heads/main",
        ])
        .output()?;
    ok(&out, "merge-evidence privacy")?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stdout.contains(secret), "stdout leaked content: {stdout}");
    assert!(!stderr.contains(secret), "stderr leaked content: {stderr}");
    let repo_path = repo.to_string_lossy();
    assert!(
        !stdout.contains(repo_path.as_ref()),
        "stdout leaked host path: {stdout}"
    );
    assert!(
        !stderr.contains(repo_path.as_ref()),
        "stderr leaked host path: {stderr}"
    );
    let _ = std::fs::remove_dir_all(repo);
    Ok(())
}

#[test]
fn merge_evidence_missing_selector_is_command_failure() -> TestResult {
    let repo = unique_repo("missing-selector")?;
    let out = prikk(&repo).arg("init").output()?;
    ok(&out, "init")?;
    let out = prikk(&repo)
        .args(["merge-evidence", "--baseline-block", &"0".repeat(64)])
        .output()?;
    fail(&out, "merge-evidence missing selector")?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("requires --left-block or --left-ref"),
        "stderr: {stderr}"
    );
    let _ = std::fs::remove_dir_all(repo);
    Ok(())
}

#[test]
fn merge_evidence_failure_path_is_read_only() -> TestResult {
    let repo = unique_repo("failure-read-only")?;
    let _baseline = init_with_sealed_genesis(&repo)?;
    let before = snapshot_files(&repo)?;

    let out = prikk(&repo)
        .args([
            "merge-evidence",
            "--baseline-block",
            &"0".repeat(64),
            "--left-ref",
            "heads/main",
            "--right-ref",
            "heads/main",
        ])
        .output()?;
    fail(&out, "merge-evidence missing baseline")?;
    assert_eq!(snapshot_files(&repo)?, before);
    let _ = std::fs::remove_dir_all(repo);
    Ok(())
}

fn seal_block_id(stdout: &str) -> TestResult<String> {
    let block_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("block id: "))
        .ok_or_else(|| io::Error::other("seal output did not include block id"))?;
    Ok(block_id.to_string())
}

fn snapshot_files(root: &Path) -> TestResult<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files)?;
    Ok(files)
}

fn collect_files(root: &Path, current: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> TestResult {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|err| io::Error::other(err.to_string()))?
                .to_string_lossy()
                .to_string();
            files.insert(relative, std::fs::read(path)?);
        }
    }
    Ok(())
}
