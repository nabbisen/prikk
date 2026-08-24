//! DC-09 Phase 0 — canonical identity vector harness.
//!
//! Three deliberately separated test classes (DC-00 §8):
//!
//! 1. **hard FDD vectors** (`hard`) — normative, hand-pinned values from the
//!    ratified FDD-03. Never regenerated. The empty-PATCH anchor
//!    `510ab866…` (FDD-03 §4.1) lives here and is the regression floor for the
//!    whole reconciliation.
//! 2. **generated snapshots** (`snapshot`) — values produced by the current code
//!    over a fixed case set, diffed against committed `snapshot.txt`. Regenerate
//!    with `PRIKK_REGEN=1`; any change is reviewed. Generated snapshots never
//!    substitute for hard vectors.
//! 3. **negative validators** (`negative`) — rejection tests. Seeded here; later
//!    DC-09 phases (codec value_type, payload discriminators, node-id) extend it.
//!
//! The harness changes no identity bytes — it only observes them — so it is the
//! safe Phase 0 floor every later phase asserts against.

#![cfg(test)]

use prikk_hash::to_hex;

use crate::canonical::{CanonicalEncode, CanonicalWriter};
use crate::id::{ObjectId, ObjectType};

mod hard;
mod negative;
mod snapshot;

/// A canonical ObjectId case: a named `(object_type, schema_version, payload)`.
pub(crate) struct IdCase {
    /// Stable case name (snapshot key).
    pub name: &'static str,
    /// Object type.
    pub object_type: ObjectType,
    /// Schema version.
    pub schema_version: u32,
    /// Unsigned canonical payload bytes.
    pub payload: &'static [u8],
}

impl IdCase {
    fn id(&self) -> ObjectId {
        ObjectId::from_canonical_payload(self.object_type, self.schema_version, self.payload)
    }
}

/// Fixed case set the generated snapshot covers. Adding or removing a case is a
/// reviewed change that regenerates `snapshot.txt`.
pub(crate) const SNAPSHOT_CASES: &[IdCase] = &[
    IdCase {
        name: "empty_patch",
        object_type: ObjectType::Patch,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "patch_payload",
        object_type: ObjectType::Patch,
        schema_version: 1,
        payload: b"payload",
    },
    IdCase {
        name: "empty_block",
        object_type: ObjectType::Block,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_refstate",
        object_type: ObjectType::RefState,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "patch_schema2",
        object_type: ObjectType::Patch,
        schema_version: 2,
        payload: b"payload",
    },
    // Renumbered + new types (FDD-03 §3) — empty payloads isolate the type code.
    IdCase {
        name: "empty_refupdate",
        object_type: ObjectType::RefUpdate,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_tag",
        object_type: ObjectType::Tag,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_attestation",
        object_type: ObjectType::Attestation,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_blob",
        object_type: ObjectType::Blob,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_block_summary_cache",
        object_type: ObjectType::BlockSummaryCache,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_recovery_note",
        object_type: ObjectType::RecoveryNote,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_project_genesis",
        object_type: ObjectType::ProjectGenesis,
        schema_version: 1,
        payload: b"",
    },
    IdCase {
        name: "empty_recognition_claim",
        object_type: ObjectType::RecognitionClaim,
        schema_version: 1,
        payload: b"",
    },
];

/// Populated Block payload exercising the DC-09 Phase 3 granular value types
/// (`enum_u16` kind, `object_id` parent/patch/snapshot references) plus a raw
/// 32-byte `bytes` merkle root. Empty payloads cannot witness these.
/// Populated Blob payload (FDD-03 §9.11): `enum_u16` blob_kind, `bytes` content,
/// `u64` declared_size.
// infallible: declared_size matches content length via `new`.
#[allow(clippy::expect_used)]
pub(crate) fn blob_populated_payload() -> Vec<u8> {
    let blob = crate::BlobPayload::new(crate::BlobKind::Binary, vec![0xAB, 0xCD, 0xEF]);
    blob.to_canonical_bytes().expect("blob encodes")
}

/// Populated RecognitionClaim payload (RFC 115 Stage 2, D3): a real `block_id` plus two sorted,
/// distinct `patch_ids`, so the repeated `object_id` framing (tag 2, emitted twice) is witnessed,
/// not just the single-`block_id` field an empty-`patch_ids` case could not have (and the type
/// refuses anyway -- `patch_ids` must be non-empty). `parent_block_ids` empty -- a root block has
/// none, and RFC 116 N3's own amendment requires this frozen vector's bytes not move: an empty
/// repeated field (tag 3) writes nothing, so this case is unchanged from before N3 landed.
// infallible: patch_ids sorted and non-empty.
#[allow(clippy::expect_used)]
pub(crate) fn recognition_claim_populated_payload() -> Vec<u8> {
    let claim = crate::RecognitionClaimPayload {
        block_id: ObjectId::from_bytes([0x11; 32]),
        patch_ids: vec![
            ObjectId::from_bytes([0x22; 32]),
            ObjectId::from_bytes([0x33; 32]),
        ],
        parent_block_ids: Vec::new(),
    };
    claim
        .to_canonical_bytes()
        .expect("recognition claim encodes")
}

/// A second RecognitionClaim vector (RFC 116 design-v1.md §3, N3), witnessing the new
/// `parent_block_ids` field (tag 3) with real content -- the existing `recognition_claim_populated`
/// case above stays empty-parented on purpose, so this is the only vector that actually exercises
/// tag 3's repeated `object_id` framing.
// infallible: patch_ids and parent_block_ids both sorted and non-empty.
#[allow(clippy::expect_used)]
pub(crate) fn recognition_claim_with_parents_payload() -> Vec<u8> {
    let claim = crate::RecognitionClaimPayload {
        block_id: ObjectId::from_bytes([0x44; 32]),
        patch_ids: vec![ObjectId::from_bytes([0x55; 32])],
        parent_block_ids: vec![
            ObjectId::from_bytes([0x66; 32]),
            ObjectId::from_bytes([0x77; 32]),
        ],
    };
    claim
        .to_canonical_bytes()
        .expect("recognition claim with parents encodes")
}

/// Populated Patch payload carrying one operation, so the `operations` list
/// framing is witnessable. An empty patch has no operations and cannot show the
/// `record_list_item` (0x21) item type required by FDD-03 §9.1 tag 1. The inner
/// operation is a §9.3 `CreateFile` record (repo_path, node_id, object_id, mode).
// infallible: single op, op_seq starts at 1, no parents/intent/preconditions.
#[allow(clippy::expect_used)]
pub(crate) fn patch_operations_populated_payload() -> Vec<u8> {
    let patch = crate::PatchPayload {
        operations: vec![crate::Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: crate::OperationKind::CreateFile(crate::CreateFile {
                path: "a.txt".to_string(),
                node_id: crate::NodeId::from_bytes([0x22; 32]),
                blob_id: ObjectId::from_bytes([0x11; 32]),
                mode: 0o100_644,
            }),
        }],
        intent: None,
        preconditions: Vec::new(),
        purpose: crate::PatchPurpose::Normal,
    };
    patch.to_canonical_bytes().expect("patch encodes")
}

// infallible: sorted unique ids, valid kind.
#[allow(clippy::expect_used)]
pub(crate) fn block_populated_payload() -> Vec<u8> {
    let block = crate::BlockPayload {
        parent_block_ids: vec![ObjectId::from_bytes([0x11; 32])],
        kind: crate::BlockKind::Normal,
        patch_ids: vec![
            ObjectId::from_bytes([0x22; 32]),
            ObjectId::from_bytes([0x33; 32]),
        ],
        state_merkle_root: crate::MerkleRoot([0x44; 32]),
        snapshot_blob_ref: Some(ObjectId::from_bytes([0x55; 32])),
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    block.to_canonical_bytes().expect("block encodes")
}

/// Populated RefState payload exercising `enum_u16` kind, `object_id` references,
/// a `utf8` ref name, and a `u64` sequence.
// infallible: sorted unique attestation ids, valid kind.
#[allow(clippy::expect_used)]
pub(crate) fn refs_populated_payload() -> Vec<u8> {
    let refstate = crate::RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: crate::RefKind::Branch,
        target_object_id: ObjectId::from_bytes([0x66; 32]),
        update_seq: 7,
        previous_ref_state_id: Some(ObjectId::from_bytes([0x77; 32])),
        required_attestation_ids: vec![ObjectId::from_bytes([0x88; 32])],
        closed: false,
    };
    refstate.to_canonical_bytes().expect("refstate encodes")
}

/// Populated, **closed** RefState payload (DC-61): the same case as
/// `refs_populated_payload`, plus the closed marker (tag 7, `bool`). New pinned identity for the
/// schema-2 wire shape, alongside the existing schema-1 `refs_populated` vector — the schema bump's
/// one accounted-for identity addition (see `handoffs/DC-61-branch-closure/`).
// infallible: sorted unique attestation ids, valid kind.
#[allow(clippy::expect_used)]
pub(crate) fn refs_closed_payload() -> Vec<u8> {
    let refstate = crate::RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: crate::RefKind::Branch,
        target_object_id: ObjectId::from_bytes([0x66; 32]),
        update_seq: 7,
        previous_ref_state_id: Some(ObjectId::from_bytes([0x77; 32])),
        required_attestation_ids: vec![ObjectId::from_bytes([0x88; 32])],
        closed: true,
    };
    refstate
        .to_canonical_bytes()
        .expect("closed refstate encodes")
}

/// Populated Attestation payload exercising FDD-03 §9.9 field tags: `object_id`
/// target, `utf8` policy version, `bytes` plugin-set hash, a `record_list_item`
/// results list (one §9.10 entry), `enum_u16` status, `u64`, `bool`.
// infallible: single sorted result, valid status.
#[allow(clippy::expect_used)]
pub(crate) fn attestation_populated_payload() -> Vec<u8> {
    let att = crate::AttestationPayload {
        target_block_id: ObjectId::from_bytes([0x11; 32]),
        policy_version: "v1".to_string(),
        plugin_set_hash: vec![0x22; 32],
        results: vec![crate::PluginResultEntry {
            plugin_id: "audit-secrets".to_string(),
            plugin_version: "0.1".to_string(),
            status: crate::AttestationStatus::Pass,
            report_hash: vec![0x33; 32],
            finding_count: 0,
        }],
        status: crate::AttestationStatus::Pass,
        created_at: 1_700_000_000,
        is_reproducible_offline: true,
    };
    att.to_canonical_bytes().expect("attestation encodes")
}

/// A codec-exercising sample payload covering several FDD-03 §7.1 value types
/// (`bool`, `u16`, `u32`, `utf8`, `bytes`). Hashing it captures any change to the
/// value_type code points — the empty-payload anchor cannot, since it has no
/// field records.
// infallible: ascending nonzero tags and valid values.
#[allow(clippy::expect_used)]
pub(crate) fn codec_sample_payload() -> Vec<u8> {
    let mut w = CanonicalWriter::new();
    w.field_bool(1, true).expect("bool");
    w.field_u16(2, 0x0102).expect("u16");
    w.field_u32(3, 7).expect("u32");
    w.field_string(4, "hi").expect("string");
    w.field_bytes(5, &[0xAB, 0xCD]).expect("bytes");
    w.finish()
}

/// Deterministic snapshot text: one `name|object_type_code|schema|payload_hex|id_hex`
/// line per case, in `SNAPSHOT_CASES` order, then the codec sample.
pub(crate) fn generate_snapshot() -> String {
    let mut out = String::new();
    out.push_str("# prikk-object identity snapshot — generated; regenerate with PRIKK_REGEN=1\n");
    out.push_str("# columns: name|object_type_code|schema_version|payload_hex|object_id_hex\n");
    for case in SNAPSHOT_CASES {
        out.push_str(&format!(
            "{}|{}|{}|{}|{}\n",
            case.name,
            case.object_type.code(),
            case.schema_version,
            to_hex(case.payload),
            case.id().to_hex(),
        ));
    }
    let codec = codec_sample_payload();
    let codec_id = ObjectId::from_canonical_payload(ObjectType::Patch, 1, &codec);
    out.push_str(&format!(
        "codec_sample|{}|1|{}|{}\n",
        ObjectType::Patch.code(),
        to_hex(&codec),
        codec_id.to_hex(),
    ));
    let block_p = block_populated_payload();
    let block_id = ObjectId::from_canonical_payload(ObjectType::Block, 1, &block_p);
    out.push_str(&format!(
        "block_populated|{}|1|{}|{}\n",
        ObjectType::Block.code(),
        to_hex(&block_p),
        block_id.to_hex(),
    ));
    let refs_p = refs_populated_payload();
    let refs_id = ObjectId::from_canonical_payload(ObjectType::RefState, 1, &refs_p);
    out.push_str(&format!(
        "refs_populated|{}|1|{}|{}\n",
        ObjectType::RefState.code(),
        to_hex(&refs_p),
        refs_id.to_hex(),
    ));
    let refs_closed_p = refs_closed_payload();
    let refs_closed_id = ObjectId::from_canonical_payload(ObjectType::RefState, 2, &refs_closed_p);
    out.push_str(&format!(
        "refs_closed|{}|2|{}|{}\n",
        ObjectType::RefState.code(),
        to_hex(&refs_closed_p),
        refs_closed_id.to_hex(),
    ));
    let att_p = attestation_populated_payload();
    let att_id = ObjectId::from_canonical_payload(ObjectType::Attestation, 1, &att_p);
    out.push_str(&format!(
        "attestation_populated|{}|1|{}|{}\n",
        ObjectType::Attestation.code(),
        to_hex(&att_p),
        att_id.to_hex(),
    ));
    let blob_p = blob_populated_payload();
    let blob_id = ObjectId::from_canonical_payload(ObjectType::Blob, 1, &blob_p);
    out.push_str(&format!(
        "blob_populated|{}|1|{}|{}\n",
        ObjectType::Blob.code(),
        to_hex(&blob_p),
        blob_id.to_hex(),
    ));
    let recognition_claim_p = recognition_claim_populated_payload();
    let recognition_claim_id =
        ObjectId::from_canonical_payload(ObjectType::RecognitionClaim, 1, &recognition_claim_p);
    out.push_str(&format!(
        "recognition_claim_populated|{}|1|{}|{}\n",
        ObjectType::RecognitionClaim.code(),
        to_hex(&recognition_claim_p),
        recognition_claim_id.to_hex(),
    ));
    let recognition_claim_with_parents_p = recognition_claim_with_parents_payload();
    let recognition_claim_with_parents_id = ObjectId::from_canonical_payload(
        ObjectType::RecognitionClaim,
        1,
        &recognition_claim_with_parents_p,
    );
    out.push_str(&format!(
        "recognition_claim_with_parents|{}|1|{}|{}\n",
        ObjectType::RecognitionClaim.code(),
        to_hex(&recognition_claim_with_parents_p),
        recognition_claim_with_parents_id.to_hex(),
    ));
    out
}
