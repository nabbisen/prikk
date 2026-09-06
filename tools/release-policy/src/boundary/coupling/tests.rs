#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::{DECLARED_CYCLES, DECLARED_HUBS, check, check_allowlists_are_well_formed};
use crate::boundary::BoundaryError;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root")
        .to_path_buf()
}

#[test]
fn the_real_repository_passes_with_no_undeclared_cycle_or_hub() {
    let mut errors: Vec<BoundaryError> = Vec::new();
    check(&repo_root(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

/// Self-guard on the escape hatch itself (`DECLARED_UNDOCUMENTED`'s own tests are the model): a
/// stale or misspelled entry must not silently exempt nothing.
#[test]
fn declared_cycles_have_real_reasons_and_removal_statements() {
    for entry in DECLARED_CYCLES {
        assert!(!entry.edges.is_empty(), "{:?}", entry.edges);
        assert!(entry.reason.trim().len() >= 20, "{:?}", entry.edges);
        assert!(
            entry.what_would_remove_it.trim().len() >= 20,
            "{:?}",
            entry.edges
        );
    }
}

#[test]
fn declared_hubs_have_real_reasons() {
    for entry in DECLARED_HUBS {
        assert!(entry.reason.trim().len() >= 20, "{}", entry.module);
    }
}

/// Control 4: an entry with an empty or placeholder reason is refused, same as
/// `DECLARED_UNDOCUMENTED`'s own guard.
#[test]
fn a_placeholder_reason_is_refused() {
    struct FakeCycle {
        reason: &'static str,
        what_would_remove_it: &'static str,
    }
    // Mirrors `check_allowlists_are_well_formed`'s own logic against a deliberately bad entry,
    // since the real constant cannot be mutated for a test.
    let fake = FakeCycle {
        reason: "todo",
        what_would_remove_it: "n/a",
    };
    assert!(super::is_placeholder(fake.reason));
    assert!(super::is_placeholder(fake.what_would_remove_it));
    let mut errors = Vec::new();
    check_allowlists_are_well_formed(&mut errors);
    assert!(
        errors.is_empty(),
        "the real DECLARED_CYCLES/DECLARED_HUBS must already be well-formed"
    );
}
