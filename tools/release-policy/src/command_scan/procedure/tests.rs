#![allow(clippy::unwrap_used)]

use super::yaml_scripts;

#[test]
fn extracts_scalar_sequence_flow_and_blocks() {
    let scripts = yaml_scripts(
        r#"url: ${{ metadata }}
defaults:
  run:
    working-directory: docs
- run: cargo test --workspace
-  run: mdbook build
- "run": cargo fmt --check
- 'run': cargo test --workspace
- run : mdbook build
- {name: book, run: cargo fmt --check}
- {run: cargo test --workspace, name: test}
- name: block step
  run: mdbook build
- run: >-
    cargo
    fmt --check
- run: |
    echo first
    echo second
"#,
    )
    .unwrap();
    assert_eq!(
        scripts,
        [
            "cargo test --workspace",
            "mdbook build",
            "cargo fmt --check",
            "cargo test --workspace",
            "mdbook build",
            "cargo fmt --check",
            "cargo test --workspace",
            "mdbook build",
            "cargo fmt --check",
            "echo first\necho second",
        ]
    );
}

#[test]
fn malformed_run_scalar_fails_closed() {
    assert_eq!(
        yaml_scripts("- run: 'unterminated"),
        Err("unsupported-quoted-yaml-run-scalar")
    );
    assert_eq!(yaml_scripts("- run:"), Err("empty-yaml-run-scalar"));
    assert_eq!(
        yaml_scripts("- {name: deploy, run cargo publish}"),
        Err("malformed-yaml-flow-field")
    );
}
