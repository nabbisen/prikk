//! `prikk branch` — list and create local branch refs.
//!
//! Branch creation already exists: `commit --ref heads/topic` followed by `seal --ref heads/topic`
//! creates an unborn branch as a signed Root block at `update_seq = 1` (DC-13 Non-Default Ref
//! Genesis). `branch create --from` must publish the *same* ref-state shape DC-13's genesis
//! publishes — maintainer-signed, `RefKind::Branch`, `update_seq = 1`,
//! `previous_ref_state_id = None` for a name with no surviving log — so a branch is
//! indistinguishable afterward regardless of which path created it. The only permitted
//! difference is the target: DC-13 seals a block it just created; this points at a block that
//! already exists.
//!
//! No `branch delete` and no current-branch pointer/`switch`: see the module-level non-goals in
//! the accepted RFC (DC-60). Deletion moved to DC-61, which needs a ref-log tombstone — a format
//! change DC-60's non-goals exclude. Every command still resolves `--ref` exactly as before.

use std::path::PathBuf;

use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType, RefKind, RefStatePayload,
    RefUpdatePayload,
};
use prikk_store::{
    DEFAULT_CHECKOUT_REF, FileObjectStore, MaintainerSigner, RefPublication, RefStore,
    maintainer_signature, validate_local_branch_ref,
};

/// Dispatch `prikk branch [list|create]`.
pub fn run_branch(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        None | Some("list") => run_list(root, iter.collect()),
        Some("create") => run_create(root, iter.collect()),
        Some(other) => Err(format!(
            "unknown branch subcommand: {other} (expected list or create)"
        )),
    }
}

fn run_list(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    if let Some(arg) = args.into_iter().next() {
        return Err(format!("unknown branch list argument: {arg}"));
    }
    let layout = crate::open_repository(root)?;
    let ref_store = RefStore::new(layout);
    let entries = ref_store
        .list_ref_pointers()
        .map_err(|err| err.to_string())?;
    if entries.is_empty() {
        println!("no branches");
    } else {
        for entry in entries {
            println!("{} {}", entry.ref_name, entry.ref_state_id);
        }
    }
    Ok(())
}

fn run_create(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let parsed = parse_create_args(args)?;
    let layout = crate::open_repository(root)?;
    layout
        .require_current_format()
        .map_err(|err| err.to_string())?;
    let canonical = validate_local_branch_ref(&parsed.name).map_err(|err| err.to_string())?;
    let ref_store = RefStore::new(layout.clone());
    let object_store = FileObjectStore::new(layout.clone());

    if ref_store
        .read_current_ref_state_id(&canonical)
        .map_err(|err| err.to_string())?
        .is_some()
    {
        return Err(format!("branch {canonical} already exists"));
    }

    // A ref log can survive an interrupted publication with no live pointer. Creating over it
    // would produce the "pointer absent, log present" state that DC-61 exists to resolve with a
    // ref-log tombstone; `publish` would refuse anyway, but fail closed here with a clear message
    // rather than a generic classification failure.
    if ref_store
        .recoverable_missing_ref(&canonical)
        .map_err(|err| err.to_string())?
        .is_some()
    {
        return Err(format!(
            "branch {canonical} has a surviving ref log with no live pointer; resuming it is not \
             yet supported (see DC-61), and creating over it would produce a corrupt state"
        ));
    }

    let from_ref = parsed
        .from
        .unwrap_or_else(|| DEFAULT_CHECKOUT_REF.to_string());
    let target_object_id = resolve_published_target(&ref_store, &object_store, &from_ref)?;

    let signer = crate::maintainer_signer_from_env()?;
    let ref_state_payload = RefStatePayload {
        ref_name: canonical.clone(),
        kind: RefKind::Branch,
        target_object_id,
        update_seq: 1,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
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
        new_target_object_id: target_object_id,
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
        .publish(&publication)
        .map_err(|err| err.to_string())?;

    println!("created branch {canonical}");
    println!("target block: {target_object_id}");
    println!("RefState: {published_ref_state_id}");
    println!("update_seq: 1");
    Ok(())
}

/// Resolve `--from`'s target block, requiring it to be a currently published ref.
fn resolve_published_target(
    ref_store: &RefStore,
    object_store: &FileObjectStore,
    from_ref: &str,
) -> std::result::Result<ObjectId, String> {
    let from_ref_state_id = ref_store
        .read_current_ref_state_id(from_ref)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("--from ref {from_ref} does not resolve to a published ref"))?;
    let from_envelope = object_store
        .read_typed(from_ref_state_id, ObjectType::RefState)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("--from ref {from_ref} RefState {from_ref_state_id} is missing"))?;
    let from_payload = RefStatePayload::decode_canonical(&from_envelope.canonical_payload)
        .map_err(|err| err.to_string())?;
    if from_payload.ref_name != from_ref {
        return Err(format!(
            "--from RefState name mismatch: expected {from_ref}, got {}",
            from_payload.ref_name
        ));
    }
    if object_store
        .read_typed(from_payload.target_object_id, ObjectType::Block)
        .map_err(|err| err.to_string())?
        .is_none()
    {
        return Err(format!(
            "--from ref {from_ref} targets missing block {}",
            from_payload.target_object_id
        ));
    }
    Ok(from_payload.target_object_id)
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
    from: Option<String>,
}

fn parse_create_args(args: Vec<String>) -> std::result::Result<CreateArgs, String> {
    let mut name = None;
    let mut from = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--from" => {
                let Some(value) = iter.next() else {
                    return Err("branch create --from requires a value".to_string());
                };
                from = Some(value);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown branch create argument: {other}"));
            }
            _ => {
                if name.is_some() {
                    return Err("branch create accepts at most one name".to_string());
                }
                name = Some(arg);
            }
        }
    }
    let Some(name) = name else {
        return Err("branch create requires <name>".to_string());
    };
    Ok(CreateArgs { name, from })
}
