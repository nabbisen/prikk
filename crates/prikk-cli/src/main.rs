#![forbid(unsafe_code)]

//! PRIKK command-line entry point.
//!
//! PR-008 exposes minimal repository layout commands, active WAL status, an empty-commit scaffold,
//! ref pointer counts, and read-only repository verification. The commit command deliberately
//! appends a signed patch envelope to the active WAL only; seal, patch application, and real diff
//! capture remain later increments.

use std::path::PathBuf;
use std::process::ExitCode;

use prikk_hash::sha256;
use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectType, OperationCondition, OperationConditionEntry,
    PatchPayload, Signature, SignatureAlgorithm, SignerRole,
};
use prikk_store::{verify_repository, ActiveSession, RefStore, RepositoryLayout, Wal};

const VERSION: &str = "0.1.0-pr008";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::from(1)
        }
    }
}

fn run() -> std::result::Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some("--version") | Some("-V") => {
            println!("prikk {VERSION}");
            Ok(())
        }
        Some("init") => {
            let root = match args.next() {
                Some(path) => PathBuf::from(path),
                None => current_dir()?,
            };
            RepositoryLayout::init(root.clone()).map_err(|err| err.to_string())?;
            println!("initialized PRIKK repository at {}", root.join(".prikk").display());
            Ok(())
        }
        Some("commit") => {
            let message = parse_empty_commit_message(args.collect())?;
            let root = current_dir()?;
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let envelope = empty_patch_envelope(&message)?;
            let patch_id = envelope.object_id();
            let session = ActiveSession::new(layout);
            let result = session.append_patch(&envelope).map_err(|err| err.to_string())?;
            println!("recorded empty patch in active WAL");
            println!("patch id: {patch_id}");
            println!("WAL sequence: {}", result.wal_sequence);
            println!("note: real diff capture and seal remain later PRs");
            Ok(())
        }
        Some("status") => {
            let root = current_dir()?;
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let wal = Wal::new(layout.default_queue_wal_path());
            let replay = wal.replay().map_err(|err| err.to_string())?;
            println!("prikk repository: {}", layout.prikk_dir().display());
            let ref_store = RefStore::new(layout.clone());
            let main_ref = ref_store
                .read_current_ref_state_id("heads/main")
                .map_err(|err| err.to_string())?;
            println!("active WAL records: {}", replay.records.len());
            println!("trailing partial WAL bytes: {}", replay.trailing_partial_bytes);
            match main_ref {
                Some(id) => println!("heads/main RefState: {id}"),
                None => println!("heads/main RefState: <not published>"),
            }
            println!("status: seal, patch algebra, plugins, and sync not implemented in PR-008");
            Ok(())
        }
        Some("verify") => {
            let root = match args.next() {
                Some(path) => PathBuf::from(path),
                None => current_dir()?,
            };
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let report = verify_repository(&layout).map_err(|err| err.to_string())?;
            println!("verified repository: {}", layout.prikk_dir().display());
            println!("checked objects: {}", report.checked_objects);
            println!("checked WAL records: {}", report.checked_wal_records);
            println!("checked refs: {}", report.checked_refs);
            println!("checked ref-log records: {}", report.checked_ref_log_records);
            println!("trailing partial WAL bytes: {}", report.trailing_partial_wal_bytes);
            if report.has_trailing_partial_wal() {
                println!("warning: active WAL contains an incomplete trailing record");
            }
            Ok(())
        }
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

fn parse_empty_commit_message(args: Vec<String>) -> std::result::Result<String, String> {
    let mut allow_empty = false;
    let mut message = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--allow-empty" => allow_empty = true,
            "-m" | "--message" => {
                let Some(value) = iter.next() else {
                    return Err("commit message option requires a value".to_string());
                };
                message = Some(value);
            }
            other => return Err(format!("unknown commit argument: {other}")),
        }
    }
    if !allow_empty {
        return Err("PR-008 supports only `prikk commit --allow-empty -m <message>`".to_string());
    }
    let Some(message) = message else {
        return Err("empty commit requires -m <message>".to_string());
    };
    if message.trim().is_empty() {
        return Err("commit message must not be empty".to_string());
    }
    Ok(message)
}

fn empty_patch_envelope(message: &str) -> std::result::Result<ObjectEnvelope, String> {
    let message_hash = sha256(message.as_bytes());
    let payload = PatchPayload {
        operations: Vec::new(),
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: vec![OperationConditionEntry {
            key: "prikk.dev.empty-commit-message-sha256".to_string(),
            value: OperationCondition::OldContentHash(message_hash.to_vec()),
        }],
    };
    let payload_bytes = payload.to_canonical_bytes().map_err(|err| err.to_string())?;
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload_bytes);
    envelope.add_signature(dev_author_signature(message)).map_err(|err| err.to_string())?;
    Ok(envelope)
}

fn dev_author_signature(message: &str) -> Signature {
    let mut signature_preimage = Vec::new();
    signature_preimage.extend_from_slice(b"prikk.dev.placeholder-signature.v1");
    signature_preimage.extend_from_slice(message.as_bytes());
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "dev-placeholder-author".to_string(),
        signature_bytes: sha256(&signature_preimage).to_vec(),
        created_at: 0,
        signer_role: SignerRole::Author,
    }
}

fn current_dir() -> std::result::Result<PathBuf, String> {
    std::env::current_dir().map_err(|err| err.to_string())
}

fn print_help() {
    println!("prikk {VERSION}");
    println!();
    println!("Usage:");
    println!("  prikk init [path]                         Create a .prikk repository layout");
    println!("  prikk commit --allow-empty -m <message>   Append an empty patch to the active WAL");
    println!("  prikk status                              Check repository and active WAL status");
    println!("  prikk verify [path]                       Verify objects and WAL records");
    println!("  prikk --version                           Print version");
}
