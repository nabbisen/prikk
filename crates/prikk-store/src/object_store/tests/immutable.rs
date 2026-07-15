//! DC-36 immutable object publication tests.

use std::path::{Path, PathBuf};

use prikk_object::{ObjectEnvelope, ObjectType};

use crate::file_codec::encode_envelope_file;
use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::{dummy_signature, unique_temp_dir};
use crate::{
    DoctorSeverity, FileObjectStore, ObjectReader, ObjectWriter, RepositoryLayout,
    doctor_repository, verify_repository,
};

fn signed_blob(payload: &[u8]) -> prikk_error::Result<ObjectEnvelope> {
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, payload.to_vec());
    envelope.add_signature(dummy_signature())?;
    Ok(envelope)
}

fn required_parent(path: &Path) -> prikk_error::Result<&Path> {
    path.parent()
        .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))
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
    Ok((root, layout, signed_blob(b"candidate")?))
}

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

#[test]
fn exact_existing_bytes_are_synced_and_accepted() -> prikk_error::Result<()> {
    let (root, layout, envelope) = setup("object-exact-existing")?;
    let mut store = FileObjectStore::new(layout);
    let id = store.write_object(&envelope)?;
    fail_once_for_test(TestFailPoint::ImmutableCleanupSync);
    assert!(store.write_object(&envelope).is_err());
    assert_eq!(store.write_object(&envelope)?, id);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn same_object_id_with_different_signature_transport_is_rejected() -> prikk_error::Result<()> {
    let (root, layout, envelope) = setup("object-signature-mismatch")?;
    let mut different = envelope.clone();
    mutate_first_signature_byte(&mut different)?;
    assert_eq!(different.object_id(), envelope.object_id());
    let mut store = FileObjectStore::new(layout.clone());
    store.write_object(&envelope)?;
    assert!(store.write_object(&different).is_err());
    assert_eq!(
        std::fs::read(layout.object_path(ObjectType::Blob, envelope.object_id()))?,
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
        let path = layout.object_path(ObjectType::Blob, envelope.object_id());
        std::fs::create_dir_all(required_parent(&path)?)?;
        std::fs::write(&path, bytes)?;
        let mut store = FileObjectStore::new(layout);
        assert!(store.write_object(&envelope).is_err(), "case {name}");
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
        let path = layout.object_path(ObjectType::Blob, envelope.object_id());
        let shard = required_parent(&path)?;
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
            "shard-symlink" => symlink(&external, shard)?,
            _ => unreachable!(),
        }
        let mut store = FileObjectStore::new(layout);
        assert!(store.write_object(&envelope).is_err(), "case {kind}");
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn final_fifo_is_rejected_without_blocking() -> prikk_error::Result<()> {
    use std::sync::mpsc;
    use std::time::Duration;

    use rustix::fs::{CWD, Mode, mkfifoat};

    let (root, layout, envelope) = setup("object-fifo")?;
    let path = layout.object_path(ObjectType::Blob, envelope.object_id());
    std::fs::create_dir_all(required_parent(&path)?)?;
    mkfifoat(CWD, &path, Mode::from_raw_mode(0o600))
        .map_err(|error| std::io::Error::from_raw_os_error(error.raw_os_error()))?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut store = FileObjectStore::new(layout);
        let _ = sender.send(store.write_object(&envelope));
    });
    let result = receiver.recv_timeout(Duration::from_secs(1));
    let publication = result.map_err(|_| {
        prikk_error::PrikkError::Io("FIFO rejection exceeded the bounded wait".to_string())
    })?;
    assert!(publication.is_err());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

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
        let path = layout.object_path(ObjectType::Blob, envelope.object_id());
        let shard = required_parent(&path)?.to_path_buf();
        let mut store = FileObjectStore::new(layout);
        fail_once_for_test(point);
        let error = match store.write_object(&envelope) {
            Ok(_) => {
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
        assert_eq!(store.write_object(&envelope)?, envelope.object_id());
        assert!(path.is_file());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn crash_left_temp_is_ignored_by_reads_and_warned_without_cleanup() -> prikk_error::Result<()> {
    let (root, layout, envelope) = setup("object-temp-diagnostics")?;
    let id = envelope.object_id();
    let shard = layout
        .object_path(ObjectType::Blob, id)
        .parent()
        .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?
        .to_path_buf();
    let mut store = FileObjectStore::new(layout.clone());
    fail_once_for_test(TestFailPoint::ImmutableFileSync);
    assert!(store.write_object(&envelope).is_err());
    let temps = temp_paths(&shard)?;
    assert_eq!(temps.len(), 1);
    assert_eq!(store.read_object(id)?, None);

    let verification = verify_repository(&layout)?;
    assert_eq!(verification.checked_objects, 0);
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
