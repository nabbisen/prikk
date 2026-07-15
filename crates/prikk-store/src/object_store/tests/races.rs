//! Same-ID immutable publication race tests.

use std::sync::{Arc, Barrier};

use prikk_object::{ObjectEnvelope, ObjectType};

use crate::fsutil::{TestFailPoint, fail_once_for_test, set_immutable_install_barrier_for_test};
use crate::test_support::{dummy_signature, unique_temp_dir};
use crate::{FileObjectStore, ObjectWriter, RepositoryLayout};

fn transport(variant: u8) -> prikk_error::Result<ObjectEnvelope> {
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"shared-payload".to_vec());
    let mut signature = dummy_signature();
    let byte = signature.signature_bytes.first_mut().ok_or_else(|| {
        prikk_error::PrikkError::Integrity("missing test signature byte".to_string())
    })?;
    *byte = variant;
    envelope.add_signature(signature)?;
    Ok(envelope)
}

#[test]
fn same_process_race_never_overwrites_winning_transport() -> prikk_error::Result<()> {
    let root = unique_temp_dir("object-thread-race");
    let layout = RepositoryLayout::init(root.clone())?;
    let left = transport(1)?;
    let right = transport(2)?;
    assert_eq!(left.object_id(), right.object_id());
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for envelope in [left.clone(), right.clone()] {
        let thread_layout = layout.clone();
        let thread_barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let mut store = FileObjectStore::new(thread_layout);
            set_immutable_install_barrier_for_test(thread_barrier);
            store.write_object(&envelope)
        }));
    }
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .map_err(|_| prikk_error::PrikkError::Io("object race thread panicked".to_string()))
        })
        .collect::<prikk_error::Result<_>>()?;
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let bytes = std::fs::read(layout.object_path(ObjectType::Blob, left.object_id()))?;
    let left_bytes = crate::file_codec::encode_envelope_file(&left)?;
    let right_bytes = crate::file_codec::encode_envelope_file(&right)?;
    assert!(bytes == left_bytes || bytes == right_bytes);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn same_transport_race_is_idempotent_and_leaves_no_temp() -> prikk_error::Result<()> {
    let root = unique_temp_dir("object-equal-thread-race");
    let layout = RepositoryLayout::init(root.clone())?;
    let envelope = transport(1)?;
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let thread_layout = layout.clone();
        let thread_envelope = envelope.clone();
        let thread_barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let mut store = FileObjectStore::new(thread_layout);
            set_immutable_install_barrier_for_test(thread_barrier);
            store.write_object(&thread_envelope)
        }));
    }
    for handle in handles {
        assert_eq!(
            handle.join().map_err(|_| {
                prikk_error::PrikkError::Io("object race thread panicked".to_string())
            })??,
            envelope.object_id()
        );
    }
    let path = layout.object_path(ObjectType::Blob, envelope.object_id());
    let shard = path
        .parent()
        .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?;
    assert_eq!(
        std::fs::read(&path)?,
        crate::file_codec::encode_envelope_file(&envelope)?
    );
    for entry in std::fs::read_dir(shard)? {
        assert!(!entry?.file_name().to_string_lossy().contains(".tmp."));
    }
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn separate_process_race_never_overwrites_winning_transport() -> prikk_error::Result<()> {
    let root = unique_temp_dir("object-process-race");
    RepositoryLayout::init(root.clone())?;
    let executable = std::env::current_exe()?;
    let test_name = "object_store::tests::races::object_writer_process_helper";
    let mut children = Vec::new();
    for variant in [1_u8, 2_u8] {
        children.push(
            std::process::Command::new(&executable)
                .args(["--exact", test_name])
                .env("PRIKK_DC36_OBJECT_ROOT", &root)
                .env("PRIKK_DC36_OBJECT_VARIANT", variant.to_string())
                .spawn()?,
        );
    }
    let statuses: Vec<_> = children
        .into_iter()
        .map(|mut child| child.wait())
        .collect::<std::io::Result<_>>()?;
    assert_eq!(statuses.iter().filter(|status| status.success()).count(), 1);
    assert_eq!(
        statuses.iter().filter(|status| !status.success()).count(),
        1
    );

    let envelope = transport(1)?;
    let path =
        RepositoryLayout::open(root.clone())?.object_path(ObjectType::Blob, envelope.object_id());
    let bytes = std::fs::read(path)?;
    assert!(
        bytes == crate::file_codec::encode_envelope_file(&transport(1)?)?
            || bytes == crate::file_codec::encode_envelope_file(&transport(2)?)?
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn fresh_process_retry_resyncs_installed_final_without_cleaning_old_temp() -> prikk_error::Result<()>
{
    let root = unique_temp_dir("object-process-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let executable = std::env::current_exe()?;
    let first = run_child(&executable, &root, 1, Some("install-sync"))?;
    assert!(!first.success());
    let envelope = transport(1)?;
    let path = layout.object_path(ObjectType::Blob, envelope.object_id());
    assert!(path.is_file());
    let shard = path
        .parent()
        .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?;
    let old_temp_count = std::fs::read_dir(shard)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
        .count();
    assert_eq!(old_temp_count, 1);

    let sync_failure = run_child(&executable, &root, 1, Some("cleanup-sync"))?;
    assert!(!sync_failure.success());
    assert!(run_child(&executable, &root, 1, None)?.success());
    let retained_temp_count = std::fs::read_dir(shard)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
        .count();
    assert_eq!(retained_temp_count, 1);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

fn run_child(
    executable: &std::path::Path,
    root: &std::path::Path,
    variant: u8,
    failpoint: Option<&str>,
) -> std::io::Result<std::process::ExitStatus> {
    let mut command = std::process::Command::new(executable);
    command
        .args([
            "--exact",
            "object_store::tests::races::object_writer_process_helper",
        ])
        .env("PRIKK_DC36_OBJECT_ROOT", root)
        .env("PRIKK_DC36_OBJECT_VARIANT", variant.to_string());
    if let Some(failpoint) = failpoint {
        command.env("PRIKK_DC36_OBJECT_FAILPOINT", failpoint);
    }
    command.status()
}

#[test]
fn object_writer_process_helper() {
    let Ok(root) = std::env::var("PRIKK_DC36_OBJECT_ROOT") else {
        return;
    };
    let variant = std::env::var("PRIKK_DC36_OBJECT_VARIANT")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or_default();
    match std::env::var("PRIKK_DC36_OBJECT_FAILPOINT").as_deref() {
        Ok("install-sync") => fail_once_for_test(TestFailPoint::ImmutableInstallSync),
        Ok("cleanup-sync") => fail_once_for_test(TestFailPoint::ImmutableCleanupSync),
        _ => {}
    }
    let result = RepositoryLayout::open(root)
        .and_then(|layout| transport(variant).map(|envelope| (layout, envelope)))
        .and_then(|(layout, envelope)| FileObjectStore::new(layout).write_object(&envelope));
    if result.is_err() {
        std::process::exit(23);
    }
}
