//! DC-36 immutable publication conformance tests for `DurabilityContract::publish_immutable` (G5).
//!
//! **Re-targeted for RFC 102 Stage 3** (design-v1.md §12.3): `FileObjectStore::write_object` no
//! longer routes through this primitive -- object writes go through `index.rs`'s container append
//! protocol instead. Ruled "keep, do not delete": G5 still exists (its production caller is gone,
//! the primitive is not), and these tests are the record of what it still guarantees, called
//! directly through `publish_immutable_file` at a fixed test-only relative path rather than through
//! `FileObjectStore`'s (now container-backed) object addressing.

use std::path::{Path, PathBuf};

use prikk_object::{ObjectEnvelope, ObjectType};

use crate::file_codec::{decode_envelope_file, encode_envelope_file};
use crate::fsutil::publish_immutable_file;
// DC-97: no Windows failpoint wiring exists (same reason as G3/G8) -- every test using these stays
// inline-gated below.
use crate::RepositoryLayout;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::{dummy_signature, unique_temp_dir};
// DC-97: only used by the failpoint-gated tests below.
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::{DoctorSeverity, doctor_repository, verify_repository};

const TEST_OBJECT_RELATIVE: &str = "objects/g5-conformance-test.pobj";

fn signed_blob(payload: &[u8]) -> prikk_error::Result<ObjectEnvelope> {
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, payload.to_vec());
    envelope.add_signature(dummy_signature())?;
    Ok(envelope)
}

fn mutate_first_signature_byte(envelope: &mut ObjectEnvelope) -> prikk_error::Result<()> {
    let byte = envelope
        .signatures
        .first_mut()
        .and_then(|signature| signature.signature_bytes.first_mut())
        .ok_or_else(|| {
            prikk_error::PrikkError::Integrity("missing test signature byte".to_string())
        })?;
    *byte ^= 0xff;
    Ok(())
}

fn setup(name: &str) -> prikk_error::Result<(PathBuf, RepositoryLayout, ObjectEnvelope)> {
    let root = unique_temp_dir(name);
    let layout = RepositoryLayout::init(root.clone())?;
    std::fs::create_dir_all(layout.prikk_dir().join("objects"))?;
    Ok((root, layout, signed_blob(b"candidate")?))
}

fn absolute_test_object_path(layout: &RepositoryLayout) -> PathBuf {
    layout.prikk_dir().join(TEST_OBJECT_RELATIVE)
}

/// Mirrors `object_store.rs`'s own `validate_existing_object`, the validator every production
/// `publish_immutable` call used to run -- kept here verbatim so these tests still exercise the same
/// existing-bytes acceptance shape, not a simplified stand-in.
fn validate_existing_test_object(
    bytes: &[u8],
    expected_type: ObjectType,
    expected_id: prikk_object::ObjectId,
) -> prikk_error::Result<()> {
    let envelope = decode_envelope_file(bytes).map_err(|error| {
        prikk_error::PrikkError::Integrity(format!(
            "existing immutable object is malformed: {error}"
        ))
    })?;
    if envelope.object_type != expected_type {
        return Err(prikk_error::PrikkError::Integrity(format!(
            "existing object type {} differs from path type {expected_type}",
            envelope.object_type
        )));
    }
    let actual_id = envelope.object_id();
    if actual_id != expected_id {
        return Err(prikk_error::PrikkError::Integrity(format!(
            "existing object id {actual_id} differs from path id {expected_id}"
        )));
    }
    Ok(())
}

fn publish(layout: &RepositoryLayout, envelope: &ObjectEnvelope) -> prikk_error::Result<()> {
    let candidate = encode_envelope_file(envelope)?;
    let object_type = envelope.object_type;
    let object_id = envelope.object_id();
    publish_immutable_file(
        layout.repository_mutation_root(),
        Path::new(TEST_OBJECT_RELATIVE),
        &candidate,
        move |existing| validate_existing_test_object(existing, object_type, object_id),
    )
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn temp_paths(shard: &Path) -> prikk_error::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(shard)? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(".tmp."))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn exact_existing_bytes_are_synced_and_accepted() -> prikk_error::Result<()> {
    let (root, layout, envelope) = setup("object-exact-existing")?;
    publish(&layout, &envelope)?;
    fail_once_for_test(TestFailPoint::ImmutableCleanupSync);
    assert!(publish(&layout, &envelope).is_err());
    assert!(publish(&layout, &envelope).is_ok());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The property RFC 102 Stage 3 explicitly preserves (design-v1.md §12.3 item 2): a same-id,
/// different-bytes candidate is rejected, not silently accepted. Object writes no longer route
/// through this exact check, but the underlying detection still exists here, and the equivalent
/// guarantee for container writes is read-time content-hash validation (`index.rs`).
#[test]
fn same_object_id_with_different_signature_transport_is_rejected() -> prikk_error::Result<()> {
    let (root, layout, envelope) = setup("object-signature-mismatch")?;
    let mut different = envelope.clone();
    mutate_first_signature_byte(&mut different)?;
    assert_eq!(different.object_id(), envelope.object_id());
    publish(&layout, &envelope)?;
    assert!(publish(&layout, &different).is_err());
    assert_eq!(
        std::fs::read(absolute_test_object_path(&layout))?,
        encode_envelope_file(&envelope)?
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn malformed_wrong_id_and_wrong_type_existing_files_are_rejected() -> prikk_error::Result<()> {
    for (name, bytes) in [
        ("malformed", b"malformed".to_vec()),
        ("wrong-id", encode_envelope_file(&signed_blob(b"other")?)?),
        (
            "wrong-type",
            encode_envelope_file(&ObjectEnvelope::unsigned(ObjectType::Patch, 1, vec![]))?,
        ),
    ] {
        let (root, layout, envelope) = setup(&format!("object-{name}"))?;
        std::fs::write(absolute_test_object_path(&layout), bytes)?;
        assert!(publish(&layout, &envelope).is_err(), "case {name}");
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[cfg(target_family = "unix")]
#[test]
fn final_symlink_directory_and_symlinked_shard_are_rejected() -> prikk_error::Result<()> {
    use std::os::unix::fs::symlink;

    for kind in ["symlink", "directory", "shard-symlink"] {
        let (root, layout, envelope) = setup(&format!("object-{kind}"))?;
        let path = absolute_test_object_path(&layout);
        let shard = path
            .parent()
            .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?;
        let external = root.join("external");
        std::fs::create_dir_all(&external)?;
        match kind {
            "symlink" => {
                std::fs::create_dir_all(shard)?;
                let target = external.join("winner");
                std::fs::write(&target, encode_envelope_file(&envelope)?)?;
                symlink(target, &path)?;
            }
            "directory" => std::fs::create_dir_all(&path)?,
            "shard-symlink" => {
                std::fs::remove_dir_all(shard)?;
                symlink(&external, shard)?;
            }
            _ => unreachable!(),
        }
        assert!(publish(&layout, &envelope).is_err(), "case {kind}");
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

/// DC-81: ported per POSIX.1-2017 §2.9.7 (see the equivalent note on
/// `fsutil::tests::append_and_truncate_reject_fifo_without_blocking`) — a genuine port, not a
/// recompile; macOS runtime behavior needs the CI job to confirm, not asserted here.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn final_fifo_is_rejected_without_blocking() -> prikk_error::Result<()> {
    use std::sync::mpsc;
    use std::time::Duration;

    use crate::test_support::create_fifo_for_test;

    let (root, layout, envelope) = setup("object-fifo")?;
    let path = absolute_test_object_path(&layout);
    create_fifo_for_test(&path, 0o600)?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(publish(&layout, &envelope));
    });
    let result = receiver.recv_timeout(Duration::from_secs(1));
    let publication = result.map_err(|_| {
        prikk_error::PrikkError::Io("FIFO rejection exceeded the bounded wait".to_string())
    })?;
    assert!(publication.is_err());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn immutable_failpoints_retain_required_artifacts_and_retry() -> prikk_error::Result<()> {
    for point in [
        TestFailPoint::ImmutableFileSync,
        TestFailPoint::ImmutableInstall,
        TestFailPoint::ImmutableInstallUnsupported,
        TestFailPoint::ImmutableInstallNoSys,
        TestFailPoint::ImmutableInstallPermission,
        TestFailPoint::ImmutableInstallSync,
        TestFailPoint::ImmutableTempUnlink,
        TestFailPoint::ImmutableCleanupSync,
    ] {
        let (root, layout, envelope) = setup(&format!("object-{point:?}"))?;
        let path = absolute_test_object_path(&layout);
        let shard = path
            .parent()
            .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?
            .to_path_buf();
        fail_once_for_test(point);
        let error = match publish(&layout, &envelope) {
            Ok(()) => {
                return Err(prikk_error::PrikkError::Integrity(
                    "injected immutable failure unexpectedly succeeded".to_string(),
                ));
            }
            Err(error) => error,
        };
        let installed = path.is_file();
        let temps = temp_paths(&shard)?;
        match point {
            TestFailPoint::ImmutableFileSync | TestFailPoint::ImmutableInstall => {
                assert!(!installed);
                assert_eq!(temps.len(), 1);
            }
            TestFailPoint::ImmutableInstallUnsupported
            | TestFailPoint::ImmutableInstallNoSys
            | TestFailPoint::ImmutableInstallPermission => {
                assert!(
                    error
                        .to_string()
                        .contains("unsupported by filesystem or policy")
                );
                assert!(!installed);
                assert_eq!(temps.len(), 1);
            }
            TestFailPoint::ImmutableInstallSync | TestFailPoint::ImmutableTempUnlink => {
                assert!(installed);
                assert_eq!(temps.len(), 1);
            }
            TestFailPoint::ImmutableCleanupSync => {
                assert!(installed);
                assert!(temps.is_empty());
            }
            _ => unreachable!(),
        }
        assert!(publish(&layout, &envelope).is_ok());
        assert!(path.is_file());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

/// The dormant diagnostic this exercises (`verification.object_temp_paths`,
/// `PRIKK-DOCTOR-OBJECT-TEMP-DEBRIS`) is kept per design-v1.md §12.3 item 3 -- it can no longer fire
/// from a real `FileObjectStore` write under format-3, but `verify`/`doctor` still scan for this
/// exact debris shape, at a real per-type object path (`layout.object_path`, still the shape that
/// scan looks in -- untouched by the container rewire since object addressing isn't its concern),
/// standing in for whatever future caller might still produce it. This is the one test in this file
/// that cannot use the fixed `TEST_OBJECT_RELATIVE` path every other test uses, precisely because
/// the diagnostic it proves still works is itself scoped to that real addressing scheme.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn crash_left_temp_is_ignored_by_reads_and_warned_without_cleanup() -> prikk_error::Result<()> {
    let (root, layout, envelope) = setup("object-temp-diagnostics")?;
    let path = layout.object_path(envelope.object_type, envelope.object_id());
    let shard = path
        .parent()
        .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?
        .to_path_buf();
    std::fs::create_dir_all(&shard)?;
    let relative = layout.repository_relative(&path)?;
    let candidate = encode_envelope_file(&envelope)?;
    let object_type = envelope.object_type;
    let object_id = envelope.object_id();
    fail_once_for_test(TestFailPoint::ImmutableFileSync);
    assert!(
        publish_immutable_file(
            layout.repository_mutation_root(),
            &relative,
            &candidate,
            move |existing| validate_existing_test_object(existing, object_type, object_id),
        )
        .is_err()
    );
    let temps = temp_paths(&shard)?;
    assert_eq!(temps.len(), 1);

    let verification = verify_repository(&layout)?;
    assert_eq!(verification.object_temp_paths, temps);
    let doctor = doctor_repository(&layout);
    assert!(doctor.is_healthy());
    assert!(doctor.issues.iter().any(|issue| {
        issue.code == "PRIKK-DOCTOR-OBJECT-TEMP-DEBRIS" && issue.severity == DoctorSeverity::Warning
    }));
    assert!(temps.first().is_some_and(|path| path.is_file()));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
