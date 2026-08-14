use super::*;

#[test]
fn current_format_seal_rejects_every_strict_wal_read_failure_without_mutation() -> TestResult {
    for failure in [
        StrictFailure::MalformedLength,
        StrictFailure::Duplicate,
        StrictFailure::InvertedOrder,
    ] {
        let root = unique_root()?;
        build_current_format_strict_wal_fixture(&root, failure)?;
        let before = snapshot_tree(&root)?;
        let output = run_owned(&root, &["seal".to_string(), "--allow-no-audit".to_string()])?;
        assert!(
            !output.status.success(),
            "{failure:?} unexpectedly sealed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(snapshot_tree(&root)?, before, "{failure:?} changed bytes");
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}
