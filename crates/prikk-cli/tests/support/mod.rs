//! Shared CLI end-to-end test harness (DC-67).
//!
//! DC-61, DC-65, and DC-66's test files each rolled their own `commit`/`seal`/key setup — copy-pasted
//! three times before this consolidation. Every test in `dc67_ordinary_use_conformance.rs` uses this
//! module instead of a fourth (and fifth, sixth, ...) copy. Existing files are left as they are; this
//! is not a retrofit, only the point past which no one should copy-paste it again.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn prikk(repo: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_prikk"));
    cmd.current_dir(repo);
    cmd
}

pub fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed (status {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

pub fn unique_repo(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "prikk-cli-dc67-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub const AUTHOR_KEY_ID: &str = "dc67-test-author";
pub const AUTHOR_SEED_HEX: &str =
    "3300445566778899001122334455667788990011223344556677889900112233";
pub const MAINTAINER_KEY_ID: &str = "dc67-test-maintainer";
pub const MAINTAINER_SEED: [u8; 32] = [
    0x71, 0x71, 0x82, 0x82, 0x93, 0x93, 0xa4, 0xa4, 0xb5, 0xb5, 0xc6, 0xc6, 0xd7, 0xd7, 0xe8, 0xe8,
    0xf9, 0xf9, 0x0a, 0x0a, 0x1b, 0x1b, 0x2c, 0x2c, 0x3d, 0x3d, 0x4e, 0x4e, 0x5f, 0x5f, 0x60, 0x60,
];

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn maintainer_public_key_hex() -> String {
    use prikk_store::MaintainerSigner;
    let signer =
        prikk_store::Ed25519MaintainerSigner::from_seed(MAINTAINER_KEY_ID, &MAINTAINER_SEED)
            .expect("fixed maintainer seed derives a valid signer");
    hex(&signer.public_key_bytes())
}

pub fn init(repo: &Path) {
    ok(&prikk(repo).arg("init").output().unwrap(), "init");
}

/// Commit on `ref_name` (repository-relative, e.g. `"heads/main"`).
pub fn commit(repo: &Path, ref_name: &str, message: &str) -> Output {
    prikk(repo)
        .env("PRIKK_AUTHOR_KEY_ID", AUTHOR_KEY_ID)
        .env("PRIKK_AUTHOR_SEED", AUTHOR_SEED_HEX)
        .args(["commit", "--ref", ref_name, "-m", message])
        .output()
        .unwrap()
}

/// Trust the fixed maintainer key (idempotent-enough for repeated calls within one test — only
/// `seal` itself must succeed).
pub fn trust_maintainer(repo: &Path) {
    let _ = prikk(repo)
        .args([
            "trust",
            "maintainer",
            "add",
            "--key-id",
            MAINTAINER_KEY_ID,
            "--public-key",
            &maintainer_public_key_hex(),
        ])
        .output()
        .unwrap();
}

/// Seal `ref_name`, trusting the fixed maintainer key first.
pub fn seal(repo: &Path, ref_name: &str) -> Output {
    trust_maintainer(repo);
    prikk(repo)
        .env("PRIKK_MAINTAINER_KEY_ID", MAINTAINER_KEY_ID)
        .env("PRIKK_MAINTAINER_SEED", hex(&MAINTAINER_SEED))
        .args(["seal", "--allow-no-audit", "--ref", ref_name])
        .output()
        .unwrap()
}

/// One generation: write `path` with `content`, commit, seal — the mutate/commit/seal cycle §3
/// defines a "generation" as. Asserts both steps succeed.
pub fn generation(repo: &Path, ref_name: &str, path: &str, content: &[u8], message: &str) {
    std::fs::write(repo.join(path), content).unwrap();
    ok(
        &commit(repo, ref_name, message),
        &format!("commit: {message}"),
    );
    ok(&seal(repo, ref_name), &format!("seal: {message}"));
}

pub fn branch_create(repo: &Path, name: &str, from: &str) -> Output {
    trust_maintainer(repo);
    prikk(repo)
        .env("PRIKK_MAINTAINER_KEY_ID", MAINTAINER_KEY_ID)
        .env("PRIKK_MAINTAINER_SEED", hex(&MAINTAINER_SEED))
        .args(["branch", "create", name, "--from", from])
        .output()
        .unwrap()
}

pub fn branch_close(repo: &Path, name: &str) -> Output {
    trust_maintainer(repo);
    prikk(repo)
        .env("PRIKK_MAINTAINER_KEY_ID", MAINTAINER_KEY_ID)
        .env("PRIKK_MAINTAINER_SEED", hex(&MAINTAINER_SEED))
        .args(["branch", "close", name])
        .output()
        .unwrap()
}

pub fn tag_create(repo: &Path, name: &str, target: &str) -> Output {
    trust_maintainer(repo);
    prikk(repo)
        .env("PRIKK_MAINTAINER_KEY_ID", MAINTAINER_KEY_ID)
        .env("PRIKK_MAINTAINER_SEED", hex(&MAINTAINER_SEED))
        .args(["tag", "create", name, "--target", target])
        .output()
        .unwrap()
}

pub fn verify(repo: &Path) -> Output {
    prikk(repo).arg("verify").output().unwrap()
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), &dst_path).unwrap();
        }
    }
}

/// DC-67 criterion 2, the load-bearing technique: copy `repo`'s `.prikk` into a fresh directory,
/// `checkout --patch-materialize` it there, and return the rebuilt worktree's root. `verify` passing
/// proves history is *structurally* valid; reading files back from the returned root and asserting
/// their bytes is what proves it is *semantically* correct.
///
/// `checkout --patch-materialize` takes the **repository** path (the directory containing `.prikk`),
/// not an output directory — passing the wrong one is silently plausible and was gotten wrong twice
/// during DC-66 verification.
pub fn rebuild_from_sealed_history(repo: &Path, tag: &str) -> PathBuf {
    let materialize_root = unique_repo(&format!("{tag}-materialize"));
    std::fs::create_dir_all(materialize_root.join(".prikk")).unwrap();
    copy_dir_recursive(&repo.join(".prikk"), &materialize_root.join(".prikk"));
    let out = prikk(&materialize_root)
        .arg("checkout")
        .arg("--patch-materialize")
        .output()
        .unwrap();
    ok(&out, "checkout --patch-materialize");
    materialize_root
}
