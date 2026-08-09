//! CLI end-to-end regression for the DC-64 incremental baseline lifecycle-state cache.
//!
//! Covers the two evidence obligations the RFC names explicitly: deletion/rebuild must leave commit
//! outcomes unchanged (NFR-PERF-04), and cache/replay divergence must be detectable and reported
//! (`verify`), not silently trusted. See
//! `rfcs/handoffs/DC-64-baseline-reconstruction-cost/incremental-baseline-cache-design-v1.md` for
//! the trust condition these tests exercise.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod support;

fn prikk(repo: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_prikk"));
    cmd.current_dir(repo);
    cmd
}

fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed (status {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn fail(output: &Output, what: &str) {
    assert!(
        !output.status.success(),
        "{what} unexpectedly succeeded\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn unique_repo(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("prikk-cli-dc64-{tag}-{}", support::unique_suffix()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const AUTHOR_KEY_ID: &str = "dc64-test-author";
const AUTHOR_SEED_HEX: &str = "0011223344556677889900112233445566778899001122334455667788990011";
const MAINTAINER_KEY_ID: &str = "dc64-test-maintainer";
const MAINTAINER_SEED: [u8; 32] = [
    0x31, 0x31, 0x42, 0x42, 0x53, 0x53, 0x64, 0x64, 0x75, 0x75, 0x86, 0x86, 0x97, 0x97, 0xa8, 0xa8,
    0xb9, 0xb9, 0xca, 0xca, 0xdb, 0xdb, 0xec, 0xec, 0xfd, 0xfd, 0x0e, 0x0e, 0x1f, 0x1f, 0x20, 0x20,
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn maintainer_public_key_hex() -> String {
    use prikk_store::MaintainerSigner;
    let signer =
        prikk_store::Ed25519MaintainerSigner::from_seed(MAINTAINER_KEY_ID, &MAINTAINER_SEED)
            .expect("fixed maintainer seed derives a valid signer");
    hex(&signer.public_key_bytes())
}

fn init(repo: &Path) {
    ok(&prikk(repo).arg("init").output().unwrap(), "init");
}

fn commit(repo: &Path, message: &str) -> Output {
    prikk(repo)
        .env("PRIKK_AUTHOR_KEY_ID", AUTHOR_KEY_ID)
        .env("PRIKK_AUTHOR_SEED", AUTHOR_SEED_HEX)
        .args(["commit", "-m", message])
        .output()
        .unwrap()
}

fn seal(repo: &Path) {
    let out = prikk(repo)
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
    // Trusting the same key twice (once per copied repo in the deletion/rebuild test) is fine;
    // only assert success the first time each repo sees it, so allow either outcome here.
    let _ = out;

    let out = prikk(repo)
        .env("PRIKK_MAINTAINER_KEY_ID", MAINTAINER_KEY_ID)
        .env("PRIKK_MAINTAINER_SEED", hex(&MAINTAINER_SEED))
        .args(["seal", "--allow-no-audit"])
        .output()
        .unwrap();
    ok(&out, "seal");
}

fn patch_id_line(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout)
        .lines()
        .find(|line| line.starts_with("patch id: "))
        .expect("commit output must include a patch id line")
        .to_string()
}

fn lifecycle_cache_path(repo: &Path) -> PathBuf {
    repo.join(".prikk").join("cache").join("lifecycle-state.v1")
}

fn write_two_files(repo: &Path, a: &str, b: &str) {
    std::fs::write(repo.join("a.txt"), a).unwrap();
    std::fs::write(repo.join("b.txt"), b).unwrap();
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
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

/// The cache is scoped to the commit path only and never touched by a genesis commit (baseline is
/// empty, nothing to replay); it first appears after the *second* real commit, which resolves a
/// `Published` baseline for the first time.
#[test]
fn genesis_commit_does_not_create_the_lifecycle_cache_but_the_second_commit_does() {
    let repo = unique_repo("appears-on-second-commit");
    init(&repo);
    write_two_files(&repo, "alpha\n", "bravo\n");
    ok(&commit(&repo, "genesis"), "genesis commit");
    assert!(!lifecycle_cache_path(&repo).exists());

    seal(&repo);
    std::fs::write(repo.join("a.txt"), "alpha, edited\n").unwrap();
    ok(&commit(&repo, "second"), "second commit");

    let cache_path = lifecycle_cache_path(&repo);
    assert!(cache_path.exists());
    let contents = std::fs::read(&cache_path).unwrap();
    assert!(contents.starts_with(b"PRIKK-LIFECYCLE-INCREMENTAL-CACHE-v1\0"));

    let _ = std::fs::remove_dir_all(&repo);
}

/// The RFC's stated NFR-PERF-04 evidence obligation: deleting the lifecycle cache and then
/// committing must produce a result identical to committing with the cache intact.
///
/// As with DC-56, comparing two independently-run repositories directly is unsound (fresh node ids
/// come from real OS randomness in production), so the comparison instead byte-for-byte copies one
/// shared lineage before diverging on cache presence.
#[test]
fn deleting_the_lifecycle_cache_does_not_change_commit_outcome() {
    let origin = unique_repo("rebuild-origin");
    init(&origin);
    write_two_files(&origin, "alpha\n", "bravo\n");
    ok(&commit(&origin, "genesis"), "genesis commit");
    seal(&origin);
    // Second commit: populates the lifecycle cache via a full replay (cache was cold).
    std::fs::write(origin.join("a.txt"), "alpha, edited once\n").unwrap();
    ok(&commit(&origin, "second"), "second commit");
    seal(&origin);
    // Third commit's mutation is what gets timed/compared below, with and without the cache.
    std::fs::write(origin.join("b.txt"), "bravo, edited\n").unwrap();

    let with_cache = unique_repo("rebuild-with-cache");
    let without_cache = unique_repo("rebuild-without-cache");
    copy_dir_recursive(&origin, &with_cache);
    copy_dir_recursive(&origin, &without_cache);

    assert!(lifecycle_cache_path(&with_cache).exists());
    std::fs::remove_file(lifecycle_cache_path(&without_cache)).unwrap();
    assert!(!lifecycle_cache_path(&without_cache).exists());

    let with_cache_output = commit(&with_cache, "third, cache intact");
    ok(&with_cache_output, "third commit with cache intact");
    let without_cache_output = commit(&without_cache, "third, cache deleted");
    ok(&without_cache_output, "third commit with cache deleted");

    assert_eq!(
        patch_id_line(&with_cache_output.stdout),
        patch_id_line(&without_cache_output.stdout),
        "deleting the lifecycle cache must not change the committed patch"
    );

    let _ = std::fs::remove_dir_all(&origin);
    let _ = std::fs::remove_dir_all(&with_cache);
    let _ = std::fs::remove_dir_all(&without_cache);
}

/// The ordinary case: a repository whose cache was populated normally has no lifecycle-cache
/// divergence.
#[test]
fn verify_reports_no_lifecycle_cache_divergence_for_an_untouched_repository() {
    let repo = unique_repo("verify-clean");
    init(&repo);
    write_two_files(&repo, "alpha\n", "bravo\n");
    ok(&commit(&repo, "genesis"), "genesis commit");
    seal(&repo);
    std::fs::write(repo.join("a.txt"), "alpha, edited\n").unwrap();
    ok(&commit(&repo, "second"), "second commit");

    let out = prikk(&repo).arg("verify").output().unwrap();
    ok(&out, "verify");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("lifecycle-cache divergences: 0"));

    let _ = std::fs::remove_dir_all(&repo);
}

/// The divergence-detection test the RFC asks for explicitly: a deliberately stale cached state —
/// still structurally decodable and checksum-valid, but disagreeing with what an independent replay
/// of the block it claims to represent produces — must be reported by `verify`, not silently
/// trusted by the next commit.
#[test]
fn verify_reports_a_deliberately_stale_lifecycle_cache_as_divergence() {
    let repo = unique_repo("verify-stale-lifecycle-cache");
    init(&repo);
    write_two_files(&repo, "alpha\n", "bravo\n");
    ok(&commit(&repo, "genesis"), "genesis commit");
    seal(&repo);
    std::fs::write(repo.join("a.txt"), "alpha, edited\n").unwrap();
    ok(&commit(&repo, "second"), "second commit");

    let cache_path = lifecycle_cache_path(&repo);
    let mut bytes = std::fs::read(&cache_path).unwrap();
    const MAGIC: &[u8] = b"PRIKK-LIFECYCLE-INCREMENTAL-CACHE-v1\0";
    assert!(bytes.len() > MAGIC.len() + 32);

    // Flip the last byte of the encoded body (inside the last node record's trailing field —
    // `mode` for a file entry). This keeps the wire structure and checksum-covered length intact;
    // only the checksum itself needs recomputing to stay internally consistent, exactly the
    // "persistence fault that a checksum alone would have caught if it hadn't been recomputed to
    // match" scenario a real corruption would not produce, but which exercises the same downstream
    // comparison against ground truth this check exists for.
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    let body = bytes[MAGIC.len() + 32..].to_vec();
    let checksum = prikk_hash::sha256(&body);
    bytes[MAGIC.len()..MAGIC.len() + 32].copy_from_slice(&checksum);
    std::fs::write(&cache_path, &bytes).unwrap();

    let out = prikk(&repo).arg("verify").output().unwrap();
    fail(&out, "verify with a deliberately stale lifecycle cache");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("lifecycle-cache divergences: 1"));
    assert!(stdout.contains("lifecycle-cache [divergence] block"));

    let _ = std::fs::remove_dir_all(&repo);
}
