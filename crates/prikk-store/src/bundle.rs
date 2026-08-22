//! History exchange artifact (DC-78 §D6): a **verifiable subset**, never a summary. A bundle carries
//! the exported ref's RefState plus every object reachable from its target Block back to genesis
//! (ruling 2) — the same objects `verify_repository` already checks for any locally sealed history,
//! unconditionally, regardless of which ref (if any) points to them. Import writes exactly those
//! objects plus one `received` pointer (`crate::received`); it never touches `refs/by-id/`, never
//! advances a local ref, and never adopts a MAINTAINER key into the local trust policy — adopting
//! trust for an imported key remains the operator's own explicit `trust maintainer add` call. The
//! receiver's confidence comes from running ordinary, unmodified `verify_repository` afterward: this
//! module adds a serialization boundary and an import path, and deliberately no new verification
//! machinery (§D6's "no new verification path").
//!
//! **`heads/*` and `tags/*` both export** (`resolve_ref_target_block`). A branch's `target_object_id`
//! names its Block directly; a tag's names a Tag object one hop away
//! (`TagPayload.target_block_id` is the Block), mirroring the two-hop model
//! `refs/verify/scan.rs`'s own `ensure_block_exists`/tag-target check already uses. **The Tag
//! envelope itself is part of the exported object set, not just the Block closure it points to**:
//! the tag ref's `RefState` travels unconditionally either way, and a receiver holding a signed
//! RefState that names a Tag object they do not have fails `verify` with a message naming exactly
//! that (DC-78 bundle-export tag gap follow-up, `bundle-export-tag-ref-gap-v1.md` — found while
//! investigating RFC 115 Stage 1's own reachable-set question, fixed independently of it).
//!
//! **DC-53 Stage 2, D6: `PBNDL002` carries an AUTHOR key-material section.** The section's scope is
//! exactly the AUTHOR `key_id`s of the Patches this bundle carries, derived from the exported objects
//! themselves, never the whole local `author_key_index` container -- exporting everything this
//! repository has ever seen would leak every author it has observed to every recipient, a disclosure
//! the sender did not choose. Material is optional per-`key_id`: a bundle omits a `key_id` this
//! repository never recorded material for (import still succeeds; the Patch reads Unverifiable, not
//! Sound and not a failure). **Import records material; `verify` decides (D7)** -- import performs no
//! cryptographic check of whether a transported key actually matches any Patch's signature, the same
//! way it already writes objects without re-verifying them; a transported key that doesn't verify a
//! Patch's signature is recorded anyway and `verify` reports that Patch `Failed`, reached through the
//! transport path rather than local authoring but the same underlying check (D3's third row).
//!
//! **Two independent conflict checks, and both are fully pre-write.** Before any write: the bundle's
//! own author-key section must not itself claim two different public keys for one `key_id` -- a
//! hostile or merely stale bundle must fail the whole import rather than leave a receiver with an
//! unresolvable, permanently unverifiable `key_id` (`author_key_index.rs`'s own container has no
//! prune/repair path). The second check -- a transported key conflicting with material this
//! repository *already* has -- is checked the same way: **every** transported key is validated
//! against local material, via `check_author_key_conflict`, before any of them is recorded, both
//! inside the one `ActiveLock` held for the section. This is why: a refused import must leave the
//! author-key container exactly as untouched as a refused bundle-internal check leaves it -- checking
//! and recording one entry at a time let a conflict at entry `k` leave entries `1..k-1` durably
//! appended first (DC-53 Stage 2 follow-up, `multi-key-import-partial-write-v1.md`). Recording itself
//! still goes through `record_author_key_material`, Step 1's own function, unchanged -- no separate
//! transport-side pinning rule, and no second copy of the conflict definition (Step 1's C2 ruling).
//!
//! **`PBNDL001` (Stage 1 and earlier) is accepted on import, never emitted on export** (DC-53 Stage 2
//! follow-up, `bundle-v1-import-regression-v1.md`). `layout.rs`'s own retired-repository-format
//! messages instruct a user to open an old repository with an old prikk build and `bundle export`
//! from it; that build only ever produces `PBNDL001`, and refusing to import it here severed the one
//! migration path those messages promise, in both directions at once -- found and fixed the same day
//! it shipped. A `PBNDL001` bundle decodes exactly like a `PBNDL002` one with an empty author-key
//! section: not a special case, since DC-53 already defines "no recorded material" as `Unverifiable`
//! (vector 7). Read compatibility only, the same asymmetry every repository-format transition in this
//! project already has -- read what the past wrote, write only the present.
//!
//! **`import_bundle` validates closure completeness before any write** (DC-78
//! `import-closure-validation-handoff-v1.md`): the exported ref's target must resolve, and every blob
//! (whether a Patch's own operations or a Block's own `snapshot_blob_ref` names it), patch, and block
//! parent a carried object names must be present -- carried by this bundle, or already in this
//! repository. `accept_exchange_artifact` (`patch_exchange/accept.rs`) already refused
//! the same class of defect at receipt; `import_bundle` did not, so a bundle whose target object it
//! never shipped used to import successfully and land a dangling received pointer, visible only at the
//! next `verify`, long after the import that caused it. **This is an intentional behaviour change: a
//! bundle that previously imported may now be refused.** That is the point, not a regression -- a tag
//! bundle produced before the DC-78 tag-export fix (`d605c10`), which carries the RefState but not the
//! Tag object, is the concrete case: it now fails at import, naming the bundle at the moment it is
//! offered, instead of importing and failing a later `verify` with no indication of which import caused
//! it.

use std::collections::{BTreeMap, BTreeSet};

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlockPayload, ObjectEnvelope, ObjectId, ObjectType, RefKind, RefStatePayload, Signature,
    SignerRole, TagPayload,
};

use crate::author_key_index::{
    AuthorKeyEntry, check_author_key_conflict, lookup_author_key_entries,
    record_author_key_material,
};
use crate::byte_cursor::ByteCursor;
use crate::file_codec::{decode_envelope_file, encode_envelope_file, push_bytes_u64, push_u64};
use crate::fsutil::len_to_u64;
use crate::layout::{LockableContainer, RepositoryLayout};
use crate::lock::{ActiveLock, acquire_container_locks};
use crate::object_store::{ObjectReadSnapshot, ObjectReader, ObjectWriteSession, ObjectWriter};
use crate::patch_replay::decode::{
    DecodedDeletePreimage, DecodedOperationKind, decode_patch_operations,
};
use crate::refs::{RefStore, ensure_ref_target_valid};

/// DC-53 Stage 2, D6: the format bump that made room for the author-key section. Always emitted on
/// export; `PBNDL001` is still accepted on import (see `RETIRED_BUNDLE_MAGIC_V1`), so this is a
/// write-side-only version, not a hard cutover.
const BUNDLE_MAGIC: &[u8; 8] = b"PBNDL002";
/// `PBNDL001` (Stage 1 and earlier bundles, no author-key section). Accepted on import -- see
/// `decode_bundle`'s own doc and the module doc's follow-up note -- never emitted on export.
const RETIRED_BUNDLE_MAGIC_V1: &[u8; 8] = b"PBNDL001";

/// DC-86 default hard block on a bundle's declared object count, checked as early as the format
/// allows — right after the count header field, before a single object is decoded. Not a claim about
/// what any real bundle needs; a ceiling an operator can rely on existing at all.
pub const DEFAULT_BUNDLE_MAX_OBJECT_COUNT: usize = 100_000;

/// DC-86 default hard block on a bundle's total encoded byte length, checked before any decoding
/// begins. This length-prefixed format can never decode to more logical content than its encoded
/// input size, so bounding the input bytes is a tight, cheap proxy for bounding decoded bytes —
/// cheaper than decoding first only to discover the result should have been refused. 256 MiB.
pub const DEFAULT_BUNDLE_MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;

/// DC-86 resource bound for [`import_bundle`], checked before any object is decoded or written —
/// DC-57's shape: a hard block ahead of any write, with a documented default the CLI may override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleImportOptions {
    /// Maximum objects a bundle may declare. Refused before the decode loop runs.
    pub max_object_count: usize,
    /// Maximum encoded byte length a bundle may have. Refused before `decode_bundle` runs at all.
    pub max_total_bytes: usize,
}

impl BundleImportOptions {
    /// [`DEFAULT_BUNDLE_MAX_OBJECT_COUNT`] and [`DEFAULT_BUNDLE_MAX_TOTAL_BYTES`].
    #[must_use]
    pub const fn default_limits() -> Self {
        Self {
            max_object_count: DEFAULT_BUNDLE_MAX_OBJECT_COUNT,
            max_total_bytes: DEFAULT_BUNDLE_MAX_TOTAL_BYTES,
        }
    }

    /// Override the maximum object count.
    #[must_use]
    pub const fn with_max_object_count(mut self, max_object_count: usize) -> Self {
        self.max_object_count = max_object_count;
        self
    }

    /// Override the maximum total encoded byte length.
    #[must_use]
    pub const fn with_max_total_bytes(mut self, max_total_bytes: usize) -> Self {
        self.max_total_bytes = max_total_bytes;
        self
    }
}

/// Summary of a bundle export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleExportReport {
    /// The exported ref's own name in the source repository.
    pub ref_name: String,
    /// The exported ref's target Block at export time.
    pub tip_block_id: ObjectId,
    /// Total objects carried in the bundle (the RefState plus its full genesis-complete closure).
    pub object_count: usize,
    /// DC-53 Stage 2, D6: AUTHOR key entries carried in the bundle's author-key section -- one per
    /// distinct `key_id` among the bundle's Patches for which this repository has local material,
    /// never a count of the Patches themselves.
    pub author_key_count: usize,
}

/// Summary of a bundle import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleImportReport {
    /// The local received-namespace name the import was recorded under (`remotes/<origin ref name>`).
    pub ref_name: String,
    /// The imported RefState's object id, now the received pointer's target.
    pub ref_state_id: ObjectId,
    /// Total objects the bundle carried.
    pub object_count: usize,
    /// Objects that did not already exist in this repository's object store before this import.
    pub written_object_count: usize,
    /// DC-53 Stage 2, D7: AUTHOR key entries the bundle carried and this import recorded locally.
    /// Continuity only, not a trust decision -- unlike a trusted maintainer key, recording this
    /// grants no admission judgement, only lets `verify` distinguish Sound from Unverifiable for the
    /// Patches it covers.
    pub recorded_author_key_count: usize,
}

/// Export a genesis-complete, verifiable subset of objects for `ref_name` (DC-78 §D4/§D6). Walks the
/// full Block ancestor closure (all parents, not mainline-only — ruling 2's "genesis-complete") plus
/// every Patch and Blob those blocks reference, plus the exported RefState's own required
/// attestations. Returns the report and the encoded bundle bytes; writing them to a file is a CLI
/// concern, not this crate's.
pub fn export_bundle(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<(BundleExportReport, Vec<u8>)> {
    let ref_store = RefStore::new(layout.clone());
    // RFC 111 §6.1: export is read-only end to end (never calls `write_object`), so it takes one
    // decoded index snapshot here instead of paying a fresh decode per `read_required` call below --
    // and there can be many, one per object in the whole exported closure.
    let object_store = ObjectReadSnapshot::open(layout)?;
    let Some(ref_state_id) = ref_store.read_current_ref_state_id(ref_name)? else {
        return Err(PrikkError::Integrity(format!(
            "ref {ref_name} does not exist, nothing to export"
        )));
    };
    let ref_state_envelope = object_store
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing RefState object: {ref_state_id}")))?;
    let ref_state_payload = RefStatePayload::decode_canonical(
        &ref_state_envelope.canonical_payload,
        ref_state_envelope.schema_version,
    )?;
    let mut tag_envelopes: Vec<ObjectEnvelope> = Vec::new();
    let tip_block_id =
        resolve_ref_target_block(&object_store, &ref_state_payload, &mut tag_envelopes)?;

    // Genesis-complete (ruling 2) covers the publication chain too, not only the Block DAG: a
    // received ref's `log --ref` walks `previous_ref_state_id` exactly as a local ref's does, so
    // every earlier RefState this ref ever published is required, not only the tip.
    let mut ref_state_chain: Vec<ObjectEnvelope> = vec![ref_state_envelope];
    let mut required_attestation_ids: BTreeSet<ObjectId> = ref_state_payload
        .required_attestation_ids
        .iter()
        .copied()
        .collect();
    let mut ancestors = crate::merge_evidence::ancestors_inclusive(&object_store, tip_block_id)?;
    let mut previous = ref_state_payload.previous_ref_state_id;
    let mut seen_ref_states: BTreeSet<ObjectId> = BTreeSet::from([ref_state_id]);
    while let Some(previous_id) = previous {
        if !seen_ref_states.insert(previous_id) {
            return Err(PrikkError::Integrity(format!(
                "RefState chain for {ref_name} contains a cycle at {previous_id}"
            )));
        }
        let envelope = read_required(&object_store, previous_id, ObjectType::RefState)?;
        let payload = RefStatePayload::decode_canonical(
            &envelope.canonical_payload,
            envelope.schema_version,
        )?;
        required_attestation_ids.extend(payload.required_attestation_ids.iter().copied());
        let target_block_id =
            resolve_ref_target_block(&object_store, &payload, &mut tag_envelopes)?;
        ancestors.extend(crate::merge_evidence::ancestors_inclusive(
            &object_store,
            target_block_id,
        )?);
        previous = payload.previous_ref_state_id;
        ref_state_chain.push(envelope);
    }

    let mut patch_ids: BTreeSet<ObjectId> = BTreeSet::new();
    let mut blob_ids: BTreeSet<ObjectId> = BTreeSet::new();
    for payload in ancestors.values() {
        patch_ids.extend(payload.patch_ids.iter().copied());
        if let Some(blob_id) = payload.snapshot_blob_ref {
            blob_ids.insert(blob_id);
        }
    }

    let mut objects: Vec<ObjectEnvelope> = ref_state_chain;
    objects.append(&mut tag_envelopes);
    for block_id in ancestors.keys() {
        objects.push(read_required(&object_store, *block_id, ObjectType::Block)?);
    }
    let mut patch_envelopes: Vec<ObjectEnvelope> = Vec::with_capacity(patch_ids.len());
    for patch_id in &patch_ids {
        let envelope = read_required(&object_store, *patch_id, ObjectType::Patch)?;
        // A Patch's operations can themselves reference Blobs (CreateFile, file-kind DeleteNode,
        // ReplaceBinary) independently of any Block's snapshot_blob_ref — a repository-local
        // verify never notices a missing one of these, since it never replays lifecycle state for
        // an export; the receiver's ordinary verify does, so every one of these must travel too.
        for operation in
            crate::patch_replay::decode::decode_patch_operations(&envelope.canonical_payload)?
        {
            match operation.kind {
                crate::patch_replay::decode::DecodedOperationKind::CreateFile {
                    blob_id, ..
                } => {
                    blob_ids.insert(blob_id);
                }
                crate::patch_replay::decode::DecodedOperationKind::ReplaceBinary {
                    old_blob_id,
                    new_blob_id,
                    ..
                } => {
                    blob_ids.insert(old_blob_id);
                    blob_ids.insert(new_blob_id);
                }
                crate::patch_replay::decode::DecodedOperationKind::DeleteNode {
                    preimage:
                        crate::patch_replay::decode::DecodedDeletePreimage::File { old_blob_id, .. },
                    ..
                } => {
                    blob_ids.insert(old_blob_id);
                }
                _ => {}
            }
        }
        patch_envelopes.push(envelope);
    }
    objects.extend(patch_envelopes);
    for blob_id in &blob_ids {
        objects.push(read_required(&object_store, *blob_id, ObjectType::Blob)?);
    }
    for attestation_id in &required_attestation_ids {
        objects.push(read_required(
            &object_store,
            *attestation_id,
            ObjectType::Attestation,
        )?);
    }

    // DC-53 Stage 2, D6: the author-key section's scope is exactly the AUTHOR key_ids of the
    // Patches this bundle carries -- derived from `objects` itself, never the whole local
    // `author_key_index` container (which would leak every author this repository has ever seen).
    let mut author_key_ids: BTreeSet<String> = BTreeSet::new();
    for envelope in &objects {
        if envelope.object_type != ObjectType::Patch {
            continue;
        }
        if let Some(signature) = envelope
            .signatures
            .iter()
            .find(|signature| signature.signer_role == SignerRole::Author)
        {
            author_key_ids.insert(signature.key_id.clone());
        }
    }
    let mut author_keys: Vec<AuthorKeyEntry> = Vec::with_capacity(author_key_ids.len());
    for key_id in &author_key_ids {
        let entries = lookup_author_key_entries(layout, key_id)?;
        let mut distinct = entries.iter().map(|entry| entry.public_key);
        let Some(first) = distinct.next() else {
            // No local material for this key_id -- omitted from the section, not an error (§3's
            // "material is optional per-author", vector 7).
            continue;
        };
        if distinct.any(|public_key| public_key != first) {
            // A legacy local conflict for a key_id this export would otherwise carry. Only possible
            // from a repository predating Stage 2's own rejection of new conflicts (Step 1's
            // migration scan found none in this project's own fixtures). Fail rather than silently
            // pick one: presenting the receiver with an arbitrarily-chosen key would look like a
            // provenance claim this sender's own repository does not actually make.
            return Err(PrikkError::Integrity(format!(
                "author key_id {key_id} has more than one distinct recorded public key locally; \
                 refusing to export a provenance claim this repository's own material does not \
                 agree on -- run doctor, though no repair exists for this container"
            )));
        }
        author_keys.push(AuthorKeyEntry {
            key_id: key_id.clone(),
            public_key: first,
        });
    }

    let object_count = objects.len();
    let author_key_count = author_keys.len();
    let bytes = encode_bundle(ref_name, &objects, &author_keys)?;
    Ok((
        BundleExportReport {
            ref_name: ref_name.to_string(),
            tip_block_id,
            object_count,
            author_key_count,
        },
        bytes,
    ))
}

/// Import a bundle's objects and record a `received` pointer for its ref (DC-78 §D4). Never touches
/// `refs/by-id/`, never advances a local ref, and never adopts any MAINTAINER key into the local trust
/// policy — the imported RefState/Blocks remain ordinary, structurally-checkable objects that `verify`
/// will report as untrusted until the operator explicitly runs `trust maintainer add` for the key that
/// sealed them. That is a deliberate choice, not an oversight: auto-adopting a key seen in imported
/// history would be a new, unreviewed trust mechanism, exactly what §D6 rules out.
pub fn import_bundle(
    layout: &RepositoryLayout,
    bytes: &[u8],
    options: &BundleImportOptions,
) -> Result<BundleImportReport> {
    if bytes.len() > options.max_total_bytes {
        return Err(PrikkError::MalformedData(format!(
            "bundle is {} bytes, over the configured limit of {} bytes",
            bytes.len(),
            options.max_total_bytes
        )));
    }
    let (origin_ref_name, objects, author_keys) = decode_bundle(bytes, options.max_object_count)?;
    let Some(ref_state_envelope) = objects.first() else {
        return Err(PrikkError::MalformedData(
            "bundle contains no objects".to_string(),
        ));
    };
    if ref_state_envelope.object_type != ObjectType::RefState {
        return Err(PrikkError::MalformedData(
            "bundle's first object must be the exported ref's RefState".to_string(),
        ));
    }
    let ref_state_id = ref_state_envelope.object_id();

    // DC-53 Stage 2, D7/C2: reject the whole import before any write if the bundle's own
    // author-key section already disagrees with itself -- a hostile or stale bundle must not be
    // able to leave a receiver with an unresolvable, permanently unverifiable key_id.
    // `record_author_key_material` below still catches a conflict against *this repository's*
    // existing material; this catches a conflict *within the bundle*, which that call alone
    // wouldn't see if the bundle's own two conflicting entries happened to be processed in an order
    // where the first one matched nothing local yet.
    let mut bundle_key_ids: BTreeMap<&str, [u8; 32]> = BTreeMap::new();
    for entry in &author_keys {
        match bundle_key_ids.get(entry.key_id.as_str()) {
            Some(existing) if *existing != entry.public_key => {
                return Err(PrikkError::MalformedData(format!(
                    "bundle's author-key section carries two different public keys for key_id {} \
                     -- refusing the whole import",
                    entry.key_id
                )));
            }
            Some(_) => {}
            None => {
                bundle_key_ids.insert(&entry.key_id, entry.public_key);
            }
        }
    }

    // DC-78 closure-validation handoff §2/§3: everything below is read-only and must run, and pass,
    // before the first object write below -- a refused bundle must leave no pointer at all, and the
    // pointer is written last, so validating up front is what makes that true. §2's own definition of
    // "present": carried by this bundle, or already in this repository -- an incremental import onto
    // a repository that already holds part of the history must not be refused for objects it already
    // has (D7's rule again). `accept_exchange_artifact` (`patch_exchange/accept.rs`) already gets this
    // right for the patch-exchange path; this makes `import_bundle` match it rather than "align" it
    // to something new (§6's own instruction).
    let read_snapshot = ObjectReadSnapshot::open(layout)?;
    let bundle_objects_by_id: BTreeMap<ObjectId, ObjectEnvelope> = objects
        .iter()
        .map(|envelope| (envelope.object_id(), envelope.clone()))
        .collect();

    // Item 1: the exported ref's target resolves. Reuses `ensure_ref_target_valid`
    // (`refs/verify/scan.rs`) unchanged -- it is already kind-aware (one hop for a Branch, two for a
    // Tag) and already `pub(crate)` at `crate::refs`, so no visibility widening was needed to reach it
    // from here.
    let ref_state_payload = RefStatePayload::decode_canonical(
        &ref_state_envelope.canonical_payload,
        ref_state_envelope.schema_version,
    )?;
    let combined_reader = BundleAndLocalReader {
        bundle_objects: &bundle_objects_by_id,
        local: &read_snapshot,
    };
    ensure_ref_target_valid(
        &combined_reader,
        ref_state_payload.kind,
        ref_state_payload.target_object_id,
        ref_state_id,
    )?;

    // Items 2 and 3: every blob a carried patch's operations reference, and every patch a carried
    // block names, must be present. Mirrors `accept_exchange_artifact`'s Phase B item 6 exactly,
    // including the "or already present locally" half.
    for envelope in &objects {
        if envelope.object_type != ObjectType::Patch {
            continue;
        }
        for operation in decode_patch_operations(&envelope.canonical_payload)? {
            for blob_id in bundle_referenced_blob_ids(&operation.kind) {
                if !bundle_objects_by_id.contains_key(&blob_id)
                    && !read_snapshot.contains_object(ObjectType::Blob, blob_id)
                {
                    return Err(PrikkError::Integrity(format!(
                        "patch {} references blob {blob_id}, which is neither carried by this \
                         bundle nor already present in this repository -- refusing the whole \
                         import, no partial write",
                        envelope.object_id()
                    )));
                }
            }
        }
    }
    for envelope in &objects {
        if envelope.object_type != ObjectType::Block {
            continue;
        }
        let block_id = envelope.object_id();
        let block_payload = BlockPayload::decode_canonical(&envelope.canonical_payload)?;
        for patch_id in &block_payload.patch_ids {
            if !bundle_objects_by_id.contains_key(patch_id)
                && !read_snapshot.contains_object(ObjectType::Patch, *patch_id)
            {
                return Err(PrikkError::Integrity(format!(
                    "block {block_id} names patch {patch_id}, which is neither carried by this \
                     bundle nor already present in this repository -- refusing the whole import, \
                     no partial write"
                )));
            }
        }
        // Item 4: every parent a carried block names must be present too. `export_bundle` walks the
        // full ancestor closure, so this holds for anything the current exporter produces -- checking
        // it is set membership and costs nothing (handoff §2 item 4).
        for parent_block_id in &block_payload.parent_block_ids {
            if !bundle_objects_by_id.contains_key(parent_block_id)
                && !read_snapshot.contains_object(ObjectType::Block, *parent_block_id)
            {
                return Err(PrikkError::Integrity(format!(
                    "block {block_id} names parent {parent_block_id}, which is neither carried \
                     by this bundle nor already present in this repository -- refusing the whole \
                     import, no partial write"
                )));
            }
        }
        // Review condition (`DC-78-import-closure-validation-review-v1.md` §3): a Block's own
        // `snapshot_blob_ref` is a blob reference too, same rule as a Patch's operation-level ones --
        // `export_bundle` already puts it in the same `blob_ids` set as those, so every legitimate
        // export already satisfies this; the check exists for the untrusted, hand-crafted case, which
        // is exactly the case this whole increment is for.
        if let Some(snapshot_blob_id) = block_payload.snapshot_blob_ref {
            if !bundle_objects_by_id.contains_key(&snapshot_blob_id)
                && !read_snapshot.contains_object(ObjectType::Blob, snapshot_blob_id)
            {
                return Err(PrikkError::Integrity(format!(
                    "block {block_id} names snapshot blob {snapshot_blob_id}, which is neither \
                     carried by this bundle nor already present in this repository -- refusing \
                     the whole import, no partial write"
                )));
            }
        }
    }

    // RFC 111 §6.1 Stage 2: `import_bundle`'s ref-equivalent write is `received::write_received_-
    // pointer`, a wholly separate mechanism (received-ref index, not the pointer-index/ref-log
    // `RefStore::publish` touches) with no `FileObjectStore` construction of its own -- confirmed by
    // reading `received.rs`. No ref-publication threading needed, only the plain swap.
    let mut object_store = ObjectWriteSession::open(layout)?;
    let mut written_object_count = 0_usize;
    for envelope in &objects {
        let id = envelope.object_id();
        if !object_store.contains_object(envelope.object_type, id)? {
            written_object_count = written_object_count.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("bundle import written-object count overflow".to_string())
            })?;
        }
        object_store.write_object(envelope)?;
    }

    // DC-53 Stage 2, D7: import records material, `verify` decides -- no cryptographic check here,
    // matching the object writes above. `import_bundle` is now a third caller of
    // `record_author_key_material` (`node_authoring.rs`, `rollback_draft.rs` are the other two),
    // so it acquires `ActiveLock` around this section the same way they do -- the same container,
    // the same check-then-act, the same unrecoverable conflict state, one lock path per
    // repository so this also serializes against a concurrent commit or rollback-draft, not only
    // against another import. A conflict here (against this repository's own existing material,
    // distinct from the bundle-internal check above) fails the whole import, not just this entry.
    //
    // DC-53 Stage 2 follow-up (`multi-key-import-partial-write-v1.md`): with `m > 1` transported
    // keys, checking-then-recording one entry at a time let a conflict at entry `k` leave entries
    // `1..k-1` durably appended to a container with no prune, no compaction, and no repair --
    // exactly the partial-write hazard layer 1's bundle-internal check exists to prevent, just one
    // layer later. Fixed the same way: validate every entry against local material *before*
    // recording any of it, both passes inside the one `ActiveLock` already held (a validate-then-
    // record split across the lock boundary would be a check-then-act race across the two passes,
    // the same defect this fixes).
    let mut recorded_author_key_count = 0_usize;
    {
        let active_lock = ActiveLock::acquire(layout)?;
        for (&key_id, &public_key) in &bundle_key_ids {
            check_author_key_conflict(layout, key_id, public_key)?;
        }
        for entry in &author_keys {
            record_author_key_material(layout, &entry.key_id, entry.public_key, &active_lock)?;
            recorded_author_key_count =
                recorded_author_key_count.checked_add(1).ok_or_else(|| {
                    PrikkError::Integrity(
                        "bundle import recorded-author-key count overflow".to_string(),
                    )
                })?;
        }
    }

    let received_ref_name = format!("remotes/{origin_ref_name}");
    // RFC 102 Stage 6 Step 2, design-v1.md §15.8: `import_bundle` held no lock at all before this
    // stage. Scoped to the received-index write alone, not the object writes above: those have
    // their own, separately registered concurrency gap -- see
    // docs/src/reference/concurrency-locking.md#object-container-writes-are-not-among-the-four-locked-containers,
    // out of this stage's scope.
    let _received_index_lock =
        acquire_container_locks(layout, &[LockableContainer::ReceivedIndex])?;
    crate::received::write_received_pointer(layout, &received_ref_name, ref_state_id)?;

    Ok(BundleImportReport {
        ref_name: received_ref_name,
        ref_state_id,
        object_count: objects.len(),
        written_object_count,
        recorded_author_key_count,
    })
}

fn read_required(
    object_store: &impl ObjectReader,
    id: ObjectId,
    object_type: ObjectType,
) -> Result<ObjectEnvelope> {
    object_store
        .read_typed(id, object_type)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing {object_type} object: {id}")))
}

/// `import_bundle`'s own view of §2's "present" definition: carried by this bundle, or already in
/// this repository. Checked before any bundle object is written, so a bundle-carried object is not
/// yet reachable through `local` even once import succeeds -- `bundle_objects` is what makes it
/// visible during validation.
struct BundleAndLocalReader<'a> {
    bundle_objects: &'a BTreeMap<ObjectId, ObjectEnvelope>,
    local: &'a ObjectReadSnapshot,
}

impl ObjectReader for BundleAndLocalReader<'_> {
    fn read_object(&self, id: ObjectId) -> Result<Option<ObjectEnvelope>> {
        if let Some(envelope) = self.bundle_objects.get(&id) {
            return Ok(Some(envelope.clone()));
        }
        self.local.read_object(id)
    }
}

/// Every blob id one decoded patch operation references -- restated from
/// `patch_exchange/accept.rs`'s own `referenced_blob_ids` (the same three kinds `export_bundle`'s
/// closure walk above also scans for), for `import_bundle`'s own closure-completeness check. Kept as
/// a separate, per-module copy rather than a cross-module `pub(crate)` call: `accept.rs`'s own doc
/// comment already restates this same match once for `artifact.rs`'s benefit, so a third small copy
/// here follows the precedent this file already has, not a new one.
fn bundle_referenced_blob_ids(kind: &DecodedOperationKind) -> Vec<ObjectId> {
    match kind {
        DecodedOperationKind::CreateFile { blob_id, .. } => vec![*blob_id],
        DecodedOperationKind::ReplaceBinary {
            old_blob_id,
            new_blob_id,
            ..
        } => vec![*old_blob_id, *new_blob_id],
        DecodedOperationKind::DeleteNode {
            preimage: DecodedDeletePreimage::File { old_blob_id, .. },
            ..
        } => vec![*old_blob_id],
        _ => Vec::new(),
    }
}

/// Resolve `ref_state_payload.target_object_id` to the Block it ultimately names -- one hop for a
/// `Branch` (the target *is* the Block), two hops for a `Tag` (the target is a Tag object; its own
/// `target_block_id` is the Block). `refs/verify/scan.rs`'s own `ensure_block_exists`/tag-target
/// check already models this same two-hop shape; `export_bundle` did not (DC-78 bundle-export tag
/// gap follow-up, `bundle-export-tag-ref-gap-v1.md`).
///
/// **Exhaustive match, no wildcard**: a future `RefKind` variant must fail to compile here rather
/// than silently resolve to nothing and surface as a misleading "missing Block" error the way the
/// original defect did.
///
/// For a `Tag`, the resolved Tag envelope is pushed onto `tag_envelopes` so the caller can include
/// it in the exported object set -- omitting it would hand a receiver a signed RefState naming an
/// object they do not have, which fails their `verify` with a message naming exactly that.
fn resolve_ref_target_block(
    object_store: &impl ObjectReader,
    ref_state_payload: &RefStatePayload,
    tag_envelopes: &mut Vec<ObjectEnvelope>,
) -> Result<ObjectId> {
    match ref_state_payload.kind {
        RefKind::Branch => Ok(ref_state_payload.target_object_id),
        RefKind::Tag => {
            let tag_id = ref_state_payload.target_object_id;
            let tag_envelope = read_required(object_store, tag_id, ObjectType::Tag)?;
            let tag_payload = TagPayload::decode_canonical(&tag_envelope.canonical_payload)?;
            let target_block_id = tag_payload.target_block_id;
            tag_envelopes.push(tag_envelope);
            Ok(target_block_id)
        }
    }
}

fn encode_bundle(
    ref_name: &str,
    objects: &[ObjectEnvelope],
    author_keys: &[AuthorKeyEntry],
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(BUNDLE_MAGIC);
    push_bytes_u64(&mut out, ref_name.as_bytes())?;
    push_u64(&mut out, len_to_u64(objects.len())?);
    for envelope in objects {
        push_bytes_u64(&mut out, &encode_envelope_file(envelope)?)?;
    }
    // DC-53 Stage 2, D6: the author-key section, appended after the object list.
    push_u64(&mut out, len_to_u64(author_keys.len())?);
    for entry in author_keys {
        push_bytes_u64(&mut out, entry.key_id.as_bytes())?;
        out.extend_from_slice(&entry.public_key);
    }
    Ok(out)
}

/// Encode a `PBNDL001`-shaped bundle -- what a Stage-1-or-earlier build actually produced, and the
/// only thing `decode_bundle`'s `PBNDL001` acceptance needs to handle correctly. Test-only: no
/// production caller ever emits this format (`encode_bundle` above always writes `BUNDLE_MAGIC`).
/// Built from the same encoding primitives as `encode_bundle`, mirroring its pre-author-key-section
/// body exactly, rather than hand-editing bytes -- a hand-built fixture would prove the parser
/// accepts a byte shape, not that the real historical format actually decodes.
#[cfg(all(test, target_os = "linux"))]
fn encode_bundle_v1_for_test(ref_name: &str, objects: &[ObjectEnvelope]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(RETIRED_BUNDLE_MAGIC_V1);
    push_bytes_u64(&mut out, ref_name.as_bytes())?;
    push_u64(&mut out, len_to_u64(objects.len())?);
    for envelope in objects {
        push_bytes_u64(&mut out, &encode_envelope_file(envelope)?)?;
    }
    Ok(out)
}

fn decode_bundle(
    bytes: &[u8],
    max_object_count: usize,
) -> Result<(String, Vec<ObjectEnvelope>, Vec<AuthorKeyEntry>)> {
    let mut cursor = ByteCursor::new(bytes);
    let magic = cursor.read_array::<8>()?;
    // DC-53 Stage 2 follow-up (bundle-v1-import-regression-v1.md): `PBNDL001` is accepted here, not
    // refused. `layout.rs`'s own retired-format messages instruct a user to open an old repository
    // with an old prikk build and `bundle export` from it -- that build only ever emits `PBNDL001`,
    // and a current build refusing to import it severed the one migration path those messages
    // promise, in both directions at once. Export still only ever emits `BUNDLE_MAGIC`
    // (`PBNDL002`) -- this is read compatibility only, the same asymmetry every repository-format
    // transition in this project already has (read what the past wrote, write only the present).
    // A `PBNDL001` bundle is a `PBNDL002` bundle without the author-key section: not a special case,
    // since DC-53 already defines "no recorded material" as `Unverifiable` (vector 7), so decoding
    // one simply treats the author-key set as empty rather than reading a section that was never
    // written.
    let has_author_key_section = if &magic == BUNDLE_MAGIC {
        true
    } else if &magic == RETIRED_BUNDLE_MAGIC_V1 {
        false
    } else {
        return Err(PrikkError::MalformedData(
            "invalid bundle magic".to_string(),
        ));
    };
    let ref_name_bytes = cursor.read_bytes_u64()?;
    let ref_name = String::from_utf8(ref_name_bytes).map_err(|err| {
        PrikkError::MalformedData(format!("invalid bundle ref name utf-8: {err}"))
    })?;
    let count = cursor.read_u64()?;
    // DC-86: refused here, before the loop below decodes a single object — a declared count over
    // the limit must not cost more than reading one u64 to reject, regardless of how large `count`
    // claims to be or how much of `bytes` actually backs it.
    if count > len_to_u64(max_object_count)? {
        return Err(PrikkError::MalformedData(format!(
            "bundle declares {count} objects, over the configured limit of {max_object_count}"
        )));
    }
    let mut objects = Vec::new();
    for _ in 0..count {
        let encoded = cursor.read_bytes_u64()?;
        objects.push(decode_envelope_file(&encoded)?);
    }
    // DC-53 Stage 2, D6/C1 (plan review): the same declared-count bound DC-86 already applies to
    // the object count, applied here too -- a second declared count in a format a hostile sender
    // fully controls, with no bound of its own, would reopen the hole DC-86 closed. An
    // author-key entry can never legitimately outnumber the Patches in the same bundle, so reusing
    // `max_object_count` as the ceiling needs no new option surface.
    let mut author_keys = Vec::new();
    if has_author_key_section {
        let author_key_count = cursor.read_u64()?;
        if author_key_count > len_to_u64(max_object_count)? {
            return Err(PrikkError::MalformedData(format!(
                "bundle declares {author_key_count} author key entries, over the configured limit \
                 of {max_object_count}"
            )));
        }
        for _ in 0..author_key_count {
            let key_id_bytes = cursor.read_bytes_u64()?;
            let key_id = String::from_utf8(key_id_bytes).map_err(|err| {
                PrikkError::MalformedData(format!("invalid bundle author key_id utf-8: {err}"))
            })?;
            // DC-53 Stage 2, plan review: reuse `Signature::validate_key_id` rather than a second
            // notion of what a legal key id is -- it is the same rule these ids must satisfy to
            // ever match a signature's own `key_id`.
            Signature::validate_key_id(&key_id)?;
            let public_key = cursor.read_array::<32>()?;
            author_keys.push(AuthorKeyEntry { key_id, public_key });
        }
    }
    // A `PBNDL001` bundle's bytes end right after the object list -- no author-key section was ever
    // written, so there is nothing further to consume and this check still catches genuine trailing
    // garbage on either format.
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in bundle".to_string(),
        ));
    }
    Ok((ref_name, objects, author_keys))
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
