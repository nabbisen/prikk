//! Handoff §6 control 1, the rigorous form: same input, two **separate process invocations** of
//! the compiled `extract-profile` binary, byte-identical stdout.
//!
//! An in-process double call of the library function would not actually catch the realistic way
//! this breaks: Rust's default `HashMap` hasher is randomized *per process*, not per call, so two
//! calls to a `HashMap`-using extractor inside the same test process would still agree with each
//! other even if the implementation were nondeterministic across real runs. Two child processes
//! each get their own randomized hasher state, so this is the form that would actually have
//! failed had `extract.rs` used `HashMap` instead of `BTreeMap` anywhere in the histogram-building
//! path -- confirmed by temporarily introducing one during development and watching this test fail
//! (see the implementation report).

#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn control1_two_separate_processes_produce_byte_identical_output() {
    let run = || {
        let output = Command::new(env!("CARGO_BIN_EXE_extract-profile"))
            .arg(fixture("log.txt"))
            .arg(fixture("ls-tree.txt"))
            .arg(fixture("context.toml"))
            .output()
            .expect("extract-profile must run");
        assert!(
            output.status.success(),
            "extract-profile failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };

    let first = run();
    let second = run();
    assert_eq!(
        first, second,
        "two separate process invocations of the same input must produce byte-identical output"
    );
}
