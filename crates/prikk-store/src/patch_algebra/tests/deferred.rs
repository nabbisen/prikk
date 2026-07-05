use super::*;

#[test]
fn rename_is_unknown_not_independent() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "old.bin", blob(1), MODE_REGULAR);
    let left = rename_path(1, node(1), "old.bin", "new.bin");
    let right = change_perm(2, node(1), MODE_REGULAR, MODE_EXECUTABLE);

    assert_unknown(
        classify_pair(&baseline, &left, &right),
        UnknownReason::RenameDeferred,
    );
}

#[test]
fn symlink_is_unknown_not_independent() {
    let baseline = NodeLifecycleState::new();
    let left = create_symlink(1, "link", node(1), "target");
    let right = create_file(2, "other", node(2), blob(1), MODE_REGULAR);

    assert_unknown(
        classify_pair(&baseline, &left, &right),
        UnknownReason::SymlinkDeferred,
    );
}

#[test]
fn malformed_path_is_unknown_not_skipped() {
    let baseline = NodeLifecycleState::new();
    let left = create_file(1, "/absolute", node(1), blob(1), MODE_REGULAR);
    let right = create_file(2, "ok", node(2), blob(2), MODE_REGULAR);

    assert_unknown(
        classify_pair(&baseline, &left, &right),
        UnknownReason::MalformedOperation,
    );
}
