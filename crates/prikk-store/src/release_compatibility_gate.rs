//! RFC 119 track C — Gate G1, pre-1.0 form: a release does not *silently* break what earlier
//! releases wrote.
//!
//! **This file is `format_stability_gate.rs`'s missing sibling** (RFC 119 §10 track C, §2 of the
//! handoff): that gate guards *format version bumps*; `0.23.0`'s `Tag` break went through the hole
//! beside it -- an in-place payload amendment at the *same* schema version, for which no format
//! bump occurs, so Gate B never fires. Same three-layer shape, same discipline ("a gate is only
//! trusted once it has been observed failing," `rfc111_index_decode_cost_gate.rs`), same
//! frozen-fixture precedent (`crates/prikk-cli/tests/fixtures/dc55_pre_swap_repo`).
//!
//! **The guarantee is against silence, not against breaking.** Breaking is permitted when
//! authorized and declared -- `0.23.0`'s own `Tag` amendment was exactly that. The outcome per
//! persisted object type is ternary:
//! - **compatible** -- decodes cleanly under current code;
//! - **breaking, declared** -- does not, and [`DECLARED_BREAKS`] names it, with a reason and a
//!   remedy;
//! - **breaking, undeclared** -- the only failure.
//!
//! **Fixture**: `crates/prikk-cli/tests/fixtures/rfc119_g1_0_23_0_repo`, a real repository written
//! by the real `0.23.0` binary (built from the `0.23.0` git tag in an isolated worktree, never from
//! this working tree). **Do not regenerate it.** See the RFC 119 track C report for exactly how it
//! was produced and what it contains.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use prikk_object::{BlobPayload, BlockPayload, ObjectType, RecognitionClaimPayload, TagPayload};

use crate::container::decode_container_records;
use crate::fsutil::read_file_if_exists;
use crate::layout::{ContainerSlot, RepositoryLayout, persisted_object_types};
use crate::patch_replay::decode::decode_patch_operations;

/// One declared compatibility break between two adjacent releases -- the shape of Gate A's
/// `frozen`/`RFC114_ADMITTED_BUT_UNWRITTEN` pair and `format_stability_gate.rs`'s
/// `FORMATS_WITH_MIGRATION_COVERAGE`: a committed list, never satisfiable by editing alone (the
/// object type must be real, checked below), always carrying a reason and a remedy.
struct DeclaredBreak {
    /// The two releases involved, older first.
    version_pair: &'static str,
    /// Which persisted object type stops decoding.
    object_type: ObjectType,
    /// Quoted from `CHANGELOG.md`, not re-derived -- the wording is the record.
    reason: &'static str,
    /// A remedy, or an explicit statement that none exists. An entry with an empty remedy is a
    /// documentation defect, not a passing gate (handoff §5) -- there is no way to express "empty"
    /// here other than writing the true sentence, which this one does.
    remedy: &'static str,
}

/// Seeded with `0.23.0`'s own `Tag` break (RFC 119 track C handoff §5). **Not currently exercised
/// by [`g1_last_release_fixture_is_compatible_or_the_break_is_declared`]**: that test compares the
/// `0.23.0`-vintage fixture against *current* code, and nothing has changed `Tag` handling since
/// `0.23.0` shipped, so the fixture is, correctly, still fully compatible today (see the report's
/// own note on this). This entry is the historical record of the break that motivated building this
/// gate at all, carried forward the same way `format_stability_gate.rs` starts with an empty-but-
/// ready `FORMATS_WITH_MIGRATION_COVERAGE` -- it becomes load-bearing the day a *future* release
/// breaks `Tag` again, or if this gate's own fixture is ever rebuilt from a pre-`0.23.0` tag.
const DECLARED_BREAKS: &[DeclaredBreak] = &[DeclaredBreak {
    version_pair: "0.22.1 -> 0.23.0",
    object_type: ObjectType::Tag,
    reason: "TagPayload gained two fields -- patch_set_digest and patch_count (RFC 117) -- added \
             in place at schema_version 1, not as a new schema version. 0.23.0 reading a 0.22.1 tag \
             fails with `Tag missing patch_set_digest`; 0.22.1 reading a 0.23.0 tag fails with \
             `unknown Tag field tag: 6` (CHANGELOG.md, 0.23.0 \"Breaking change\").",
    remedy: "A repository written by 0.22.1 that already holds a tag cannot be repaired under \
             0.23.0 -- there is no `prikk tag delete`, and `prikk tag create` refuses when a tag \
             ref of that name already exists. Keep using 0.22.1 for that repository, or start a \
             fresh repository under 0.23.0. There is no in-place remediation today (CHANGELOG.md, \
             0.23.0 \"Breaking change\").",
}];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("prikk-store's manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// The frozen last-release fixture's root (the directory *containing* `.prikk`, matching
/// `RepositoryLayout::open`'s own expectation).
fn last_release_fixture_root() -> PathBuf {
    repo_root().join("crates/prikk-cli/tests/fixtures/rfc119_g1_0_23_0_repo")
}

/// Read one persisted object type's live (slot A -- compaction never runs on object containers,
/// per `ContainerSlot`'s own doc) container file and decode every record with the *same* decoder
/// each type's real production read path uses -- `Patch` via `decode_patch_operations` (the real
/// general decoder, patch-schema-2 handoff), everything else via its own `XPayload::decode_canonical`.
/// Returns the number of records that decoded cleanly, or the first decode failure's message.
fn check_type_decodes(layout: &RepositoryLayout, object_type: ObjectType) -> Result<usize, String> {
    let container_path = layout.container_slot_path(object_type, ContainerSlot::A);
    let relative = layout
        .repository_relative(&container_path)
        .map_err(|err| err.to_string())?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &relative)
        .map_err(|err| err.to_string())?
    else {
        return Ok(0);
    };
    let replay = decode_container_records(object_type, &bytes).map_err(|err| err.to_string())?;
    let mut checked = 0_usize;
    for record in &replay.records {
        let envelope = &record.envelope;
        let decode_result: Result<(), prikk_error::PrikkError> = match object_type {
            ObjectType::Block => {
                BlockPayload::decode_canonical(&envelope.canonical_payload).map(|_| ())
            }
            ObjectType::Blob => {
                BlobPayload::decode_canonical(&envelope.canonical_payload).map(|_| ())
            }
            ObjectType::Tag => {
                TagPayload::decode_canonical(&envelope.canonical_payload).map(|_| ())
            }
            ObjectType::RefState => prikk_object::RefStatePayload::decode_canonical(
                &envelope.canonical_payload,
                envelope.schema_version,
            )
            .map(|_| ()),
            ObjectType::RecognitionClaim => {
                RecognitionClaimPayload::decode_canonical(&envelope.canonical_payload).map(|_| ())
            }
            ObjectType::Patch => {
                decode_patch_operations(&envelope.canonical_payload, envelope.schema_version)
                    .map(|_| ())
            }
            other => {
                panic!(
                    "G1 has no decode check wired up for persisted type {other} -- \
                     persisted_object_types() grew without this gate growing with it"
                )
            }
        };
        decode_result.map_err(|err| {
            format!(
                "object {} ({object_type}) failed to decode: {err}",
                envelope.object_id()
            )
        })?;
        checked += 1;
    }
    Ok(checked)
}

#[test]
fn last_release_fixture_exists() {
    let root = last_release_fixture_root();
    assert!(
        root.join(".prikk").is_dir(),
        "the last-release compatibility fixture is missing at {} -- see this module's doc for how \
         it must be rebuilt (from the actual release tag's binary, never from current code)",
        root.display()
    );
}

/// Layer 1 (mirroring `format_stability_gate.rs`'s own layer 2): every declared break must name a
/// real persisted object type, so a typo or a stale entry cannot silently exempt nothing.
#[test]
fn every_declared_break_names_a_persisted_object_type() {
    for declared in DECLARED_BREAKS {
        assert!(
            persisted_object_types().contains(&declared.object_type),
            "DECLARED_BREAKS entry for {} names {}, which is not a persisted object type",
            declared.version_pair,
            declared.object_type
        );
        assert!(
            !declared.reason.trim().is_empty(),
            "DECLARED_BREAKS entry for {} ({}) has no reason",
            declared.version_pair,
            declared.object_type
        );
        assert!(
            !declared.remedy.trim().is_empty(),
            "DECLARED_BREAKS entry for {} ({}) has no remedy",
            declared.version_pair,
            declared.object_type
        );
    }
}

/// The real conformance check: every persisted object type in the last-release fixture must either
/// decode cleanly under current code, or have its failure covered by [`DECLARED_BREAKS`]. This is
/// the test the four controls exercise (RFC 119 track C handoff §7); it currently passes because
/// nothing has changed any persisted type's decode contract since `0.23.0` shipped -- the same
/// "nothing to test yet" state `format_stability_gate.rs`'s own layers 1/2 start in.
#[test]
fn g1_last_release_fixture_is_compatible_or_the_break_is_declared() {
    let root = last_release_fixture_root();
    let layout = RepositoryLayout::open(&root).expect("last-release fixture repository opens");
    for &object_type in &persisted_object_types() {
        if let Err(message) = check_type_decodes(&layout, object_type) {
            let declared = DECLARED_BREAKS
                .iter()
                .any(|break_| break_.object_type == object_type);
            assert!(
                declared,
                "undeclared compatibility break: {message}, against the last-release fixture at \
                 {} -- if this break is authorized, add a DECLARED_BREAKS entry with a reason and a \
                 remedy; if not, this is a live defect",
                root.display()
            );
        }
    }
}

/// Coverage is the gate's real specification (handoff §4/§8): committed literal counts per
/// persisted object type, so removing coverage is caught here, distinctly from a decode failure --
/// the same "committed, never generated at test time" discipline `dc55_identity_evidence.rs`'s own
/// `every_frozen_object_id_matches_its_own_filename` count uses.
#[test]
fn last_release_fixture_coverage_matches_the_committed_counts() {
    let root = last_release_fixture_root();
    let layout = RepositoryLayout::open(&root).expect("last-release fixture repository opens");
    let expected: &[(ObjectType, usize)] = &[
        (ObjectType::Patch, 2),
        (ObjectType::Block, 1),
        (ObjectType::Blob, 2),
        (ObjectType::RefState, 2),
        (ObjectType::Tag, 1),
        (ObjectType::RecognitionClaim, 1),
        (ObjectType::Attestation, 0),
    ];
    for &(object_type, expected_count) in expected {
        let actual = check_type_decodes(&layout, object_type)
            .unwrap_or_else(|err| panic!("{object_type} must decode cleanly here: {err}"));
        assert_eq!(
            actual, expected_count,
            "{object_type}: expected {expected_count} persisted, found {actual} -- update this \
             count deliberately if the fixture is ever legitimately rebuilt, never to paper over a \
             coverage regression"
        );
    }
}
