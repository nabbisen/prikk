#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::{destination_exists, write_new_file_durably};
use std::path::PathBuf;

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "prikk-durable-output-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn destination_exists_is_true_for_a_real_file_and_false_for_absence() {
    let dir = unique_dir("exists");
    let present = dir.join("present.bin");
    let absent = dir.join("absent.bin");
    std::fs::write(&present, b"anything").unwrap();

    assert!(destination_exists(&present));
    assert!(!destination_exists(&absent));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn write_new_file_durably_writes_exact_bytes_to_a_new_destination() {
    let dir = unique_dir("new");
    let destination = dir.join("backup.bundle");

    write_new_file_durably(&destination, b"hello durable world").unwrap();

    assert_eq!(std::fs::read(&destination).unwrap(), b"hello durable world");

    let _ = std::fs::remove_dir_all(dir);
}

/// Control 3's "permitted case succeeds" half, at this layer: a second write to the same
/// destination replaces the content -- the collision *policy* lives in `bundle.rs`'s own
/// `--force` check, not here; this function's own job is only ever the write.
#[test]
fn write_new_file_durably_overwrites_existing_content_completely() {
    let dir = unique_dir("overwrite");
    let destination = dir.join("backup.bundle");
    std::fs::write(&destination, b"old content, much longer than the new one").unwrap();

    write_new_file_durably(&destination, b"new").unwrap();

    assert_eq!(std::fs::read(&destination).unwrap(), b"new");

    let _ = std::fs::remove_dir_all(dir);
}

/// Control 1, the decisive one, and control 2 together: an unwritable directory makes
/// `File::create_new` for the temp file fail before a single byte of the attempted new content is
/// written anywhere -- the earliest possible failure point, and the cleanest demonstration that a
/// failure before the rename touches neither the pre-existing destination nor creates any temp
/// file at all. A permission-based failure is `#[cfg(unix)]`-only (Windows ACL semantics differ
/// enough that this project already gates its own permission-bit assertions the same way,
/// `dc67_ordinary_use_conformance.rs`'s DC-71 note) -- not a narrower claim about the production
/// code, only about which failure-injection technique is portable enough to assert on here.
#[cfg(unix)]
#[test]
fn a_failed_write_leaves_the_previous_destination_intact_and_creates_no_temp_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = unique_dir("failed-write-intact");
    let destination = dir.join("backup.bundle");
    std::fs::write(&destination, b"the only real backup, must survive").unwrap();

    let original_mode = std::fs::metadata(&dir).unwrap().permissions().mode();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();

    let result = write_new_file_durably(&destination, b"attempted replacement");

    // Restore write permission before any assertion can panic and skip cleanup.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(original_mode)).unwrap();

    assert!(
        result.is_err(),
        "the write must fail when its own directory refuses new files"
    );
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"the only real backup, must survive",
        "the previous destination must be completely untouched by a failed write"
    );
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(
        entries,
        vec![std::ffi::OsString::from("backup.bundle")],
        "no temp file may exist after a failed write: {entries:?}"
    );

    let _ = std::fs::remove_dir_all(dir);
}

/// Control 2, restated for the no-prior-destination case: a failure whose parent directory does
/// not exist at all leaves nothing at the destination either. The two tests together cover both
/// of control 2's own halves ("neither at the destination nor as an abandoned temp file") in the
/// two distinct starting states a real export can be in: overwriting, and writing fresh.
#[test]
fn a_failed_write_to_a_missing_directory_leaves_nothing_behind() {
    let dir = unique_dir("failed-write-nothing");
    let destination = dir.join("does-not-exist-yet").join("backup.bundle");

    let result = write_new_file_durably(&destination, b"never written");
    assert!(result.is_err());
    assert!(
        !destination_exists(&destination),
        "nothing must appear at the destination"
    );

    let _ = std::fs::remove_dir_all(dir);
}
