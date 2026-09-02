//! RFC 123 §4 Option C-revised: `commit -m <message>` requires and validates a message, then
//! discards it -- silently, before this note existed. Drives the compiled binary to prove the note
//! actually prints, and that the claim it makes (the message never reaches `prikk log`) is true, not
//! merely asserted.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

mod support;

#[test]
fn commit_prints_the_message_not_stored_note() {
    let repo = support::unique_repo("rfc123-message-note");
    support::init(&repo);
    std::fs::write(repo.join("f.txt"), "hello").unwrap();
    let output = support::commit(&repo, "heads/main", "a distinctive test message");
    support::ok(&output, "commit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "note: the message is validated but not stored -- it will not appear in `prikk log`"
        ),
        "stdout: {stdout}"
    );
}

/// The note's own claim, demonstrated rather than assumed: the message really does not reach
/// `prikk log`, sealed or not.
#[test]
fn the_message_never_reaches_log() {
    let repo = support::unique_repo("rfc123-message-note-log");
    support::init(&repo);
    std::fs::write(repo.join("f.txt"), "hello").unwrap();
    support::ok(
        &support::commit(&repo, "heads/main", "a distinctive test message"),
        "commit",
    );
    support::ok(&support::seal(&repo, "heads/main"), "seal");

    let log = support::prikk(&repo).arg("log").output().unwrap();
    support::ok(&log, "log");
    let log_stdout = String::from_utf8_lossy(&log.stdout);
    assert!(
        !log_stdout.contains("a distinctive test message"),
        "log must not show the commit message: {log_stdout}"
    );
}

/// `-m` is still required -- this increment adds a note, not an optional flag.
#[test]
fn dash_m_is_still_required() {
    let repo = support::unique_repo("rfc123-message-note-dash-m");
    support::init(&repo);
    std::fs::write(repo.join("f.txt"), "hello").unwrap();
    let output = support::prikk(&repo)
        .env("PRIKK_AUTHOR_KEY_ID", support::AUTHOR_KEY_ID)
        .env("PRIKK_AUTHOR_SEED", support::AUTHOR_SEED_HEX)
        .arg("commit")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "commit with no -m must still refuse"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("-m"), "stderr: {stderr}");
}
