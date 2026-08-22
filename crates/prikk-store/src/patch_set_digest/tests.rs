//! RFC 115 Stage 1 tests: frozen vectors, negative-control-observed preimage discipline, ref-kind
//! coverage (`heads/*`, `tags/*`, `remotes/*`), and the two-repositories-different-block-structure
//! property the digest exists for.

use prikk_hash::to_hex;
use prikk_object::{
    BlockKind, CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType, RefKind, RefStatePayload,
    RefUpdatePayload, TagPayload,
};

use super::{
    compute_patch_set_digest, compute_patch_set_digest_for_ref,
    compute_patch_set_digest_from_block, patch_set_digest_preimage,
};
use crate::test_support::{maintainer_signature, signed_block, unique_temp_dir};
use crate::{
    FileObjectStore, ObjectReader, ObjectWriter, RefPublication, RefStore, RepositoryLayout,
};

// RFC 115 Stage 1, `stage-1-patch-set-digest-handoff-v1.md` §3: committed literal vectors, computed
// once via a throwaway probe test and hardcoded here -- a vector derived at test time from the code
// under test asserts only that the code agrees with itself.

const EMPTY_PREIMAGE_HEX: &str =
    "5052494b4b2d50415443482d5345542d4449474553542d76310000000000000000";
const EMPTY_DIGEST_HEX: &str = "0cec641f3a910cc20c633152500c30f88915647aeaa80020d2151c820d8fa618";

const THREE_PREIMAGE_HEX: &str = "5052494b4b2d50415443482d5345542d4449474553542d76310000000000000003111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222222222222222222223333333333333333333333333333333333333333333333333333333333333333";
const THREE_DIGEST_HEX: &str = "1d832f45a66c6fb980c0d7d96c47328d269dd87b655aa309afc7db19de786235";

/// Vector 1: the empty patch set. The case most likely to be broken by a refactor and least likely
/// to be noticed (handoff §3 item 3) -- the count must still be hashed, distinguishing it from any
/// other degenerate shape.
#[test]
fn rfc115_vector_1_empty_patch_set() -> prikk_error::Result<()> {
    let empty: Vec<ObjectId> = Vec::new();
    assert_eq!(
        to_hex(&patch_set_digest_preimage(&empty)?),
        EMPTY_PREIMAGE_HEX
    );
    assert_eq!(
        to_hex(&compute_patch_set_digest(&empty)?.0),
        EMPTY_DIGEST_HEX
    );
    Ok(())
}

/// Vector 2: a three-element, already-sorted, distinct patch-id set.
#[test]
fn rfc115_vector_2_three_patch_set() -> prikk_error::Result<()> {
    let three = vec![
        ObjectId::from_bytes([0x11; 32]),
        ObjectId::from_bytes([0x22; 32]),
        ObjectId::from_bytes([0x33; 32]),
    ];
    assert_eq!(
        to_hex(&patch_set_digest_preimage(&three)?),
        THREE_PREIMAGE_HEX
    );
    assert_eq!(
        to_hex(&compute_patch_set_digest(&three)?.0),
        THREE_DIGEST_HEX
    );
    Ok(())
}

/// `compute_patch_set_digest` refuses input that is not strictly sorted and deduplicated, rather
/// than silently sorting it -- matching `state_root.rs`'s `compute_state_root` discipline (fail
/// loudly on caller misuse, don't paper over it).
#[test]
fn unsorted_input_is_refused_not_silently_sorted() {
    let unsorted = vec![
        ObjectId::from_bytes([0x22; 32]),
        ObjectId::from_bytes([0x11; 32]),
    ];
    assert!(compute_patch_set_digest(&unsorted).is_err());
    let duplicated = vec![
        ObjectId::from_bytes([0x11; 32]),
        ObjectId::from_bytes([0x11; 32]),
    ];
    assert!(compute_patch_set_digest(&duplicated).is_err());
}

fn write_block(
    store: &mut FileObjectStore,
    kind: BlockKind,
    parent_block_ids: Vec<ObjectId>,
    patch_ids: Vec<ObjectId>,
) -> prikk_error::Result<ObjectId> {
    let block = signed_block(kind, parent_block_ids, patch_ids, None);
    store.write_object(&block)
}

/// RFC 115 design §7 / ruling: two repositories that hold the same patches must produce the same
/// digest **even with different block structure on each side** -- the property the digest exists
/// for. A test that builds both sides identically would not prove it (handoff §5). Patch ids here
/// are bare `ObjectId` literals, never written as real Patch objects: `patch_ids_reachable_from_block`
/// only reads Block payloads' own `patch_ids` fields, the same closure `export_bundle` walks, and
/// never dereferences a Patch object itself.
#[test]
fn same_patches_different_block_structure_produce_equal_digests() -> prikk_error::Result<()> {
    let p1 = ObjectId::from_bytes([0x41; 32]);
    let p2 = ObjectId::from_bytes([0x42; 32]);

    // Repo A: both patches sealed into one block.
    let root_a = unique_temp_dir("rfc115-digest-structure-a");
    let layout_a = RepositoryLayout::init(root_a.clone())?;
    let mut store_a = FileObjectStore::new(layout_a.clone());
    let genesis_a = write_block(&mut store_a, BlockKind::Root, Vec::new(), Vec::new())?;
    let tip_a = write_block(
        &mut store_a,
        BlockKind::Normal,
        vec![genesis_a],
        vec![p1, p2],
    )?;

    // Repo B: the same two patches, split across two blocks -- different topology, same patch set.
    let root_b = unique_temp_dir("rfc115-digest-structure-b");
    let layout_b = RepositoryLayout::init(root_b.clone())?;
    let mut store_b = FileObjectStore::new(layout_b.clone());
    let genesis_b = write_block(&mut store_b, BlockKind::Root, Vec::new(), Vec::new())?;
    let middle_b = write_block(&mut store_b, BlockKind::Normal, vec![genesis_b], vec![p1])?;
    let tip_b = write_block(&mut store_b, BlockKind::Normal, vec![middle_b], vec![p2])?;

    let digest_a = compute_patch_set_digest_from_block(&store_a, tip_a)?;
    let digest_b = compute_patch_set_digest_from_block(&store_b, tip_b)?;
    assert_eq!(
        digest_a, digest_b,
        "same patch set, different block structure, must produce equal digests"
    );

    let _ = std::fs::remove_dir_all(root_a);
    let _ = std::fs::remove_dir_all(root_b);
    Ok(())
}

/// A repository with one extra patch produces a different digest (handoff §5).
#[test]
fn one_extra_patch_produces_a_different_digest() -> prikk_error::Result<()> {
    let p1 = ObjectId::from_bytes([0x51; 32]);
    let p2 = ObjectId::from_bytes([0x52; 32]);
    let p3 = ObjectId::from_bytes([0x53; 32]);

    let root_a = unique_temp_dir("rfc115-digest-extra-a");
    let layout_a = RepositoryLayout::init(root_a.clone())?;
    let mut store_a = FileObjectStore::new(layout_a.clone());
    let genesis_a = write_block(&mut store_a, BlockKind::Root, Vec::new(), Vec::new())?;
    let tip_a = write_block(
        &mut store_a,
        BlockKind::Normal,
        vec![genesis_a],
        vec![p1, p2],
    )?;

    let root_c = unique_temp_dir("rfc115-digest-extra-c");
    let layout_c = RepositoryLayout::init(root_c.clone())?;
    let mut store_c = FileObjectStore::new(layout_c.clone());
    let genesis_c = write_block(&mut store_c, BlockKind::Root, Vec::new(), Vec::new())?;
    let tip_c = write_block(
        &mut store_c,
        BlockKind::Normal,
        vec![genesis_c],
        vec![p1, p2, p3],
    )?;

    let digest_a = compute_patch_set_digest_from_block(&store_a, tip_a)?;
    let digest_c = compute_patch_set_digest_from_block(&store_c, tip_c)?;
    assert_ne!(digest_a, digest_c, "one extra patch must change the digest");

    let _ = std::fs::remove_dir_all(root_a);
    let _ = std::fs::remove_dir_all(root_c);
    Ok(())
}

fn signed_tag_ref_state_envelope(
    ref_name: &str,
    target_object_id: ObjectId,
    update_seq: u64,
) -> prikk_error::Result<ObjectEnvelope> {
    let payload = RefStatePayload {
        ref_name: ref_name.to_string(),
        kind: RefKind::Tag,
        target_object_id,
        update_seq,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::RefState, 1, payload.to_canonical_bytes()?);
    envelope.add_signature(maintainer_signature())?;
    Ok(envelope)
}

fn signed_ref_update_envelope_for(
    ref_name: &str,
    new_ref_state_id: ObjectId,
    new_target_object_id: ObjectId,
    update_seq: u64,
) -> prikk_error::Result<ObjectEnvelope> {
    let payload = RefUpdatePayload {
        ref_name: ref_name.to_string(),
        old_ref_state_id: None,
        new_ref_state_id,
        new_target_object_id,
        update_seq,
        created_at: 0,
        author_key_id: "maintainer-key".to_string(),
    };
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::RefUpdate, 1, payload.to_canonical_bytes()?);
    envelope.add_signature(maintainer_signature())?;
    Ok(envelope)
}

/// RFC 115 Stage 1 ruling §2.2: a tag ref and the `heads/*` ref pointing at the same block must
/// produce the same digest -- the property that fails immediately if tag metadata (name, message,
/// signature) is ever folded into the preimage.
#[test]
fn tag_ref_and_heads_ref_at_the_same_block_produce_the_same_digest() -> prikk_error::Result<()> {
    let root = unique_temp_dir("rfc115-digest-tag-vs-heads");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());
    let p1 = ObjectId::from_bytes([0x61; 32]);
    let genesis = write_block(&mut store, BlockKind::Root, Vec::new(), Vec::new())?;
    let tip = write_block(&mut store, BlockKind::Normal, vec![genesis], vec![p1])?;

    let ref_store = RefStore::new(layout.clone());
    let heads_state = crate::test_support::signed_ref_state_envelope("heads/main", None, tip, 1);
    let heads_state_id = heads_state.object_id();
    let heads_update =
        crate::test_support::signed_ref_update_envelope("heads/main", None, heads_state_id, tip, 1);
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state: heads_state,
        ref_update: heads_update,
    })?;

    let tag_payload = TagPayload {
        name: "v1".to_string(),
        target_block_id: tip,
        message: None,
        created_at: 0,
        author_key_id: "maintainer-key".to_string(),
        patch_set_digest: compute_patch_set_digest_from_block(&store, tip)?,
    };
    let mut tag_envelope =
        ObjectEnvelope::unsigned(ObjectType::Tag, 1, tag_payload.to_canonical_bytes()?);
    tag_envelope.add_signature(maintainer_signature())?;
    let tag_id = store.write_object(&tag_envelope)?;

    let tag_state = signed_tag_ref_state_envelope("tags/v1", tag_id, 1)?;
    let tag_state_id = tag_state.object_id();
    let tag_update = signed_ref_update_envelope_for("tags/v1", tag_state_id, tag_id, 1)?;
    ref_store.publish(&RefPublication {
        ref_name: "tags/v1".to_string(),
        expected_previous_ref_state_id: None,
        ref_state: tag_state,
        ref_update: tag_update,
    })?;

    let digest_heads = compute_patch_set_digest_for_ref(&layout, "heads/main")?;
    let digest_tag = compute_patch_set_digest_for_ref(&layout, "tags/v1")?;
    assert_eq!(
        digest_heads, digest_tag,
        "a tag ref and a heads ref pointing at the same block must produce the same digest"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// Ruling §2.3: `remotes/*` is refused explicitly, with a message naming the reason -- not the
/// misleading "ref does not exist" `RefStore` resolution would otherwise produce.
#[test]
fn remotes_ref_is_refused_explicitly_not_resolved() {
    let root = unique_temp_dir("rfc115-digest-remotes-refused");
    let layout_result = RepositoryLayout::init(root.clone());
    assert!(layout_result.is_ok());
    if let Ok(layout) = layout_result {
        let result = compute_patch_set_digest_for_ref(&layout, "remotes/heads/main");
        let message = match result {
            Ok(_) => panic!("remotes/* must be refused, not resolved"),
            Err(error) => error.to_string(),
        };
        assert!(
            message.contains("received"),
            "the refusal must name the received-namespace reason, not a generic lookup failure: {message}"
        );
        assert!(
            !message.contains("does not exist"),
            "the refusal must not read like a missing-ref lookup failure: {message}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

// RFC 117 T1 `stage-1-tag-payload-digest-handoff-v1.md` §6 tests, beyond the frozen-vector move
// already covered by
// `signature_contract_tests::vectors::rfc114_vector_11_tag_schema_1_identity_and_signature`.

/// Build, sign and store a v2 Tag at `target_block_id`, its `patch_set_digest` computed the same
/// way `prikk tag create` does -- `compute_patch_set_digest_from_block`, never anything block-local
/// like `target_block_id` itself (row 3's own point: two different blocks must still agree here).
fn build_and_store_tag(
    store: &mut FileObjectStore,
    name: &str,
    target_block_id: ObjectId,
) -> prikk_error::Result<(TagPayload, ObjectId)> {
    let tag_payload = TagPayload {
        name: name.to_string(),
        target_block_id,
        message: None,
        created_at: 0,
        author_key_id: "maintainer-key".to_string(),
        patch_set_digest: compute_patch_set_digest_from_block(store, target_block_id)?,
    };
    let mut tag_envelope =
        ObjectEnvelope::unsigned(ObjectType::Tag, 1, tag_payload.to_canonical_bytes()?);
    tag_envelope.add_signature(maintainer_signature())?;
    let tag_id = store.write_object(&tag_envelope)?;
    Ok((tag_payload, tag_id))
}

/// §6 row 2: a tag's own `patch_set_digest` equals an independent recomputation over its target
/// block. Writes a real Tag object (production shape: field 6 populated the same way `prikk tag
/// create` populates it), reads it back through the object store, and recomputes the digest
/// separately over the same block -- proving the persisted value round-trips and really is that
/// computation, not merely present.
#[test]
fn a_tags_digest_equals_its_blocks_patch_closure_digest() -> prikk_error::Result<()> {
    let root = unique_temp_dir("rfc117-digest-tag-equals-block");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout);
    let p1 = ObjectId::from_bytes([0x51; 32]);
    let genesis = write_block(&mut store, BlockKind::Root, Vec::new(), Vec::new())?;
    let tip = write_block(&mut store, BlockKind::Normal, vec![genesis], vec![p1])?;

    let (_, tag_id) = build_and_store_tag(&mut store, "v1", tip)?;
    let stored_envelope = store
        .read_typed(tag_id, ObjectType::Tag)?
        .ok_or_else(|| prikk_error::PrikkError::Integrity("missing Tag object".to_string()))?;
    let decoded = TagPayload::decode_canonical(&stored_envelope.canonical_payload)?;

    let independently_recomputed = compute_patch_set_digest_from_block(&store, tip)?;
    assert_eq!(
        decoded.patch_set_digest, independently_recomputed,
        "a tag's own digest must equal an independent recomputation over its target block"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// §6 row 3 -- the property RFC 117 exists for: two independently-constructed repositories holding
/// the same patches produce tags with the same `patch_set_digest`, even with different block
/// structure on each side (`same_patches_different_block_structure_produce_equal_digests` above
/// already proves this for the underlying digest; this proves the *tag* now carries it correctly).
#[test]
fn two_independent_repositories_holding_the_same_patches_produce_equal_tag_digests()
-> prikk_error::Result<()> {
    let p1 = ObjectId::from_bytes([0x91; 32]);
    let p2 = ObjectId::from_bytes([0x92; 32]);

    // Repo A: both patches sealed into one block.
    let root_a = unique_temp_dir("rfc117-tag-digest-cross-repo-a");
    let layout_a = RepositoryLayout::init(root_a.clone())?;
    let mut store_a = FileObjectStore::new(layout_a);
    let genesis_a = write_block(&mut store_a, BlockKind::Root, Vec::new(), Vec::new())?;
    let tip_a = write_block(
        &mut store_a,
        BlockKind::Normal,
        vec![genesis_a],
        vec![p1, p2],
    )?;
    let (tag_a, _) = build_and_store_tag(&mut store_a, "v1", tip_a)?;

    // Repo B: the same two patches, split across two blocks -- different topology, same patch set.
    let root_b = unique_temp_dir("rfc117-tag-digest-cross-repo-b");
    let layout_b = RepositoryLayout::init(root_b.clone())?;
    let mut store_b = FileObjectStore::new(layout_b);
    let genesis_b = write_block(&mut store_b, BlockKind::Root, Vec::new(), Vec::new())?;
    let middle_b = write_block(&mut store_b, BlockKind::Normal, vec![genesis_b], vec![p1])?;
    let tip_b = write_block(&mut store_b, BlockKind::Normal, vec![middle_b], vec![p2])?;
    let (tag_b, _) = build_and_store_tag(&mut store_b, "v1", tip_b)?;

    assert_ne!(
        tag_a.target_block_id, tag_b.target_block_id,
        "fixture sanity: the two repositories' tip blocks must genuinely differ in structure"
    );
    assert_eq!(
        tag_a.patch_set_digest, tag_b.patch_set_digest,
        "two independently-constructed repositories holding the same patches must produce tags \
         with the same patch_set_digest, even with different block structure"
    );

    let _ = std::fs::remove_dir_all(root_a);
    let _ = std::fs::remove_dir_all(root_b);
    Ok(())
}
