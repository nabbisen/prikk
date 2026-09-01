//! `classify` is the part of RFC 121 §2.1's fix that decides whether a write error is silence or a
//! real failure. It is deliberately pure (no stdout, no `std::process::exit`) so that decision is
//! testable in-process -- `write_and_handle`'s own two divergent arms (`exit(0)`, `panic!`) cannot
//! be observed the same way: one would end the test process, and the other would need
//! `catch_unwind` to prove nothing more than what `classify` already proves directly.

use std::io::{self, ErrorKind};

use super::{WriteOutcome, classify};

#[test]
fn classify_passes_success_through() {
    assert!(matches!(classify(Ok(())), WriteOutcome::Ok));
}

#[test]
fn classify_recognizes_a_broken_pipe_specifically() {
    let err = io::Error::from(ErrorKind::BrokenPipe);
    assert!(matches!(classify(Err(err)), WriteOutcome::ClosedPipe));
}

/// The control that proves the fix is narrow: a write error that is not `BrokenPipe` (`ENOSPC` on a
/// redirected stdout, say) must not be classified the same way -- swallowing it alongside a closed
/// pipe would hide a genuine failure.
#[test]
fn classify_does_not_treat_other_errors_as_a_closed_pipe() {
    for kind in [
        ErrorKind::StorageFull,
        ErrorKind::PermissionDenied,
        ErrorKind::Interrupted,
        ErrorKind::WriteZero,
    ] {
        let err = io::Error::from(kind);
        assert!(
            matches!(classify(Err(err)), WriteOutcome::Failed(_)),
            "{kind:?} must not be classified as a closed pipe"
        );
    }
}
