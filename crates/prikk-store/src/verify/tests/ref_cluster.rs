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
//!
//! **Duplicate pointer identity and duplicate ref-log identity (`refs/verify/scan.rs`'s
//! `read_pointers`/`read_logs`) are ruled provably unreachable, kept, and permanently untested**
//! (DC-95-stage-1-round-6-review-v1 §3). Both fire only on a second `insert` into a name-keyed map,
//! reachable per entry only after that entry's own canonical-path check passes -- and since `layout
//! ::ref_pointer_path`/`ref_log_path` are deterministic functions of the ref name, two distinct
//! directory entries can only both pass that check under the same name via a genuine SHA-256
//! collision, or by being the literal same file (impossible in one `list_directory` pass). **This
//! is a fact about the current canonical-path scheme, not a stated format invariant** -- ruled
//! unreachable *today*, not unreachable *by design*; changing that scheme (a non-hashed component,
//! a shortened key, a namespace prefix) would make both reachable again with nothing left to catch
//! them. Kept for that reason, mirroring DC-92's topological-cycle exception (`block_state/
//! tests.rs`'s own `two_blocks_naming_each_other_as_state_parent_are_caught_as_a_cycle` doc
//! comment, the precedent this ruling cites) -- unlike that check, no lower-level function here
//! bypasses the filesystem/canonical-path constraint the way `verify_blocks_topological` bypasses
//! content-addressing, so there is no unit-level substitute demonstration to build either.
//!
//! Round 7 (Group C: direct `RefState`/`RefUpdate` construction, bypassing `RefStore::publish`)
//! adds two more: `verify_update`'s RefState/RefUpdate coherence check (load-bearing, but only once
//! its fixture gained a real matching pointer -- a log-only fixture hits `classify_ref_state`'s own
//! "pointer missing" arm regardless of the check under test, the same shape of confound round 5's
//! copy-vs-move lesson was about) and "RefState is unsigned" (downstream-redundant with publication
//! trust: an object with no signatures at all trivially has none matching a trusted key either).
//!
//! Round 9 closes four of the cluster's remaining five rows, all reachable without a format-1 flip:
//! incomplete log tail without pointer lead, format-2's `created_at == 0` check, ref-log chain/
//! sequence divergence, and `classify_ref_state`'s catch-all fallback -- all four load-bearing, all
//! four found a fixture-construction lesson before or during their probe (a fake-signed fixture
//! confounding the trust baseline, same as round 5's; a left-behind pointer making a chain-sequence
//! probe redundant with an unrelated divergence check instead of clean, discovered the same way
//! round 5's copy-vs-move lesson was). `LEGACY-LOG-LEADS` (format-1) was the one row left in this
//! cluster, deferred to the round that took on the format-1-flip technique -- and never reached
//! one, since RFC 103 retired format-1 first. The row and the test that proved it are deleted;
//! `verify.rs`'s own inventory table is the current-state record of that change, this paragraph is
//! left as history.

use prikk_error::Result;
use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType, RefKind,
    RefStatePayload, RefUpdatePayload,
};

use super::{assert_ref_failed, assert_stage_failed};
use crate::maintainer_signing::MaintainerSigner;
use crate::test_support::{
    sample_object_id, signed_empty_block_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::{
    Ed25519MaintainerSigner, FileObjectStore, ObjectWriter, RefPublication, RefStore,
    RepositoryLayout, VerificationStage, add_trusted_maintainer, derive_next_state_root,
    maintainer_signature, verify_repository,
};

fn trusted_signer(seed_label: &str, byte: u8) -> Result<Ed25519MaintainerSigner> {
    let signer = Ed25519MaintainerSigner::from_seed(seed_label, &[byte; 32])?;
    Ok(signer)
}

fn adopt(layout: &RepositoryLayout, signer: &Ed25519MaintainerSigner) -> Result<()> {
    let public_key_hex: String = signer
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    add_trusted_maintainer(layout, signer.key_id(), &public_key_hex)?;
    Ok(())
}

/// Writes a real, trusted, empty `Root` `Block`, signed by `signer` -- the target every Group-C
/// fixture below needs, for a baseline that can genuinely verify clean. `distinguishing_snapshot`
/// names a real, already-persisted `Blob` to reference, needed whenever a test writes more than one
/// otherwise-identical Root block: two empty Root blocks with no other varying field are the same
/// content-addressed object (the exact confound round 3's Block-trust test hit first), so a second
/// "distinct" target needs a real distinguishing field, not just a second call to this function.
fn write_trusted_block(
    objects: &mut FileObjectStore,
    signer: &Ed25519MaintainerSigner,
    distinguishing_snapshot: Option<ObjectId>,
) -> Result<ObjectId> {
    let payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: Vec::new(),
        state_merkle_root: derive_next_state_root(objects, None, &[])?,
        snapshot_blob_ref: distinguishing_snapshot,
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::Block, 2, payload.to_canonical_bytes()?);
    let id = envelope.object_id();
    envelope.add_signature(maintainer_signature(signer, ObjectType::Block, id)?)?;
    objects.write_object(&envelope)
}

/// Writes a real, trusted `Blob`, for use as a distinguishing `snapshot_blob_ref`.
fn write_trusted_blob(
    objects: &mut FileObjectStore,
    signer: &Ed25519MaintainerSigner,
    content: &[u8],
) -> Result<ObjectId> {
    let payload = prikk_object::BlobPayload::new(prikk_object::BlobKind::Text, content.to_vec());
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, payload.to_canonical_bytes()?);
    let id = envelope.object_id();
    envelope.add_signature(maintainer_signature(signer, ObjectType::Blob, id)?)?;
    objects.write_object(&envelope)
}

/// Writes a real, trusted `RefState` object directly (not published), signed by `signer` unless
/// `sign` is false. Returns its id.
fn write_trusted_ref_state(
    objects: &mut FileObjectStore,
    ref_name: &str,
    target_object_id: ObjectId,
    update_seq: u64,
    previous_ref_state_id: Option<ObjectId>,
    signer: &Ed25519MaintainerSigner,
    sign: bool,
) -> Result<ObjectId> {
    let payload = RefStatePayload {
        ref_name: ref_name.to_string(),
        kind: RefKind::Branch,
        target_object_id,
        update_seq,
        previous_ref_state_id,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::RefState, 1, payload.to_canonical_bytes()?);
    let id = envelope.object_id();
    if sign {
        envelope.add_signature(maintainer_signature(signer, ObjectType::RefState, id)?)?;
    }
    objects.write_object(&envelope)
}

/// Publishes `ref_name` pointing at a freshly written, real `Block`, via the normal `RefStore::
/// publish` path -- exactly what every check in this file assumes has already happened cleanly,
/// so the test's own fixture-construction step is the one deliberate irregularity, isolated.
///
/// **Fake-signed (`test_support::maintainer_signature()`), not adopted by any trust policy.** Fine
/// for a *committed test*'s own construction (production code rejects the fixture's real defect
/// before trust is ever consulted), but reusing this for a disable-and-restore *probe* has now
/// confounded four separate rounds (1, 2, 5, 9 -- `DC-95-stage-1-round-9-review-v1.md` §3): every
/// probe built on it carries a permanent `PRIKK-TRUST-POLICY-INVALID`, so `Ok` from a disabled
/// check is never distinguishable from a genuinely clean pass. **Never reach for this in a new
/// probe.** Use `write_trusted_block`/`write_trusted_ref_state` (real, adopted signer) instead --
/// they have covered every construction need this helper has since round 6. The ugly name is
/// deliberate: a name that reads as ordinary is what let the mistake recur three times after the
/// first.
fn publish_ref_to_new_block_fake_signed_confounds_probes(
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

    let (_, block_id) =
        publish_ref_to_new_block_fake_signed_confounds_probes(&layout, &mut objects, "heads/main")?;
    std::fs::remove_file(layout.object_path(ObjectType::Block, block_id))?;

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "targets missing block");
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

    publish_ref_to_new_block_fake_signed_confounds_probes(&layout, &mut objects, "heads/aux")?;
    let canonical = layout.ref_pointer_path("heads/aux");
    let misplaced = layout
        .refs_dir()
        .join("by-id")
        .join(format!("{}.ref", sample_object_id("misplaced-pointer")));
    std::fs::rename(&canonical, &misplaced)?;

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "non-canonical ref pointer");
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

    let (main_ref_state_id, _) =
        publish_ref_to_new_block_fake_signed_confounds_probes(&layout, &mut objects, "heads/main")?;
    crate::refs::write_ref_pointer_candidate(&layout, "heads/other", main_ref_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/other"),
        layout.ref_pointer_path("heads/other"),
    )?;

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "name differs from pointer ref");
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
    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "ref path has invalid shape");
    let _ = std::fs::remove_dir_all(pointer_root);

    let log_root = unique_temp_dir("verify-ref-log-path-shape");
    let layout = RepositoryLayout::init(log_root.clone())?;
    let bad_log = layout.refs_dir().join("logs").join("zz.log");
    std::fs::write(&bad_log, b"not a valid length or content")?;
    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "ref path has invalid shape");
    let _ = std::fs::remove_dir_all(log_root);
    Ok(())
}

/// DC-95 Stage 1, round 7: `verify_update`'s RefState/RefUpdate coherence check (`refs/verify/
/// scan.rs`) -- a ref-log record whose fields disagree with the `RefState` object it names as its
/// own new state. `RefStore::publish`'s own `validate_coherent_publication` cross-checks these two
/// exact objects before either is ever written, so this fixture -- like round 5's RefState/pointer
/// name mismatch -- is unreachable through the public API and must bypass it: a real `RefState`
/// object written directly (`write_trusted_ref_state`, not `publish`), and a `RefUpdate` log record
/// appended directly via the `#[cfg(test)]`-only `refs::append_log_record_for_signature_test`
/// (already re-exported in `refs.rs` for `signature_contract_tests`, reused here rather than adding
/// a new one), whose `new_target_object_id` deliberately differs from the `RefState`'s own
/// `target_object_id` -- every other field agrees, isolating this one disagreement. Built with a
/// real, adopted signer behind both the `Block` and the `RefState` from the start (the corrected
/// methodology, not a retrofit).
///
/// **A matching pointer is required, not optional -- found by omitting it first.** `verify_update`
/// runs from `read_logs`' own per-record loop (`validate_log`), reached for every log file
/// regardless of whether a matching pointer exists, so a first draft omitted the pointer entirely.
/// Probing that draft (check disabled) didn't reach a clean `Ok`: with no pointer at all, `classify_
/// ref_state`'s own `(None, Some(log))` arm fires unconditionally for this ref, reporting `PRIKK-
/// VERIFY-REF-DIVERGENCE` ("pointer is missing while committed log history exists") regardless of
/// `verify_update`'s own state -- a confound in the fixture, not evidence about the check. Fixed by
/// adding a real pointer matching `ref_state_id` at its own canonical location (`write_ref_pointer_
/// candidate` + rename, round 5's own technique), so `classify_ref_state` reaches its clean arm
/// once `verify_update` no longer objects, and only `verify_update`'s own field comparison is what
/// the probe is actually measuring.
/// **Probed, load-bearing, confirmed** (with the pointer present): disabling `verify_update`'s
/// field-agreement check lets `verify_repository` return `Ok` with every issue vector empty.
#[test]
fn verify_repository_detects_ref_update_ref_state_mismatch() -> Result<()> {
    let root = unique_temp_dir("verify-ref-update-mismatch");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let signer = trusted_signer("verify-ref-update-mismatch", 0x11)?;
    adopt(&layout, &signer)?;

    let target_block = write_trusted_block(&mut objects, &signer, None)?;
    let distinguishing_blob = write_trusted_blob(&mut objects, &signer, b"wrong-target-marker")?;
    let wrong_target_block = write_trusted_block(&mut objects, &signer, Some(distinguishing_blob))?;
    let ref_state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        1,
        None,
        &signer,
        true,
    )?;

    let update_payload = RefUpdatePayload {
        ref_name: "heads/main".to_string(),
        old_ref_state_id: None,
        new_ref_state_id: ref_state_id,
        new_target_object_id: wrong_target_block,
        update_seq: 1,
        created_at: 0,
        author_key_id: signer.key_id().to_string(),
    };
    let mut update_envelope = ObjectEnvelope::unsigned(
        ObjectType::RefUpdate,
        1,
        update_payload.to_canonical_bytes()?,
    );
    let update_id = update_envelope.object_id();
    update_envelope.add_signature(maintainer_signature(
        &signer,
        ObjectType::RefUpdate,
        update_id,
    )?)?;
    crate::refs::append_log_record_for_signature_test(&layout, "heads/main", &update_envelope)?;
    // A pointer matching ref_state_id, at its own canonical location -- otherwise, with no
    // pointer at all, classify_ref_state's own "pointer missing while log exists" arm fires
    // regardless of verify_update's state, confounding the probe below (found by running it).
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", ref_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/main"),
        layout.ref_pointer_path("heads/main"),
    )?;

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "RefState disagrees with RefUpdate");
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 7: the "RefState is unsigned" check (`verified_ref_state_payload`,
/// `refs/verify/scan.rs`) -- a `RefState` object with zero signatures. `ObjectEnvelope::validate_
/// strict` (checked by `store.write_object` on every write) does not require any signature to be
/// present at all: an empty `signatures` vec makes every one of its shape/duplicate/order checks
/// vacuously true, so `write_trusted_ref_state(..., sign: false)` writes successfully, and this
/// defect is invisible until read time. Reached via a real, matching pointer (same construction and
/// same reason as the mismatch test above -- `read_pointers` calls this same `verified_ref_state_
/// payload` function too, so either call site would do, and a log-only fixture without one would
/// hit the same "pointer missing" confound).
///
/// **Probed, downstream-redundant -- not load-bearing.** Disabling the `signatures.is_empty()`
/// check does not produce a clean `Ok`: `PublicationTrustVerifier` still classifies the RefState as
/// untrusted, `PRIKK-TRUST-PUBLICATION-UNTRUSTED` (`blocking` via `has_publication_trust_issues`),
/// since an object with zero signatures trivially has no signature matching any trusted key either.
/// This is structural, not incidental to this fixture: any `Block`/`RefState` with no signatures at
/// all fails trust verification for the same reason, regardless of what this hard check does.
/// **Probed, downstream-redundant, confirmed**: disabling the check still returns `Ok`, but with
/// `publication_trust_issues` non-empty and blocking. Kept as a regression guard on the more
/// specific "is unsigned" message (real diagnostic value distinguishing "never signed" from "signed
/// by an untrusted key"), not as a Stage-1-rule demonstration.
#[test]
fn verify_repository_detects_unsigned_ref_state() -> Result<()> {
    let root = unique_temp_dir("verify-unsigned-ref-state");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let signer = trusted_signer("verify-unsigned-ref-state", 0x12)?;
    adopt(&layout, &signer)?;

    let target_block = write_trusted_block(&mut objects, &signer, None)?;
    let ref_state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        1,
        None,
        &signer,
        false,
    )?;

    let update_payload = RefUpdatePayload {
        ref_name: "heads/main".to_string(),
        old_ref_state_id: None,
        new_ref_state_id: ref_state_id,
        new_target_object_id: target_block,
        update_seq: 1,
        created_at: 0,
        author_key_id: signer.key_id().to_string(),
    };
    let mut update_envelope = ObjectEnvelope::unsigned(
        ObjectType::RefUpdate,
        1,
        update_payload.to_canonical_bytes()?,
    );
    let update_id = update_envelope.object_id();
    update_envelope.add_signature(maintainer_signature(
        &signer,
        ObjectType::RefUpdate,
        update_id,
    )?)?;
    crate::refs::append_log_record_for_signature_test(&layout, "heads/main", &update_envelope)?;
    // A matching pointer, for the same reason the RefUpdate/RefState mismatch test above needs
    // one: without it, disabling the check under test would still hit classify_ref_state's own
    // "pointer missing while log exists" arm, confounding the probe.
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", ref_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/main"),
        layout.ref_pointer_path("heads/main"),
    )?;

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "is unsigned");
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 9: "incomplete log tail without pointer lead" (`refs/verify.rs`'s
/// `classify_ref_state`, the very first match arm) -- a log whose tip still matches the pointer
/// (nothing to repair, nothing missing) but which has undecodable trailing bytes after its last
/// complete record. Built by publishing normally, then appending fewer than `REF_LOG_HEADER_LEN`
/// (50; `refs/log.rs`) raw bytes directly to the log file -- `decode_log_records` (`refs/log.rs:173-
/// 218`) treats anything shorter than one full header as a trailing partial, not an error, so this
/// doesn't corrupt the prior valid record or require constructing a plausible-looking next header.
///
/// **Built with a real, adopted signer from the start of this probe, not the fixture's first
/// draft.** The first draft reused round 5's fake-signed helper (now renamed to make the hazard
/// visible: `publish_ref_to_new_block_fake_signed_confounds_probes`) for speed;
/// probing it produced `Ok` with `publication_trust_issues` non-empty (`PRIKK-TRUST-POLICY-INVALID`
/// baseline noise) -- not a clean report, and `has_publication_trust_issues()` would still be `true`,
/// so the probe couldn't distinguish load-bearing from redundant. Rebuilt with `write_trusted_block`/
/// `write_trusted_ref_state` and a real pointer/log pair instead (the same fix applied to the
/// `created_at` test immediately below, before its own probe could hit the identical confound).
/// **Probed, load-bearing, confirmed**: disabling the `trailing_partial_bytes != 0` check inside
/// `classify_ref_state`'s matching arm lets `verify_repository` return `Ok` with every issue vector
/// empty.
#[test]
fn verify_repository_detects_incomplete_log_tail_without_pointer_lead() -> Result<()> {
    let root = unique_temp_dir("verify-incomplete-log-tail");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let signer = trusted_signer("verify-incomplete-log-tail", 0x15)?;
    adopt(&layout, &signer)?;

    let target_block = write_trusted_block(&mut objects, &signer, None)?;
    let state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        1,
        None,
        &signer,
        true,
    )?;
    let update = build_signed_ref_update("heads/main", None, state_id, target_block, 1, &signer)?;
    crate::refs::append_log_record_for_signature_test(&layout, "heads/main", &update)?;
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/main"),
        layout.ref_pointer_path("heads/main"),
    )?;

    let mut log_file = std::fs::OpenOptions::new()
        .append(true)
        .open(layout.ref_log_path("heads/main"))?;
    std::io::Write::write_all(&mut log_file, &[0xAB, 0xCD, 0xEF])?;
    drop(log_file);

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "incomplete log tail without a pointer lead");
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 9: `verify_refs`'s format-2 `created_at == 0` check (`verify.rs:46-52`) --
/// tightens existing coverage (`refs/tests/publication_recovery/compatibility.rs`'s `format2_
/// legacy_timestamp_is_not_normalized_and_blocks_mutation`, which already builds exactly this
/// fixture but only asserts `.is_err()`) to a specific-message assertion, matching this file's own
/// bar. Reuses that test's exact construction: `RefStore::publish` a normal ref, then overwrite the
/// log record with schema-1 `created_at: 7` -- `append_log_record` unconditionally rejects a
/// nonzero `created_at` at schema 1 (`refs/log.rs`'s `validate_log_record`), so building this
/// fixture at all requires bypassing it via the `#[cfg(test)]`-only `encode_log_record_for_test`
/// and a raw `std::fs::write`, exactly as the existing test does.
/// **Probed, load-bearing, confirmed**: disabling this specific check (leaving the schema-1-at-
/// write-time rejection in `append_log_record` untouched, since that's a different call site) lets
/// `verify_repository` return `Ok` with every issue vector empty.
#[test]
fn verify_repository_detects_nonzero_created_at_under_format2() -> Result<()> {
    let root = unique_temp_dir("verify-nonzero-created-at");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let signer = trusted_signer("verify-nonzero-created-at", 0x16)?;
    adopt(&layout, &signer)?;

    let target_block = write_trusted_block(&mut objects, &signer, None)?;
    let ref_state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        1,
        None,
        &signer,
        true,
    )?;
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", ref_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/main"),
        layout.ref_pointer_path("heads/main"),
    )?;

    let update_payload = RefUpdatePayload {
        ref_name: "heads/main".to_string(),
        old_ref_state_id: None,
        new_ref_state_id: ref_state_id,
        new_target_object_id: target_block,
        update_seq: 1,
        created_at: 7,
        author_key_id: signer.key_id().to_string(),
    };
    let mut update_envelope = ObjectEnvelope::unsigned(
        ObjectType::RefUpdate,
        1,
        update_payload.to_canonical_bytes()?,
    );
    let update_id = update_envelope.object_id();
    update_envelope.add_signature(maintainer_signature(
        &signer,
        ObjectType::RefUpdate,
        update_id,
    )?)?;
    std::fs::write(
        layout.ref_log_path("heads/main"),
        crate::refs::encode_log_record_for_test(&update_envelope)?,
    )?;

    let report = verify_repository(&layout)?;
    // DC-95 Stage 2 Level 2 handoff §7 Q4, ruled: this stays a whole-set `Refs`-stage failure, not
    // an item-contained one -- a stale timestamp anywhere is evidence the format-1-to-2 migration
    // did not complete or did not cover everything, a claim about the whole repository's format-2
    // assertion, deliberately not contained to the one ref that happens to carry it.
    assert_stage_failed(
        &report,
        VerificationStage::Refs,
        "format-2 RefUpdate requires created_at == 0",
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 9: ref-log chain/sequence divergence (`refs/verify/scan.rs`'s
/// `validate_log`, the "ref-log chain or sequence diverges" check) -- tightens existing coverage
/// (`refs/tests/publication_recovery.rs`'s `ref_log_sequence_gap_fails_closed`, which already
/// builds exactly this fixture -- a second log record whose `update_seq` jumps from 1 to 3 -- but
/// only asserts `.is_err()` and `!doctor.is_healthy()`) to a specific-message assertion.
///
/// **The pointer must be advanced to the gap record's id too, found by probing the first draft
/// without it.** The existing test's own construction leaves the pointer at the first record (it
/// never needed to move it, since `.is_err()` doesn't care which check fired). Reusing that
/// construction unchanged and disabling the chain check didn't reach a clean `Ok`: with the log now
/// two records ahead of a pointer that never moved, `classify_ref_state`'s own "log leads pointer"
/// arm (`PRIKK-VERIFY-REF-DIVERGENCE`, blocking) caught the resulting mismatch instead -- real
/// evidence about a *different* defect (pointer left behind), not about this check. Fixed by also
/// repointing to the gap record's id, matching the log's new tip, so the only remaining way to
/// notice the broken internal sequence is the chain check itself.
/// **Probed, load-bearing, confirmed** (with the pointer advanced): disabling this check lets
/// `verify_repository` return `Ok` with every issue vector empty.
#[test]
fn verify_repository_detects_ref_log_sequence_gap() -> Result<()> {
    let root = unique_temp_dir("verify-ref-log-sequence-gap");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let signer = trusted_signer("verify-ref-log-sequence-gap", 0x13)?;
    adopt(&layout, &signer)?;

    let target_block = write_trusted_block(&mut objects, &signer, None)?;
    let first_state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        1,
        None,
        &signer,
        true,
    )?;
    let first_update =
        build_signed_ref_update("heads/main", None, first_state_id, target_block, 1, &signer)?;
    crate::refs::append_log_record_for_signature_test(&layout, "heads/main", &first_update)?;
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", first_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/main"),
        layout.ref_pointer_path("heads/main"),
    )?;

    // Second RefState/RefUpdate pair, self-consistent with each other, but the update_seq jumps
    // from 1 to 3 -- a gap validate_log's own chain check exists to catch.
    let gap_state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        3,
        Some(first_state_id),
        &signer,
        true,
    )?;
    let gap_update = build_signed_ref_update(
        "heads/main",
        Some(first_state_id),
        gap_state_id,
        target_block,
        3,
        &signer,
    )?;
    crate::refs::append_log_record_for_signature_test(&layout, "heads/main", &gap_update)?;
    // Advance the pointer to match the log's new (chain-broken) tip too -- otherwise, with the
    // chain check disabled, classify_ref_state's own "log leads pointer" divergence arm catches
    // the resulting pointer/log mismatch instead, which is a downstream-redundant finding about a
    // *different* defect (pointer left behind), not evidence about this check. With the pointer
    // kept in step, the only thing left to notice the broken sequence is the chain check itself.
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", gap_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/main"),
        layout.ref_pointer_path("heads/main"),
    )?;

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "ref-log chain or sequence diverges");
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 9: `classify_ref_state`'s catch-all fallback (`verify.rs:161-163`,
/// `"unexplained pointer/log divergence for ref {ref_name}"`) -- reached only when a pointer and a
/// log both individually decode and both individually satisfy `verify_update`'s own per-record
/// coherence check, but the *pair* matches none of `classify_ref_state`'s other four explicit
/// arms. Built from two entirely independent, individually self-consistent RefState/RefUpdate
/// pairs for the same ref name: a log recording a genuine first publication (`update_seq: 1`,
/// `old_ref_state_id: None`) for RefState `Y`, and a pointer -- placed directly, bypassing
/// `RefStore::publish`'s own coherence enforcement, the same way round 5's name-mismatch test and
/// round 7's coherence test both had to -- naming a completely unrelated RefState `X` (`update_seq:
/// 99`, its own `previous_ref_state_id: None`). Neither the pointer-leads-log arm (`X`'s `previous_
/// ref_state_id` doesn't name `Y`) nor the log-leads-pointer arm (`log.previous_tip` is `None`, not
/// `Some(X)`) nor the pointer-missing arm (a pointer *is* present) applies -- confirmed by tracing
/// each guard against this fixture's exact fields before writing it, not by trial and error.
/// **Probed, load-bearing, confirmed**: disabling the catch-all's own `Err` (routing the fallback to
/// `Ok(())` instead) lets `verify_repository` return `Ok` with every issue vector empty.
#[test]
fn verify_repository_detects_unexplained_pointer_log_divergence() -> Result<()> {
    let root = unique_temp_dir("verify-unexplained-divergence");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let signer = trusted_signer("verify-unexplained-divergence", 0x14)?;
    adopt(&layout, &signer)?;

    let target_block = write_trusted_block(&mut objects, &signer, None)?;

    // Y: log records a genuine, self-consistent first publication.
    let y_state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        1,
        None,
        &signer,
        true,
    )?;
    let y_update =
        build_signed_ref_update("heads/main", None, y_state_id, target_block, 1, &signer)?;
    crate::refs::append_log_record_for_signature_test(&layout, "heads/main", &y_update)?;

    // X: an unrelated RefState, pointed to directly -- no relationship to Y at all.
    let x_state_id = write_trusted_ref_state(
        &mut objects,
        "heads/main",
        target_block,
        99,
        None,
        &signer,
        true,
    )?;
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", x_state_id)?;
    std::fs::rename(
        layout.ref_tmp_path("heads/main"),
        layout.ref_pointer_path("heads/main"),
    )?;

    let report = verify_repository(&layout)?;
    assert_ref_failed(&report, "unexplained pointer/log divergence");
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

fn build_signed_ref_update(
    ref_name: &str,
    old_ref_state_id: Option<ObjectId>,
    new_ref_state_id: ObjectId,
    new_target_object_id: ObjectId,
    update_seq: u64,
    signer: &Ed25519MaintainerSigner,
) -> Result<ObjectEnvelope> {
    let payload = RefUpdatePayload {
        ref_name: ref_name.to_string(),
        old_ref_state_id,
        new_ref_state_id,
        new_target_object_id,
        update_seq,
        created_at: 0,
        author_key_id: signer.key_id().to_string(),
    };
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::RefUpdate, 1, payload.to_canonical_bytes()?);
    let id = envelope.object_id();
    envelope.add_signature(maintainer_signature(signer, ObjectType::RefUpdate, id)?)?;
    Ok(envelope)
}
