//! `prikk tag` — create and list tag refs (§6.6).
//!
//! Tags are immutable tag objects; a tag ref must point at the tag object, never directly at a
//! block — two hops: ref -> tag object -> block. Tag objects and tag ref-states are both
//! maintainer-signed, on the same terms as `seal`/`branch create`.
//!
//! `created_at` is the canonical no-clock sentinel (DC-34's "RefUpdate time policy", which
//! `TagPayload`'s corrected doc comment now also states): always zero, never derived from a wall
//! clock or accepted from a `--date` flag.
//!
//! Ref-name validation reuses `validate_local_tag_ref` (`prikk-store/src/refs.rs`), added
//! alongside the kind-aware branch in `validate_coherent_publication` — v1's AC4 finding that no
//! tag-appropriate validator existed. It mirrors `validate_local_branch_ref` with the prefix
//! requirement inverted and carries no case-collision rule, matching branches.

use std::path::PathBuf;

use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType, RefKind, RefStatePayload,
    RefUpdatePayload, TagPayload,
};
use prikk_store::{
    FileObjectStore, MaintainerSigner, ObjectReader, ObjectWriteSession, ObjectWriter,
    RefPublication, RefStore, maintainer_signature, validate_local_tag_ref,
};

/// Dispatch `prikk tag [list|create]`.
pub fn run_tag(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        None | Some("list") => run_list(root, iter.collect()),
        Some("create") => run_create(root, iter.collect()),
        Some(other) => Err(format!(
            "unknown tag subcommand: {other} (expected list or create)"
        )),
    }
}

fn run_list(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    if let Some(arg) = args.into_iter().next() {
        return Err(format!("unknown tag list argument: {arg}"));
    }
    let layout = crate::open_repository(root)?;
    let ref_store = RefStore::new(layout.clone());
    let object_store = FileObjectStore::new(layout);
    let entries = ref_store
        .list_ref_pointers()
        .map_err(|err| err.to_string())?;
    let mut printed_any = false;
    for entry in entries {
        let ref_state_envelope = object_store
            .read_typed(entry.ref_state_id, ObjectType::RefState)
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!(
                    "ref {} RefState {} is missing",
                    entry.ref_name, entry.ref_state_id
                )
            })?;
        let ref_state_payload = RefStatePayload::decode_canonical(
            &ref_state_envelope.canonical_payload,
            ref_state_envelope.schema_version,
        )
        .map_err(|err| err.to_string())?;
        if ref_state_payload.kind != RefKind::Tag {
            continue;
        }
        let tag_envelope = object_store
            .read_typed(ref_state_payload.target_object_id, ObjectType::Tag)
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!(
                    "tag ref {} targets missing tag object {}",
                    entry.ref_name, ref_state_payload.target_object_id
                )
            })?;
        let tag_payload = TagPayload::decode_canonical(&tag_envelope.canonical_payload)
            .map_err(|err| err.to_string())?;
        println!("{} {}", entry.ref_name, tag_payload.target_block_id);
        printed_any = true;
    }
    if !printed_any {
        println!("no tags");
    }
    Ok(())
}

fn run_create(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let parsed = parse_create_args(args)?;
    let layout = crate::open_repository(root)?;
    layout
        .require_current_format()
        .map_err(|err| err.to_string())?;
    let canonical = validate_local_tag_ref(&parsed.name).map_err(|err| err.to_string())?;
    let ref_store = RefStore::new(layout.clone());
    let mut object_store = ObjectWriteSession::open(&layout).map_err(|err| err.to_string())?;

    if ref_store
        .read_current_ref_state_id(&canonical)
        .map_err(|err| err.to_string())?
        .is_some()
    {
        return Err(format!("tag {canonical} already exists"));
    }

    let target_block_id = resolve_target_block(&ref_store, &object_store, &parsed.target)?;

    let signer = crate::maintainer_signer_from_env()?;
    let tag_payload = TagPayload {
        name: canonical.clone(),
        target_block_id,
        message: parsed.message,
        created_at: 0,
        author_key_id: signer.key_id().to_string(),
    };
    let tag_envelope = signed_envelope(
        ObjectType::Tag,
        1,
        tag_payload
            .to_canonical_bytes()
            .map_err(|err| err.to_string())?,
        &signer,
    )?;
    let tag_object_id = object_store
        .write_object(&tag_envelope)
        .map_err(|err| err.to_string())?;

    let ref_state_payload = RefStatePayload {
        ref_name: canonical.clone(),
        kind: RefKind::Tag,
        target_object_id: tag_object_id,
        update_seq: 1,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let ref_state_envelope = signed_envelope(
        ObjectType::RefState,
        1,
        ref_state_payload
            .to_canonical_bytes()
            .map_err(|err| err.to_string())?,
        &signer,
    )?;
    let ref_state_id = ref_state_envelope.object_id();
    let ref_update_payload = RefUpdatePayload {
        ref_name: canonical.clone(),
        old_ref_state_id: None,
        new_ref_state_id: ref_state_id,
        new_target_object_id: tag_object_id,
        update_seq: 1,
        created_at: 0,
        author_key_id: signer.key_id().to_string(),
    };
    let ref_update_envelope = signed_envelope(
        ObjectType::RefUpdate,
        1,
        ref_update_payload
            .to_canonical_bytes()
            .map_err(|err| err.to_string())?,
        &signer,
    )?;
    let publication = RefPublication {
        ref_name: canonical.clone(),
        expected_previous_ref_state_id: None,
        ref_state: ref_state_envelope,
        ref_update: ref_update_envelope,
    };
    let published_ref_state_id = ref_store
        .publish_with_object_store(&mut object_store, &publication)
        .map_err(|err| err.to_string())?;

    println!("created tag {canonical}");
    println!("tag object: {tag_object_id}");
    println!("target block: {target_block_id}");
    println!("RefState: {published_ref_state_id}");
    Ok(())
}

/// Resolve `--target`'s block, accepting either a raw 64-hex block id or a ref name whose current
/// target is a block.
fn resolve_target_block(
    ref_store: &RefStore,
    object_store: &impl ObjectReader,
    target: &str,
) -> std::result::Result<ObjectId, String> {
    if let Ok(block_id) = target.parse::<ObjectId>() {
        return object_store
            .read_typed(block_id, ObjectType::Block)
            .map_err(|err| err.to_string())?
            .map(|_| block_id)
            .ok_or_else(|| format!("--target block {block_id} does not exist"));
    }
    let target_ref_state_id = ref_store
        .read_current_ref_state_id(target)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("--target ref {target} does not resolve to a published ref"))?;
    let target_envelope = object_store
        .read_typed(target_ref_state_id, ObjectType::RefState)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| {
            format!("--target ref {target} RefState {target_ref_state_id} is missing")
        })?;
    let target_payload = RefStatePayload::decode_canonical(
        &target_envelope.canonical_payload,
        target_envelope.schema_version,
    )
    .map_err(|err| err.to_string())?;
    if target_payload.ref_name != target {
        return Err(format!(
            "--target RefState name mismatch: expected {target}, got {}",
            target_payload.ref_name
        ));
    }
    if object_store
        .read_typed(target_payload.target_object_id, ObjectType::Block)
        .map_err(|err| err.to_string())?
        .is_none()
    {
        return Err(format!(
            "--target ref {target} targets missing block {}",
            target_payload.target_object_id
        ));
    }
    Ok(target_payload.target_object_id)
}

fn signed_envelope(
    object_type: ObjectType,
    schema_version: u32,
    canonical_payload: Vec<u8>,
    signer: &impl MaintainerSigner,
) -> std::result::Result<ObjectEnvelope, String> {
    let mut envelope = ObjectEnvelope::unsigned(object_type, schema_version, canonical_payload);
    let object_id = envelope.object_id();
    envelope
        .add_signature(
            maintainer_signature(signer, object_type, object_id).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
    Ok(envelope)
}

struct CreateArgs {
    name: String,
    target: String,
    message: Option<String>,
}

fn parse_create_args(args: Vec<String>) -> std::result::Result<CreateArgs, String> {
    let mut name = None;
    let mut target = None;
    let mut message = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--target" => {
                let Some(value) = iter.next() else {
                    return Err("tag create --target requires a value".to_string());
                };
                target = Some(value);
            }
            "-m" | "--message" => {
                let Some(value) = iter.next() else {
                    return Err("tag create message option requires a value".to_string());
                };
                message = Some(value);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown tag create argument: {other}"));
            }
            _ => {
                if name.is_some() {
                    return Err("tag create accepts at most one name".to_string());
                }
                name = Some(arg);
            }
        }
    }
    let Some(name) = name else {
        return Err("tag create requires <name>".to_string());
    };
    let Some(target) = target else {
        return Err("tag create requires --target <ref|block>".to_string());
    };
    if let Some(msg) = &message {
        if msg.trim().is_empty() {
            return Err("tag message must not be empty".to_string());
        }
    }
    Ok(CreateArgs {
        name,
        target,
        message,
    })
}
