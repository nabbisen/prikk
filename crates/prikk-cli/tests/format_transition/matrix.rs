use super::*;

#[test]
fn format1_history_command_matrix_is_bounded_and_byte_preserving() -> TestResult {
    let root = unique_root()?;
    let fixture = build_legacy_fixture(&root, ActiveFixture::RollbackDraft)?;
    let baseline = fixture.root_block.to_string();
    let left = fixture.left_block.to_string();
    let right = fixture.right_block.to_string();
    let cases = vec![
        ("status", vec!["status"], true, true),
        ("log", vec!["log"], true, true),
        ("worktree status", vec!["worktree-status"], true, true),
        ("verify", vec!["verify"], false, true),
        ("doctor", vec!["doctor"], true, true),
        ("checkout plan", vec!["checkout", "--plan-only"], true, true),
        (
            "snapshot plan",
            vec!["checkout", "--snapshot-plan"],
            true,
            true,
        ),
        ("patch plan", vec!["checkout", "--patch-plan"], true, true),
        (
            "patch deletion plan",
            vec!["checkout", "--patch-delete-plan"],
            true,
            true,
        ),
        ("inverse plan", vec!["inverse-plan"], true, true),
        ("rollback preview", vec!["rollback-preview"], true, true),
        (
            "rollback draft verify",
            vec!["rollback-draft-verify"],
            true,
            true,
        ),
        (
            "doctor WAL repair",
            vec!["doctor", "--repair-wal-tail"],
            false,
            true,
        ),
        (
            "doctor ref repair",
            vec!["doctor", "--repair-main-ref"],
            false,
            true,
        ),
        (
            "snapshot materialize",
            vec!["checkout", "--snapshot-materialize"],
            false,
            true,
        ),
        (
            "patch materialize",
            vec!["checkout", "--patch-materialize"],
            false,
            true,
        ),
        (
            "patch materialize delete",
            vec!["checkout", "--patch-materialize-delete"],
            false,
            true,
        ),
        ("commit", vec!["commit", "-m", "refused"], false, true),
        (
            "rollback draft append",
            vec!["rollback-draft", "--append-inverse", "-m", "refused"],
            false,
            true,
        ),
        (
            "trust mutation",
            vec![
                "trust",
                "maintainer",
                "add",
                "--key-id",
                "refused",
                "--public-key",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ],
            false,
            true,
        ),
        (
            "ordinary seal",
            vec!["seal", "--allow-no-audit"],
            false,
            true,
        ),
        ("re-init", vec!["init"], false, false),
    ];

    for (label, args, should_succeed, expects_warning) in cases {
        let args = args.into_iter().map(str::to_string).collect::<Vec<_>>();
        let before = snapshot_tree(&root)?;
        let output = run_owned(&root, &args)?;
        assert_eq!(
            output.status.success(),
            should_succeed,
            "{label}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if expects_warning {
            assert_legacy_warning(&output);
        }
        if !should_succeed && label != "verify" && label != "re-init" {
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("unsupported format version: 1"),
                "{label}: unexpected refusal: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert_eq!(snapshot_tree(&root)?, before, "{label} changed bytes");
    }

    for command in ["merge-evidence", "merge-plan"] {
        let args = vec![
            command.to_string(),
            "--baseline-block".to_string(),
            baseline.clone(),
            "--left-block".to_string(),
            left.clone(),
            "--right-block".to_string(),
            right.clone(),
        ];
        let before = snapshot_tree(&root)?;
        let output = run_owned(&root, &args)?;
        assert!(
            output.status.success(),
            "{command}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_legacy_warning(&output);
        assert_eq!(snapshot_tree(&root)?, before, "{command} changed bytes");
    }

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn format1_exact_signer_backed_seal_completion_is_the_only_mutation() -> TestResult {
    let root = unique_root()?;
    let fixture = build_legacy_fixture(&root, ActiveFixture::InterruptedPublication)?;
    let log_before = std::fs::read(&fixture.log_path)?;
    let block_before = std::fs::read(&fixture.block_path)?;
    let output = run_owned(&root, &["seal".to_string(), "--allow-no-audit".to_string()])?;
    assert!(
        output.status.success(),
        "exact completion failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_legacy_warning(&output);
    assert_eq!(std::fs::read(&fixture.log_path)?, log_before);
    assert_eq!(std::fs::read(&fixture.block_path)?, block_before);
    assert!(root.join(".prikk/refs/by-id").read_dir()?.next().is_some());
    assert!(std::fs::read(root.join(".prikk/active/default/queue.wal"))?.is_empty());
    assert!(!root.join(".prikk/active/default/ref-name").exists());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn format2_seal_rejects_every_strict_wal_read_failure_without_mutation() -> TestResult {
    for failure in [
        StrictFailure::MalformedLength,
        StrictFailure::Duplicate,
        StrictFailure::InvertedOrder,
    ] {
        let root = unique_root()?;
        build_format2_strict_wal_fixture(&root, failure)?;
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
