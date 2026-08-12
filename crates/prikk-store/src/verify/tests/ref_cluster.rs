//! DC-95 Stage 1, round 5: the `refs/verify.rs` + `refs/verify/scan.rs` cluster -- ref
//! pointer/log structural checks, a new fixture family (pointer/RefState construction) distinct
//! from round 1-4's object-envelope placement. This round covers the three checks reachable with
//! nothing beyond a normal `RefStore::publish` plus one extra low-level filesystem operation --
//! no failpoints, no format-1 flips, no raw payload byte-patching. The remaining ~10-11 checks in
//! this cluster (format-1-specific rows, byte-level log construction, two rows that may be
//! structurally unreachable -- duplicate pointer/log identity, both content-addressed by ref name
//! and gated by their own canonical-path check before the duplicate-insert step) are later rounds.
//!
//! Two of the three are load-bearing (dangling ref target; non-canonical ref pointer path, the
//! latter only once its fixture was fixed to move rather than copy the pointer -- see that test's
//! own doc comment). The third (RefState name mismatches pointer) is downstream-redundant with
//! the pointer/log coherence classifier in `refs/verify.rs`, discovered only by inspecting the
//! full report returned with the check disabled rather than trusting that a `panic!` on the `Ok`
//! arm alone proved a clean pass.
//!
//! Round 6 adds `ensure_ref_path_shape` -- provably downstream-redundant with the pointer/log
//! canonical-path checks, not merely observed for one fixture (see that test's own doc comment for
//! the structural argument). Classifications for rounds 1-5 were re-verified against a genuinely
//! clean baseline per DC-95-stage-1-round-5-review-v1's condition; see `verify/tests.rs` and this
//! file's own load-bearing doc comments for that evidence. The corrected methodology (adopted
//! signer where a Block/RefState is involved, full report inspected on every probe) applies from
//! round 6 onward.

use prikk_error::Result;
use prikk_object::ObjectType;

use crate::test_support::{
    sample_object_id, signed_empty_block_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepositoryLayout, verify_repository,
};

/// Publishes `ref_name` pointing at a freshly written, real `Block`, via the normal `RefStore::
/// publish` path -- exactly what every check in this file assumes has already happened cleanly,
/// so the test's own fixture-construction step is the one deliberate irregularity, isolated.
fn publish_ref_to_new_block(
    layout: &RepositoryLayout,
    objects: &mut FileObjectStore,
    ref_name: &str,
) -> Result<(prikk_object::ObjectId, prikk_object::ObjectId)> {
    let block = signed_empty_block_envelope();
    let block_id = objects.write_object(&block)?;
    let ref_state = signed_ref_state_envelope(ref_name, None, block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope(ref_name, None, ref_state_id, block_id, 1);
    RefStore::new(layout.clone()).publish(&RefPublication {
        ref_name: ref_name.to_string(),
        expected_previous_ref_state_id: None,
        ref_state,
        ref_update,
    })?;
    Ok((ref_state_id, block_id))
}

/// `ensure_ref_target_valid` (`refs/verify/scan.rs`) -- a `Branch`-kind `RefState` whose
/// `target_object_id` no longer resolves to a `Block`. Published normally (so the pointer, log,
/// and `RefState` are all mutually coherent, matching every other fixture in this cluster), then
/// the target `Block` is deleted post-publish -- something no `RefStore::publish` path can itself
/// produce, since `validate_coherent_publication` only cross-checks the publication's own fields
/// against each other, never that the target object continues to exist afterward. `verify_objects`
/// runs before `verify_refs` (`verify.rs:268-269`) but never notices: it only iterates objects that
/// still exist, and nothing about a leftover dangling pointer touches the object-count scan.
/// **Probed, load-bearing, confirmed**: disabling `ensure_ref_target_valid`'s block-existence arm
/// lets `verify_repository` return `Ok`. **Re-verified against a genuinely clean baseline**
/// (DC-95-stage-1-round-5-review-v1 §2-4): the original probe used this file's fake, unadopted
/// signer for the Block/RefState pair, so the disabled-check `Ok` always carried a `PRIKK-TRUST-
/// POLICY-INVALID` finding regardless of `ensure_ref_target_valid`'s own state -- unable to
/// distinguish load-bearing from downstream-redundant. Re-probed with a real, adopted signer behind
/// both the Block and the RefState: disabling the check now returns `Ok` with `ref_publication_
/// issues`, `publication_trust_issues`, and `signature_envelope_issues` all empty. Classification
/// unchanged: load-bearing.
#[test]
fn verify_repository_detects_dangling_ref_target() -> Result<()> {
    let root = unique_temp_dir("verify-dangling-ref-target");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());

    let (_, block_id) = publish_ref_to_new_block(&layout, &mut objects, "heads/main")?;
    std::fs::remove_file(layout.object_path(ObjectType::Block, block_id))?;

    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a dangling ref target"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("targets missing block"),
        "expected a dangling-ref-target error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The "non-canonical ref pointer" check (`refs/verify/scan.rs`) -- a pointer file whose own
/// filename does not match `layout.ref_pointer_path` for the ref name *encoded inside it*, even
/// though the encoded content is otherwise a perfectly valid, real pointer. Built by publishing
/// `heads/aux` normally, then *moving* (not copying) the resulting real pointer file to a second,
/// still validly-shaped (64 hex chars + `.ref`) filename elsewhere in `by-id/` -- leaving exactly
/// one on-disk pointer for `heads/aux`, at the wrong path, and its ref-log untouched at its own
/// correct location. **The move-not-copy choice is load-bearing for what this test actually
/// proves, discovered by getting it wrong first**: an earlier version of this fixture *copied* the
/// pointer bytes, leaving the original in place too -- disabling the canonical-path check then
/// didn't produce a clean `Ok`, it produced a *different* hard `Err`, `"duplicate pointer identity
/// for heads/aux"`, because both files decoded to the same ref name and the second `insert` into
/// `read_pointers`' map collided with the first. That's a genuine finding about the copy
/// construction, not about this check -- with only one pointer on disk (moved, not duplicated),
/// disabling the check lets the misplaced, but otherwise perfectly coherent, entry insert cleanly
/// and match its own untouched log with no divergence. This check runs immediately after decode,
/// strictly before the RefState or log are even read (`scan.rs`'s `read_pointers`).
/// **Probed, load-bearing, confirmed** (with the move-based fixture): disabling this check lets
/// `verify_repository` return `Ok`. **Re-verified against a genuinely clean baseline**
/// (DC-95-stage-1-round-5-review-v1 §2-4), for the same reason as `ensure_ref_target_valid` above:
/// re-probed with a real, adopted signer behind the Block/RefState pair, disabling the check still
/// returns `Ok` with every issue vector empty. Classification unchanged: load-bearing.
#[test]
fn verify_repository_detects_noncanonical_ref_pointer_path() -> Result<()> {
    let root = unique_temp_dir("verify-noncanonical-ref-pointer");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());

    publish_ref_to_new_block(&layout, &mut objects, "heads/aux")?;
    let canonical = layout.ref_pointer_path("heads/aux");
    let misplaced = layout
        .refs_dir()
        .join("by-id")
        .join(format!("{}.ref", sample_object_id("misplaced-pointer")));
    std::fs::rename(&canonical, &misplaced)?;

    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a non-canonical ref pointer path"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("non-canonical ref pointer"),
        "expected a non-canonical-ref-pointer error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// "RefState name differs from pointer ref" (`refs/verify/scan.rs`) -- a pointer file, at its own
/// canonical location, whose encoded ref name disagrees with the `ref_name` field inside the
/// `RefState` object it points to. `RefStore::publish`'s own `validate_coherent_publication`
/// (`refs/publication.rs:164`) rejects exactly this disagreement between `publication.ref_name`
/// and the `RefState`'s internal name before anything is ever written, so this fixture cannot be
/// built through the public API at all -- it needs `refs::write_ref_pointer_candidate` directly
/// (a `#[cfg(test)]`-only re-export added alongside this test, mirroring the existing `#[cfg(test)]
/// pub(crate) use log::{...}` re-export a few lines above it in `refs.rs`, for the same reason:
/// the production write path enforces the very invariant this check exists to catch when it's
/// violated some other way). The candidate is written for `heads/other`, pointing at `heads/
/// main`'s real, unrelated `RefState` -- then promoted from its temp path to its own canonical
/// pointer location via a plain rename, so the canonical-path check (which only compares against
/// the name encoded *in this pointer*, `"heads/other"`) passes cleanly and only the name-vs-
/// RefState check can fire.
///
/// **Probed, downstream-redundant -- not the clean pass its first draft assumed.** Disabling this
/// check does not produce `Ok` with an empty report: `"heads/other"` has a pointer but no ref-log
/// of its own, and `classify_ref_state` (`refs/verify.rs`) independently classifies that shape --
/// a pointer whose payload looks like a fresh, first update with no matching log record -- as
/// `PRIKK-VERIFY-REF-DIVERGENCE` (`blocking: true`), the same code and blocking status the
/// existing "Yes"-covered pointer-missing-format-2 row already proves reaches `has_blocking_ref_
/// publication_issues`. So a defect this check alone would let through is still independently
/// caught, blocking, by the pointer/log coherence classifier one layer up -- this row is a
/// regression guard on the more specific "name differs from pointer ref" message, not a
/// demonstration of Stage 1's rule on its own. Verified by inspecting the full returned report
/// with the check disabled, not merely by confirming `verify_repository` still returns some `Err`
/// or `Ok` -- the earlier, wrong "load-bearing" read of this test came from checking only that a
/// `panic!` fired on the `Ok` arm, without looking at what the report actually contained.
#[test]
fn verify_repository_detects_ref_state_name_pointer_mismatch() -> Result<()> {
    let root = unique_temp_dir("verify-ref-state-name-mismatch");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());

    let (main_ref_state_id, _) = publish_ref_to_new_block(&layout, &mut objects, "heads/main")?;
    crate::refs::write_ref_pointer_candidate(&layout, "heads/other", main_ref_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/other"),
        layout.ref_pointer_path("heads/other"),
    )?;

    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a RefState/pointer name mismatch"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("name differs from pointer ref"),
        "expected a name-mismatch error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 6: `ensure_ref_path_shape` (`refs/verify/scan.rs`) -- a directory entry
/// under `by-id/` or `logs/` whose filename is not exactly 64 hex characters plus the expected
/// extension. Called immediately after `list_directory`, strictly before any attempt to decode the
/// entry's content (`read_pointers`/`read_logs`), so a completely bare, freshly-initialized
/// repository is enough: `by-id/`/`logs/` both exist from `RepositoryLayout::init` itself
/// (`layout.rs:206-207`), no publish needed, no Block/RefState object exists anywhere in either
/// fixture. That also means `PublicationTrustVerifier` is never invoked here at all (it only checks
/// `Block`/`RefState`, and there are none), so unlike round 5's Block/RefState-carrying checks, a
/// real adopted signer isn't needed for a clean baseline -- confirmed by inspecting the full report
/// on every probe below, not assumed from the parallel to round 3/4's already-immune fixtures.
///
/// **Probed, downstream-redundant -- provably so, not merely observed for one fixture.** Both
/// sub-cases here use undecodable garbage bytes, which is enough to prove the check rejects in
/// production (this file's own job), but disabling `ensure_ref_path_shape` with garbage content
/// doesn't reach a clean `Ok` -- decode itself fails first (`"invalid ref pointer magic"` /
/// equivalent for logs), a different check entirely. That alone would be a weak redundancy claim
/// (maybe a *decodable* wrong-shaped file behaves differently), so it was checked directly: built a
/// real, valid pointer via a normal publish, moved its real bytes verbatim to a wrong-length
/// filename, and disabled the check. Result: `"non-canonical ref pointer: short.ref"` -- the
/// canonical-path check (`scan.rs`, `path != layout.ref_pointer_path(&pointer.ref_name)`) catches
/// it instead. Confirmed identically for logs (`"non-canonical ref log: short.log"` against `path
/// != layout.ref_log_path(&name)`). **This redundancy is structural, not incidental**: `ref_pointer_
/// path`/`ref_log_path` are deterministic functions that only ever produce correctly-shaped output,
/// so no path failing the shape check can ever equal either function's result -- meaning the
/// canonical-path check necessarily also rejects anything the shape check would have, whenever
/// decode succeeds at all. `ensure_ref_path_shape` cannot be load-bearing against either canonical-
/// path check by construction, only against a decode step that would otherwise have accepted
/// malformed content -- and decode already rejects it independently, as the garbage-bytes fixtures
/// above show. Kept as a regression guard on the friendlier, more specific "invalid shape" message
/// (real diagnostic value: an operator sees *why* the file is wrong, not just that decode choked on
/// it), not as a demonstration of Stage 1's rule.
#[test]
fn verify_repository_detects_every_ref_path_shape_violation() -> Result<()> {
    let pointer_root = unique_temp_dir("verify-ref-pointer-path-shape");
    let layout = RepositoryLayout::init(pointer_root.clone())?;
    let bad_pointer = layout.refs_dir().join("by-id").join("zz.ref");
    std::fs::write(&bad_pointer, b"not a valid length or content")?;
    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a badly-shaped pointer filename"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("ref path has invalid shape"),
        "expected a ref-path-shape error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(pointer_root);

    let log_root = unique_temp_dir("verify-ref-log-path-shape");
    let layout = RepositoryLayout::init(log_root.clone())?;
    let bad_log = layout.refs_dir().join("logs").join("zz.log");
    std::fs::write(&bad_log, b"not a valid length or content")?;
    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a badly-shaped log filename"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("ref path has invalid shape"),
        "expected a ref-path-shape error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(log_root);
    Ok(())
}
