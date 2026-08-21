//! RFC 116 stage 4 handoff §6 — an end-to-end two-repository sync driven entirely through the
//! `prikk sync` CLI, no direct library calls. This is criterion 1's evidence: two machines can
//! exchange sealed history and both verify it afterward, through the surface a real user actually
//! has (RFC 116 ruling 2: no network, no transport -- the artifact files just move by whatever
//! means the operator already has, simulated here by both repositories sharing a filesystem).

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::path::PathBuf;

mod support;

use prikk_store::{
    DEFAULT_HAVE_LIST_MAX_PATCH_COUNT, DEFAULT_HAVE_LIST_MAX_TOTAL_BYTES, decode_have_list,
};

fn sync_file(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "rfc116-sync-cli-{tag}-{}.bin",
        support::unique_suffix()
    ))
}

/// §6, steps 1-5, driven entirely through the CLI:
/// 1. Repo A seals a patch. Repo B is empty.
/// 2. `sync summary` in A -> file. `sync compare --summary` in B -> reports the ref differs.
/// 3. `sync have` in B -> file. `sync build --have` in A -> artifact.
/// 4. `sync accept` in B -> prints a claim id. `sync seal --claim` in B.
/// 5. B's ref tip now reaches A's patch -- read back via `sync have` in B again, decoded, not
///    inferred from any subcommand's exit code.
#[test]
fn end_to_end_sync_via_the_cli_lands_the_delta_and_is_verified_by_reading_it_back() {
    let repo_a = support::unique_repo("rfc116-sync-a");
    support::init(&repo_a);
    support::generation(
        &repo_a,
        "heads/main",
        "a.txt",
        b"rfc116 sync cli\n",
        "first",
    );

    let repo_b = support::unique_repo("rfc116-sync-b");
    support::init(&repo_b);

    // Step 2: `sync summary` in A.
    let summary_file = sync_file("summary");
    let summary = support::prikk(&repo_a)
        .args([
            "sync",
            "summary",
            "--output",
            summary_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    support::ok(&summary, "sync summary");
    let summary_stdout = String::from_utf8_lossy(&summary.stdout);
    assert!(
        summary_stdout.contains("refs: 1"),
        "A must report exactly one ref in its summary: {summary_stdout}"
    );

    // Step 2: `sync compare --summary` in B must report the ref differs (B has no `heads/main`
    // at all, so this also exercises the `RemoteOnly`... `LocalOnly` naming from B's perspective:
    // B does not hold the ref, so it must read `remote-only` from B's own point of view).
    let compare = support::prikk(&repo_b)
        .args([
            "sync",
            "compare",
            "--summary",
            summary_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    support::ok(&compare, "sync compare");
    let compare_stdout = String::from_utf8_lossy(&compare.stdout);
    assert!(
        compare_stdout.contains("heads/main remote-only"),
        "B must report heads/main as remote-only (B does not hold it yet): {compare_stdout}"
    );

    // Step 3: `sync have` in B (B holds nothing, so this is an empty have-list) -> file.
    let have_file = sync_file("have");
    let have = support::prikk(&repo_b)
        .args([
            "sync",
            "have",
            "heads/main",
            "--output",
            have_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    support::ok(&have, "sync have");

    // Step 3: `sync build --have` in A -> artifact. A must be able to sign the claim -- adopt and
    // use the same fixed maintainer key `support::seal` already trusted in A during setup.
    let artifact_file = sync_file("artifact");
    let build = support::prikk(&repo_a)
        .env("PRIKK_MAINTAINER_KEY_ID", support::MAINTAINER_KEY_ID)
        .env(
            "PRIKK_MAINTAINER_SEED",
            support::hex(&support::MAINTAINER_SEED),
        )
        .args([
            "sync",
            "build",
            "heads/main",
            "--have",
            have_file.to_str().unwrap(),
            "--output",
            artifact_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    support::ok(&build, "sync build");
    let build_stdout = String::from_utf8_lossy(&build.stdout);
    assert!(
        build_stdout.contains("delta patches: 1") && build_stdout.contains("claims: 1"),
        "A must report exactly one delta patch and one claim: {build_stdout}"
    );

    // Step 4: `sync accept` in B -> prints a claim id, the load-bearing output this step exists
    // for (handoff §3).
    let accept = support::prikk(&repo_b)
        .args(["sync", "accept", artifact_file.to_str().unwrap()])
        .output()
        .unwrap();
    support::ok(&accept, "sync accept");
    let accept_stdout = String::from_utf8_lossy(&accept.stdout);
    assert!(
        accept_stdout.contains("patches: 1") && accept_stdout.contains("claims: 1"),
        "B must report exactly one accepted patch and one claim: {accept_stdout}"
    );
    let claim_line = accept_stdout
        .lines()
        .find(|line| line.trim_start().starts_with("claim "))
        .unwrap_or_else(|| panic!("sync accept must print a claim line: {accept_stdout}"));
    let claim_id = claim_line
        .trim_start()
        .strip_prefix("claim ")
        .and_then(|rest| rest.split(':').next())
        .unwrap_or_else(|| panic!("could not parse a claim id from: {claim_line}"))
        .trim();

    // `sync pending` in B must show the accepted-but-unsealed patch before sealing -- the only
    // observable evidence an accept did anything, per handoff §3.
    let pending = support::prikk(&repo_b)
        .args(["sync", "pending"])
        .output()
        .unwrap();
    support::ok(&pending, "sync pending");
    let pending_stdout = String::from_utf8_lossy(&pending.stdout);
    assert!(
        pending_stdout.contains("pending (accepted, unsealed) patches: 1"),
        "B must show exactly one pending patch before sealing: {pending_stdout}"
    );

    // Step 4: `sync seal --claim` in B. B needs its own trusted maintainer key to seal under --
    // the same fixed key, adopted here explicitly since B never ran `seal` itself.
    support::trust_maintainer(&repo_b);
    let seal = support::prikk(&repo_b)
        .env("PRIKK_MAINTAINER_KEY_ID", support::MAINTAINER_KEY_ID)
        .env(
            "PRIKK_MAINTAINER_SEED",
            support::hex(&support::MAINTAINER_SEED),
        )
        .args(["sync", "seal", "heads/main", "--claim", claim_id])
        .output()
        .unwrap();
    support::ok(&seal, "sync seal");
    let seal_stdout = String::from_utf8_lossy(&seal.stdout);
    assert!(
        seal_stdout.contains("sealed 1 patch(es)"),
        "B must report sealing exactly one patch: {seal_stdout}"
    );

    // Pending must now be empty -- the sealed patch is no longer accepted-but-unsealed.
    let pending_after = support::prikk(&repo_b)
        .args(["sync", "pending"])
        .output()
        .unwrap();
    support::ok(&pending_after, "sync pending after seal");
    assert!(
        String::from_utf8_lossy(&pending_after.stdout)
            .contains("pending (accepted, unsealed) patches: 0"),
        "no patch should remain pending after seal"
    );

    // Step 5, the assertion that matters: read B's own ref tip back through the CLI (`sync have`
    // again), decode it, and confirm the synced patch is reachable -- not inferred from any exit
    // code above.
    let have_after_file = sync_file("have-after");
    let have_after = support::prikk(&repo_b)
        .args([
            "sync",
            "have",
            "heads/main",
            "--output",
            have_after_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    support::ok(&have_after, "sync have after seal");
    let have_after_bytes = std::fs::read(&have_after_file).unwrap();
    let have_after_list = decode_have_list(
        &have_after_bytes,
        DEFAULT_HAVE_LIST_MAX_TOTAL_BYTES,
        DEFAULT_HAVE_LIST_MAX_PATCH_COUNT,
    )
    .unwrap();
    assert_eq!(
        have_after_list.patch_ids.len(),
        1,
        "B's own heads/main must now reach exactly the one synced patch"
    );

    // And a final `sync compare` from B's own summary against A's must read in-sync.
    let a_summary_again = sync_file("summary-final");
    let summary_again = support::prikk(&repo_a)
        .args([
            "sync",
            "summary",
            "--output",
            a_summary_again.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    support::ok(&summary_again, "sync summary (final)");
    let compare_final = support::prikk(&repo_b)
        .args([
            "sync",
            "compare",
            "--summary",
            a_summary_again.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    support::ok(&compare_final, "sync compare (final)");
    assert!(
        String::from_utf8_lossy(&compare_final.stdout).contains("heads/main in-sync"),
        "after the full loop, B must report heads/main as in-sync with A"
    );

    let _ = std::fs::remove_dir_all(repo_a);
    let _ = std::fs::remove_dir_all(repo_b);
    for file in [
        summary_file,
        have_file,
        artifact_file,
        have_after_file,
        a_summary_again,
    ] {
        let _ = std::fs::remove_file(file);
    }
}
