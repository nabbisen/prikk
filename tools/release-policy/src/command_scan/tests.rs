use super::{Invocation, invocations, scan, scan_shell, scan_yaml};

#[test]
fn recognizes_bounded_equivalent_forms() {
    for text in [
        "python3  release/check-policy.py",
        "env python3 ./release/check-policy.py",
        "python3 \\\n          release/check-policy.py",
        "printf '#'; python3 release/check-policy.py",
        "printf \"#\"; python3 -I -E -s -B ./release/check-policy.py",
        ": ''#x; python3 release/check-policy.py",
        ": \"\"#x; python3 release/check-policy.py",
        "python3 -I -E -s -B -- ./release/check-policy.py",
    ] {
        assert_eq!(invocations(text), vec![Invocation::PythonPolicy]);
    }
    for text in [
        "cargo  publish --workspace",
        "env cargo \\\n          publish --workspace",
        "command cargo\tpublish -p prikk-release-policy",
        "cmd=(cargo publish --workspace)",
        "publish_all() { cargo publish --workspace; }",
        "run: >-\n  cargo\n  publish --workspace",
        "printf '#'; cargo publish --workspace",
        "printf \"#\"; cargo publish --workspace",
        ": ''#x; cargo publish --workspace",
        ": \"\"#x; cargo publish --workspace",
    ] {
        assert!(matches!(
            invocations(text).as_slice(),
            [Invocation::Publication { phase, .. }] if phase == "publish"
        ));
    }
    assert_eq!(
        invocations("env cargo run --locked -p prikk-release-policy -- check"),
        vec![Invocation::RustPolicy]
    );
}

#[test]
fn real_comments_and_malformed_commands_fail_closed() {
    assert!(invocations("printf ok # cargo publish --workspace").is_empty());
    assert!(invocations("printf ok # python3 release/check-policy.py").is_empty());
    assert!(invocations(": '' # cargo publish --workspace").is_empty());
    assert!(invocations(": \"\" # python3 release/check-policy.py").is_empty());
    assert_eq!(scan("cargo 'publish").errors, vec!["unterminated-quote"]);
    assert_eq!(
        scan("python3 -W ignore release/check-policy.py").errors,
        vec!["unsupported-python-invocation"]
    );
    let dynamic_publication = scan("$CARGO publish --workspace");
    assert!(
        dynamic_publication
            .errors
            .contains(&"unsupported-cargo-executable")
    );
    assert!(
        dynamic_publication
            .errors
            .contains(&"unsupported-publication-invocation")
    );
    let dynamic_python = scan("$PYTHON release/check-policy.py");
    assert!(
        dynamic_python
            .errors
            .contains(&"unsupported-dynamic-command-head")
    );
    assert!(
        dynamic_python
            .errors
            .contains(&"unsupported-python-invocation")
    );
    for command in [
        "$CARGO run --locked -p prikk-release-policy -- check",
        "${CARGO} run --locked -p prikk-release-policy -- check",
        "$(resolve-cargo) run --locked -p prikk-release-policy -- check",
    ] {
        assert!(!scan(command).errors.is_empty(), "{command}");
    }
    for command in [
        "cargo \"$PHASE\" --workspace",
        "cargo \"${PHASE}\" --workspace",
        "cargo \"$ACTION\" --workspace",
        "cargo \"${ACTION}\" --workspace",
        "$CARGO \"$PHASE\" --workspace",
        "$TOOL \"$ACTION\" --workspace",
        "$TOOL publish --workspace",
        "env $TOOL publish --workspace",
        "env MODE=release ${TOOL} \"${ACTION}\" --workspace",
        "run: $TOOL \"$ACTION\" --workspace",
        "- run: $TOOL \"$ACTION\" --workspace",
        "  - run: $TOOL \"$ACTION\" --workspace",
        "- { run: $TOOL \"$ACTION\" --workspace }",
        "- {run: $TOOL \"$ACTION\" --workspace}",
        "env -u UNUSED $TOOL \"$ACTION\" --workspace",
        "env -C DIR $TOOL \"$ACTION\" --workspace",
        "env --unset UNUSED --chdir DIR $TOOL \"$ACTION\" --workspace",
        "command -p $TOOL \"$ACTION\" --workspace",
        "nice $TOOL \"$ACTION\" --workspace",
        "exec $TOOL \"$ACTION\" --workspace",
        "timeout 600 $TOOL \"$ACTION\" --workspace",
        "xargs $TOOL \"$ACTION\" --workspace",
        "nohup $TOOL \"$ACTION\" --workspace",
        "stdbuf -oL $TOOL \"$ACTION\" --workspace",
        "setsid $TOOL \"$ACTION\" --workspace",
        "ionice $TOOL \"$ACTION\" --workspace",
        "time $TOOL \"$ACTION\" --workspace",
        "project-wrapper $TOOL \"$ACTION\" --workspace",
        "- run: nice $TOOL \"$ACTION\" --workspace",
    ] {
        assert!(!scan(command).errors.is_empty(), "{command}");
    }
    for command in [
        "env --unknown VALUE $TOOL \"$ACTION\" --workspace",
        "env -u",
        "command --unknown $TOOL \"$ACTION\" --workspace",
    ] {
        assert!(!scan(command).errors.is_empty(), "{command}");
    }
    for command in [
        "echo \"$VALUE\"",
        "printf '%s' \"$VALUE\"",
        "printf '%s' '`literal`'",
        "url: ${{ steps.deployment.outputs.page_url }}",
    ] {
        assert!(scan(command).errors.is_empty(), "{command}");
    }
    for command in [
        "`deploy`",
        "`resolve-tool` publish",
        "sh -c 'cargo publish'",
        "bash -c \"cargo package --workspace\"",
        "bash -lc 'cargo publish'",
        "/bin/sh -c 'cargo publish'",
        "dash -c 'cargo publish'",
        "ksh -c 'cargo publish'",
        "zsh -c 'cargo publish'",
        "env sh -c 'cargo publish'",
        "eval 'cargo publish'",
    ] {
        assert!(!scan(command).errors.is_empty(), "{command}");
    }
}

#[test]
fn governed_procedures_are_default_closed() {
    for command in [
        "python3 -c 'import os; os.system(\"cargo publish\")'",
        "perl -e 'system(\"cargo publish\")'",
        "node -e 'require(\"child_process\").execSync(\"cargo publish\")'",
        "ruby -e 'system(\"cargo publish\")'",
        "nice sh -c 'cargo publish'",
        "xargs sh -c 'cargo publish'",
        "timeout 600 bash -c 'cargo publish'",
        "project-local-wrapper cargo fmt --check",
    ] {
        assert!(!scan_shell(command).errors.is_empty(), "{command}");
        assert!(
            !scan_yaml(&format!("- run: {command}")).errors.is_empty(),
            "{command}"
        );
    }

    let workflow = "url: ${{ steps.deployment.outputs.page_url }}\n\
                    - run: cargo fmt --check\n\
                    - run: cargo clippy --workspace --all-targets -- -D warnings\n\
                    - run: cargo test --workspace\n\
                    - run: mdbook build\n";
    assert!(scan_yaml(workflow).errors.is_empty());
}
