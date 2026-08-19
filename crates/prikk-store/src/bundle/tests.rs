//! Bundle export/import tests.

mod proptest_decode_bundle;

use prikk_object::{
    BlockKind, CanonicalEncode, CreateFile, NodeId, ObjectEnvelope, ObjectId, ObjectType,
    Operation, OperationKind, PatchPayload, PatchPurpose,
};

use crate::author_key_index::{
    AuthorKeyEntry, force_conflicting_author_key_entry_for_test, lookup_author_key_entries,
    record_author_key_material, verify_author_signature,
};
use crate::author_signing::{AuthorSigner, author_signature};
use crate::bundle::{
    BundleImportOptions, DEFAULT_BUNDLE_MAX_OBJECT_COUNT, decode_bundle, encode_bundle,
    encode_bundle_v1_for_test, export_bundle, import_bundle,
};
use crate::layout::LockableContainer;
use crate::lock::{ActiveLock, acquire_container_locks};
use crate::received::read_received_pointer;
use crate::test_support::{
    signed_block, signed_patch_blob_envelope, signed_patch_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::{
    Ed25519AuthorSigner, Ed25519MaintainerSigner, FileObjectStore, MaintainerSigner, ObjectReader,
    ObjectWriter, RefPublication, RefStore, RepositoryLayout,
};

/// Seal a two-block `heads/main` (a Root block plus a Normal child referencing one Patch, whose
/// `CreateFile` operation itself references one Blob) into `layout`, returning the tip Block id —
/// enough to exercise a genuinely multi-block, genesis-complete export rather than a single trivial
/// object, and to exercise blob discovery through a Patch's own operations, not just a Block's
/// `snapshot_blob_ref`.
fn seal_two_block_history(
    layout: &RepositoryLayout,
) -> prikk_error::Result<prikk_object::ObjectId> {
    let mut object_store = FileObjectStore::new(layout.clone());
    object_store.write_object(&signed_patch_blob_envelope())?;
    let patch = signed_patch_envelope();
    let patch_id = object_store.write_object(&patch)?;

    let root_block = signed_block(BlockKind::Root, Vec::new(), Vec::new(), None);
    let root_block_id = object_store.write_object(&root_block)?;

    let child_block = signed_block(BlockKind::Normal, vec![root_block_id], vec![patch_id], None);
    let child_block_id = object_store.write_object(&child_block)?;

    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope("heads/main", None, child_block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update =
        signed_ref_update_envelope("heads/main", None, ref_state_id, child_block_id, 1);
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state,
        ref_update,
    })?;
    Ok(child_block_id)
}

/// DC-53 Stage 2: a fixed-seed AUTHOR signer, distinct across callers via `discriminant` so tests
/// signing more than one Patch under this helper get distinct `key_id`s and object ids.
fn transport_test_signer(discriminant: u8) -> prikk_error::Result<Ed25519AuthorSigner> {
    Ed25519AuthorSigner::from_seed(
        format!("dc53-stage2-transport-{discriminant}"),
        &[discriminant; 32],
    )
}

/// Seal a two-block `heads/main` whose Patch carries a real AUTHOR signature from `signer`, rather
/// than `seal_two_block_history`'s fixed structural fixture -- DC-53 Stage 2's transport tests need
/// a signature `record_author_key_material`/`verify_author_signature` can actually be asked about.
/// If `record_locally` is true, `signer`'s key material is recorded on `layout` before this returns
/// (real authoring's own order, `node_authoring.rs:589`); if false, the Patch is signed but no local
/// material exists for it yet -- vector 7's starting state.
fn seal_two_block_history_with_author(
    layout: &RepositoryLayout,
    signer: &Ed25519AuthorSigner,
    record_locally: bool,
) -> prikk_error::Result<ObjectId> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let blob = signed_patch_blob_envelope();
    object_store.write_object(&blob)?;

    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "transport.txt".to_string(),
                node_id: NodeId::from_bytes([0x54; 32]),
                blob_id: blob.object_id(),
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut patch = ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload.to_canonical_bytes()?);
    let patch_object_id = patch.object_id();
    let signature = author_signature(signer, patch_object_id)?;
    patch.add_signature(signature)?;
    let patch_id = object_store.write_object(&patch)?;

    if record_locally {
        let active_lock = ActiveLock::acquire(layout)?;
        record_author_key_material(
            layout,
            signer.key_id(),
            signer.public_key_bytes(),
            &active_lock,
        )?;
    }

    let root_block = signed_block(BlockKind::Root, Vec::new(), Vec::new(), None);
    let root_block_id = object_store.write_object(&root_block)?;
    let child_block = signed_block(BlockKind::Normal, vec![root_block_id], vec![patch_id], None);
    let child_block_id = object_store.write_object(&child_block)?;

    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope("heads/main", None, child_block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update =
        signed_ref_update_envelope("heads/main", None, ref_state_id, child_block_id, 1);
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state,
        ref_update,
    })?;
    Ok(child_block_id)
}

/// Find the imported Patch in `target`'s object store by scanning for the one carrying
/// `transport.txt` -- `seal_two_block_history_with_author`'s own fixture path, distinct from
/// `seal_two_block_history`'s `a.txt`.
fn find_imported_transport_patch(target: &RepositoryLayout) -> prikk_error::Result<ObjectEnvelope> {
    // The Patch's object id is derived from its canonical payload, which the sender and receiver
    // both compute identically -- recompute it here rather than threading it through `import_bundle`'s
    // own report, which deliberately reports only counts (DC-53 Stage 2, D6/D7's own "no new
    // verification path" -- this helper is test-only introspection, not part of the transport
    // contract).
    let target_store = FileObjectStore::new(target.clone());
    let ref_state_id = read_received_pointer(target, "remotes/heads/main")?
        .ok_or_else(|| {
            prikk_error::PrikkError::Integrity("received ref state missing".to_string())
        })?
        .ref_state_id;
    let ref_state_envelope = target_store
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| {
            prikk_error::PrikkError::Integrity("received RefState missing".to_string())
        })?;
    let ref_state_payload = prikk_object::RefStatePayload::decode_canonical(
        &ref_state_envelope.canonical_payload,
        ref_state_envelope.schema_version,
    )?;
    let block_envelope = target_store
        .read_typed(ref_state_payload.target_object_id, ObjectType::Block)?
        .ok_or_else(|| prikk_error::PrikkError::Integrity("received Block missing".to_string()))?;
    let block_payload =
        prikk_object::BlockPayload::decode_canonical(&block_envelope.canonical_payload)?;
    for patch_id in &block_payload.patch_ids {
        if let Some(envelope) = target_store.read_typed(*patch_id, ObjectType::Patch)? {
            return Ok(envelope);
        }
    }
    Err(prikk_error::PrikkError::Integrity(
        "no Patch found in received Block".to_string(),
    ))
}

#[test]
fn export_of_missing_ref_fails() {
    let root = unique_temp_dir("bundle-export-missing-ref");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(export_bundle(&layout, "heads/main").is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn import_of_malformed_bytes_fails_closed() {
    let root = unique_temp_dir("bundle-import-malformed");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let options = BundleImportOptions::default_limits();
        assert!(import_bundle(&layout, b"not a bundle", &options).is_err());
        assert!(import_bundle(&layout, b"PBNDL001", &options).is_err());
        assert!(import_bundle(&layout, &[], &options).is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn export_then_import_carries_the_full_genesis_complete_closure() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-export-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    let child_block_id = seal_two_block_history(&source)?;

    let (report, bytes) = export_bundle(&source, "heads/main")?;
    assert_eq!(report.ref_name, "heads/main");
    assert_eq!(report.tip_block_id, child_block_id);
    // RefState + 2 Blocks + 1 Patch + 1 Blob (the Patch's CreateFile references it) = 5 objects;
    // no Attestation in this fixture.
    assert_eq!(report.object_count, 5);

    let target_root = unique_temp_dir("bundle-import-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let import_report = import_bundle(&target, &bytes, &BundleImportOptions::default_limits())?;
    assert_eq!(import_report.ref_name, "remotes/heads/main");
    assert_eq!(import_report.object_count, 5);
    assert_eq!(import_report.written_object_count, 5);

    let pointer = read_received_pointer(&target, "remotes/heads/main")?;
    assert!(pointer.is_some());
    if let Some(pointer) = pointer {
        assert_eq!(pointer.ref_state_id, import_report.ref_state_id);
    }

    let target_objects = FileObjectStore::new(target.clone());
    assert!(
        target_objects
            .read_typed(child_block_id, ObjectType::Block)?
            .is_some()
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// RFC 102 Stage 6 Step 2, design-v1.md §15.8: `import_bundle` acquires the `ReceivedIndex`
/// container lock -- the gap the received-index concurrency investigation surfaced in the first
/// place (`FINDINGS.md`), closed here. Proven the same way as the ref/trust equivalents: hold the
/// lock externally, observe the refusal, release, observe success. Object writes must not have
/// happened either -- the lock is acquired after them, so a refused import still leaves the target
/// repository's received-index untouched, but confirming the objects themselves aren't silently
/// re-imported on retry is the property this test also checks.
#[test]
fn import_refuses_while_received_index_lock_is_externally_held() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-lock-conflict-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let target_root = unique_temp_dir("bundle-lock-conflict-target");
    let target = RepositoryLayout::init(target_root.clone())?;

    let held = acquire_container_locks(&target, &[LockableContainer::ReceivedIndex])?;
    assert!(import_bundle(&target, &bytes, &BundleImportOptions::default_limits()).is_err());
    assert!(read_received_pointer(&target, "remotes/heads/main")?.is_none());
    drop(held);

    let report = import_bundle(&target, &bytes, &BundleImportOptions::default_limits())?;
    assert_eq!(report.ref_name, "remotes/heads/main");
    assert!(read_received_pointer(&target, "remotes/heads/main")?.is_some());

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

#[test]
fn import_never_writes_a_local_ref_pointer() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-negctrl-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let target_root = unique_temp_dir("bundle-negctrl-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let ref_store = RefStore::new(target.clone());
    let before = ref_store.list_ref_pointers()?;
    assert!(before.is_empty());

    import_bundle(&target, &bytes, &BundleImportOptions::default_limits())?;

    let after = RefStore::new(target.clone()).list_ref_pointers()?;
    assert_eq!(
        before, after,
        "bundle import must never advance or create a local heads/*-or-tags/* ref pointer"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

#[test]
fn reimporting_the_same_bundle_is_idempotent() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-reimport-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let target_root = unique_temp_dir("bundle-reimport-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let options = BundleImportOptions::default_limits();
    let first = import_bundle(&target, &bytes, &options)?;
    assert_eq!(first.written_object_count, 5);

    let second = import_bundle(&target, &bytes, &options)?;
    assert_eq!(second.written_object_count, 0);
    assert_eq!(second.ref_state_id, first.ref_state_id);

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// DC-86 criterion 3, the negative control: a bundle whose declared object count is exactly at the
/// limit is accepted, and the identical bundle against a limit one below its actual count is refused.
/// A bound nobody has seen fire is a bound nobody knows exists.
#[test]
fn import_object_count_limit_fires_exactly_at_the_boundary() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-limit-count-boundary-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (report, bytes) = export_bundle(&source, "heads/main")?;
    assert_eq!(report.object_count, 5);

    let under_root = unique_temp_dir("bundle-limit-count-boundary-under");
    let under_target = RepositoryLayout::init(under_root.clone())?;
    let refused = import_bundle(
        &under_target,
        &bytes,
        &BundleImportOptions::default_limits().with_max_object_count(4),
    );
    assert!(
        refused.is_err(),
        "a limit one below the actual count (5) must refuse"
    );

    let at_root = unique_temp_dir("bundle-limit-count-boundary-at");
    let at_target = RepositoryLayout::init(at_root.clone())?;
    let accepted = import_bundle(
        &at_target,
        &bytes,
        &BundleImportOptions::default_limits().with_max_object_count(5),
    );
    assert!(
        accepted.is_ok(),
        "a limit exactly at the actual count (5) must accept"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(under_root);
    let _ = std::fs::remove_dir_all(at_root);
    Ok(())
}

/// The same negative control for the total-bytes bound: one byte short of the encoded bundle's own
/// length refuses; the exact length accepts.
#[test]
fn import_total_bytes_limit_fires_exactly_at_the_boundary() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-limit-bytes-boundary-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let under_root = unique_temp_dir("bundle-limit-bytes-boundary-under");
    let under_target = RepositoryLayout::init(under_root.clone())?;
    let refused = import_bundle(
        &under_target,
        &bytes,
        &BundleImportOptions::default_limits().with_max_total_bytes(bytes.len() - 1),
    );
    assert!(
        refused.is_err(),
        "a byte limit one below the bundle's own length must refuse"
    );

    let at_root = unique_temp_dir("bundle-limit-bytes-boundary-at");
    let at_target = RepositoryLayout::init(at_root.clone())?;
    let accepted = import_bundle(
        &at_target,
        &bytes,
        &BundleImportOptions::default_limits().with_max_total_bytes(bytes.len()),
    );
    assert!(
        accepted.is_ok(),
        "a byte limit exactly at the bundle's own length must accept"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(under_root);
    let _ = std::fs::remove_dir_all(at_root);
    Ok(())
}

/// DC-86 criterion 2, measured rather than asserted: a refused over-limit import leaves the target's
/// object store with exactly the same object count as before the attempt — "it returned an error"
/// does not by itself prove nothing was written.
#[test]
fn import_refused_over_the_object_count_limit_writes_nothing() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-limit-writes-nothing-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let target_root = unique_temp_dir("bundle-limit-writes-nothing-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let before = crate::verify_repository(&target)?.checked_objects;

    let refused = import_bundle(
        &target,
        &bytes,
        &BundleImportOptions::default_limits().with_max_object_count(4),
    );
    assert!(refused.is_err());

    let after = crate::verify_repository(&target)?.checked_objects;
    assert_eq!(
        before, after,
        "a refused import must leave the object store's checked-object count unchanged"
    );
    assert_eq!(before, Some(0));

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// DC-53 Stage 2, §7 vector 7: a bundle whose author-key section omits a key for a Patch it
/// contains -- material is optional per-author -- must still import, and the Patch must read
/// Unverifiable, not Sound and not a failure.
#[test]
fn dc53_stage2_vector7_omitted_material_imports_as_unverifiable() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("dc53-vector7-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    let signer = transport_test_signer(0xa1)?;
    seal_two_block_history_with_author(&source, &signer, false)?;

    let (report, bytes) = export_bundle(&source, "heads/main")?;
    assert_eq!(
        report.author_key_count, 0,
        "no local material exists for this key_id, so the section must omit it"
    );

    let target_root = unique_temp_dir("dc53-vector7-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let import_report = import_bundle(&target, &bytes, &BundleImportOptions::default_limits())?;
    assert_eq!(import_report.recorded_author_key_count, 0);

    let imported_patch = find_imported_transport_patch(&target)?;
    let outcome = verify_author_signature(&target, &imported_patch)?;
    assert_eq!(
        outcome,
        Some((signer.key_id().to_string(), false)),
        "expected Unverifiable, got {outcome:?}"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// Positive control alongside vector 7: material that *is* recorded locally transports and the
/// imported Patch reads Sound, not merely "not a failure" -- `Unverifiable` would also pass that
/// weaker bar.
#[test]
fn dc53_stage2_transported_material_imports_as_sound() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("dc53-vector-sound-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    let signer = transport_test_signer(0xa2)?;
    seal_two_block_history_with_author(&source, &signer, true)?;

    let (report, bytes) = export_bundle(&source, "heads/main")?;
    assert_eq!(report.author_key_count, 1);

    let target_root = unique_temp_dir("dc53-vector-sound-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let import_report = import_bundle(&target, &bytes, &BundleImportOptions::default_limits())?;
    assert_eq!(import_report.recorded_author_key_count, 1);

    let imported_patch = find_imported_transport_patch(&target)?;
    let outcome = verify_author_signature(&target, &imported_patch)?;
    assert_eq!(outcome, Some((signer.key_id().to_string(), true)));

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// DC-53 Stage 2, §7 vector 8: a bundle whose author-key section carries a key that does not
/// verify the Patch's signature -- the transport-layer forgery case. Import records the material
/// anyway (D7: import records, `verify` decides, no cryptographic check at the transport layer,
/// matching how objects are written without re-verifying them); `verify_author_signature` on the
/// imported Patch is what fails, reached through the transport path rather than local authoring
/// but the same underlying check as D3's third row.
#[test]
fn dc53_stage2_vector8_a_transported_key_that_does_not_verify_reads_failed()
-> prikk_error::Result<()> {
    let source_root = unique_temp_dir("dc53-vector8-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    let signer = transport_test_signer(0xa3)?;
    seal_two_block_history_with_author(&source, &signer, true)?;

    let (_, bytes) = export_bundle(&source, "heads/main")?;
    let (ref_name, objects, mut author_keys) =
        decode_bundle(&bytes, DEFAULT_BUNDLE_MAX_OBJECT_COUNT)?;
    assert_eq!(author_keys.len(), 1);
    if let Some(entry) = author_keys.first_mut() {
        // Swap in an unrelated public key for the same key_id -- the forgery this vector targets.
        entry.public_key = [0xbb; 32];
    }
    let tampered = encode_bundle(&ref_name, &objects, &author_keys)?;

    let target_root = unique_temp_dir("dc53-vector8-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let import_report = import_bundle(&target, &tampered, &BundleImportOptions::default_limits())?;
    assert_eq!(
        import_report.recorded_author_key_count, 1,
        "import records material without checking it, D7"
    );

    let imported_patch = find_imported_transport_patch(&target)?;
    let outcome = verify_author_signature(&target, &imported_patch);
    assert!(
        outcome.is_err(),
        "the transported key must not verify the Patch's real signature -- got {outcome:?}"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// DC-53 Stage 2, D7/C2: a bundle whose own author-key section carries two different public keys
/// for one `key_id` must be refused before any write -- a hostile or merely stale bundle must not
/// be able to leave a receiver with an unresolvable, permanently unverifiable `key_id`.
#[test]
fn import_rejects_a_bundle_whose_author_key_section_disagrees_with_itself()
-> prikk_error::Result<()> {
    let source_root = unique_temp_dir("dc53-bundle-internal-conflict-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    let signer = transport_test_signer(0xa4)?;
    seal_two_block_history_with_author(&source, &signer, true)?;

    let (_, bytes) = export_bundle(&source, "heads/main")?;
    let (ref_name, objects, mut author_keys) =
        decode_bundle(&bytes, DEFAULT_BUNDLE_MAX_OBJECT_COUNT)?;
    assert_eq!(author_keys.len(), 1);
    let key_id = author_keys
        .first()
        .map(|entry| entry.key_id.clone())
        .ok_or_else(|| {
            prikk_error::PrikkError::Integrity("expected one decoded author-key entry".to_string())
        })?;
    author_keys.push(AuthorKeyEntry {
        key_id: key_id.clone(),
        public_key: [0xcc; 32],
    });
    let hostile = encode_bundle(&ref_name, &objects, &author_keys)?;

    let target_root = unique_temp_dir("dc53-bundle-internal-conflict-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let before = crate::verify_repository(&target)?.checked_objects;
    let result = import_bundle(&target, &hostile, &BundleImportOptions::default_limits());
    assert!(
        result.is_err(),
        "a bundle whose own author-key section disagrees with itself must be refused"
    );
    let after = crate::verify_repository(&target)?.checked_objects;
    assert_eq!(
        before, after,
        "refused before any write -- the object store must be untouched"
    );
    assert_eq!(before, Some(0));
    // DC-53 Stage 2 implementation review v1, C1: the object store is not the container this
    // check protects -- the hazard is a partial write to the author-key container itself, which
    // has no prune/repair path. Asserted directly, not inferred from the object store happening
    // to move together with it (a coincidence of today's ordering, not a guarantee).
    assert_eq!(
        lookup_author_key_entries(&target, &key_id)?,
        Vec::new(),
        "a refused hostile bundle must leave no author-key entry behind, not even the attacker's \
         first-listed one"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// DC-53 Stage 2, D7: reusing Step 1's `record_author_key_material` at import means a transported
/// key conflicting with material this receiver *already* has locally is refused too -- distinct
/// from the bundle-internal case above, and the receiver's own existing material must survive
/// untouched.
#[test]
fn import_rejects_a_transported_key_conflicting_with_local_material() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("dc53-import-local-conflict-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    let signer = transport_test_signer(0xa5)?;
    seal_two_block_history_with_author(&source, &signer, true)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let target_root = unique_temp_dir("dc53-import-local-conflict-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let active_lock = ActiveLock::acquire(&target)?;
    record_author_key_material(&target, signer.key_id(), [0xdd; 32], &active_lock)?;
    drop(active_lock);

    let result = import_bundle(&target, &bytes, &BundleImportOptions::default_limits());
    assert!(
        result.is_err(),
        "a transported key conflicting with existing local material must be refused"
    );
    assert!(
        read_received_pointer(&target, "remotes/heads/main")?.is_none(),
        "a refused import must not create the received pointer"
    );
    let entries = lookup_author_key_entries(&target, signer.key_id())?;
    assert_eq!(
        entries,
        vec![AuthorKeyEntry {
            key_id: signer.key_id().to_string(),
            public_key: [0xdd; 32],
        }],
        "the transported conflicting key must never be recorded -- {entries:?}"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

/// DC-53 Stage 2, §1's ratified edge case: exporting a `key_id` whose *local* material already
/// disagrees with itself (only reachable via a legacy pre-Stage-2 state, planted directly here)
/// must fail the export rather than silently pick one of the two conflicting keys -- presenting the
/// receiver with an arbitrarily-chosen key would look like a provenance claim this sender's own
/// repository does not actually make.
#[test]
fn export_fails_when_local_material_already_conflicts_for_an_exported_key_id()
-> prikk_error::Result<()> {
    let root = unique_temp_dir("dc53-export-local-conflict");
    let layout = RepositoryLayout::init(root.clone())?;
    let signer = transport_test_signer(0xa6)?;
    seal_two_block_history_with_author(&layout, &signer, true)?;
    force_conflicting_author_key_entry_for_test(&layout, signer.key_id(), [0xee; 32])?;

    let result = export_bundle(&layout, "heads/main");
    assert!(
        result.is_err(),
        "export must refuse rather than silently pick one of two conflicting local keys"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-53 Stage 2, C1 (plan review): the author-key section's own declared count needs the same
/// DC-86 bound the object count already has -- a second declared count in a format a hostile
/// sender fully controls, with no bound, would reopen the hole DC-86 closed.
#[test]
fn author_key_count_limit_fires_exactly_at_the_boundary() -> prikk_error::Result<()> {
    let ref_name = "heads/main".to_string();
    let objects: Vec<ObjectEnvelope> = Vec::new();
    let author_keys = vec![
        AuthorKeyEntry {
            key_id: "a".to_string(),
            public_key: [1; 32],
        },
        AuthorKeyEntry {
            key_id: "b".to_string(),
            public_key: [2; 32],
        },
    ];
    let bytes = encode_bundle(&ref_name, &objects, &author_keys)?;

    let refused = decode_bundle(&bytes, 1);
    assert!(
        refused.is_err(),
        "a limit one below the actual author-key count (2) must refuse"
    );

    let accepted = decode_bundle(&bytes, 2);
    assert!(
        accepted.is_ok(),
        "a limit exactly at the actual author-key count (2) must accept"
    );

    Ok(())
}

/// DC-53 Stage 2 follow-up (bundle-v1-import-regression-v1.md): the actual migration path
/// `layout.rs`'s retired-format messages promise, walked end to end. A `PBNDL001` bundle -- encoded
/// the way a Stage-1-or-earlier build really produced one, not hand-edited bytes -- must import, its
/// Patch must read `Unverifiable` (never `Sound`: this bundle carries no author-key section at all,
/// regardless of what material the sender happened to have locally), and `verify_repository` must
/// pass and say so.
#[test]
fn a_pbndl001_bundle_imports_and_its_patch_reads_unverifiable() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("dc53-pbndl001-import-source");
    let source = RepositoryLayout::init(source_root.clone())?;

    // A genuinely, fully sealed history -- real commit, real seal, real adopted maintainer -- not
    // the lightweight `signed_block` structural fixture other tests in this file use, which never
    // computes a real state-merkle-root and so would fail `verify_repository`'s block-state stage
    // for reasons unrelated to this test. `verify passes and says so` (handoff §4) means a genuine
    // pass, not one read past unrelated fixture noise.
    let author = transport_test_signer(0xa8)?;
    let maintainer = Ed25519MaintainerSigner::from_seed("dc53-pbndl001-maintainer", &[0xa9; 32])?;
    crate::trust::add_trusted_maintainer(
        &source,
        maintainer.key_id(),
        &prikk_hash::to_hex(&maintainer.public_key_bytes()),
    )?;
    std::fs::write(source.root().join("v1-import.txt"), b"v1 import\n")?;
    crate::worktree_patch::commit_worktree_changes_signed(
        &source,
        "heads/main",
        "dc53 pbndl001 fixture",
        crate::worktree_patch::WorktreePatchCommitOptions::default(),
        &author,
    )?;
    // Records `author`'s key material locally on `source` as a side effect (the same production
    // path `node_authoring.rs` uses) -- the sender genuinely has material to carry, so the v1
    // path's own omission below is a real assertion, not an accident of the fixture having
    // nothing to drop.
    crate::rfc111_seal_simulation::simulate_one_seal(&source, "heads/main", &maintainer)?;

    let (_, v2_bytes) = export_bundle(&source, "heads/main")?;
    let (ref_name, objects, author_keys) =
        decode_bundle(&v2_bytes, DEFAULT_BUNDLE_MAX_OBJECT_COUNT)?;
    assert_eq!(
        author_keys.len(),
        1,
        "sanity: the sender really did have material to carry"
    );
    let v1_bytes = encode_bundle_v1_for_test(&ref_name, &objects)?;

    let target_root = unique_temp_dir("dc53-pbndl001-import-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let import_report = import_bundle(&target, &v1_bytes, &BundleImportOptions::default_limits())?;
    assert_eq!(
        import_report.recorded_author_key_count, 0,
        "a PBNDL001 bundle carries no author-key section to record"
    );

    let imported_patch = find_imported_transport_patch(&target)?;
    let outcome = verify_author_signature(&target, &imported_patch)?;
    assert_eq!(
        outcome,
        Some((author.key_id().to_string(), false)),
        "expected Unverifiable, got {outcome:?}"
    );

    let report = crate::verify_repository(&target)?;
    assert!(
        !report.has_item_failure(),
        "verify must pass against a v1-imported repository: {report:?}"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}
