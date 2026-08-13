//! Repository-local publication trust store: the set of MAINTAINER keys this repository accepts as
//! having sealed a `Block`/`RefState` (DC-78 §D2). `required = 1` keeps its DC-11 meaning regardless
//! of how many keys are adopted — a block needs *one* trusted signature, never a threshold of
//! several (`rfcs/done/DC-78-HISTORY-EXCHANGE.md` §D2, confirmed against every existing
//! assumption at §D7.2). The parser stays strict and fixed-shape (DC-11); this module is not a
//! general TOML implementation.

use prikk_crypto::{ED25519_KEY_LEN, verify_ed25519};
use prikk_error::{PrikkError, Result};
use prikk_hash::to_hex;
use prikk_object::{ObjectEnvelope, Signature, SignatureAlgorithm, SignerRole, ascii_fold};

use crate::fsutil::{
    EntryKind, ensure_directory_required, list_directory, read_file_if_exists, read_file_required,
    write_file_atomically,
};
use crate::layout::RepositoryLayout;
use crate::lock::ActiveLock;
use crate::maintainer_signing::MaintainerSigner;

/// One publication-trust issue found during repository verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationTrustIssue {
    /// Stable issue code.
    pub code: &'static str,
    /// Human-readable explanation.
    pub message: String,
}

impl PublicationTrustIssue {
    /// Construct a publication trust issue.
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// One adopted MAINTAINER key. DC-78 §D5's full TOFU provenance (the block id a key was first
/// accepted at, and the ref name it arrived under) is recorded only for keys adopted through
/// exchange, which lands with the import path — a key declared locally via `trust maintainer add`
/// has no such provenance to record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptedMaintainerKey {
    /// The adopted maintainer key id.
    pub key_id: String,
    /// The adopted Ed25519 public key.
    pub public_key: [u8; ED25519_KEY_LEN],
}

/// The repository-local set of adopted MAINTAINER keys (DC-78 §D2). A `Block`/`RefState` is trusted
/// if *any* adopted key signed it — object trust, not ref authority; adopting a key never lets it
/// move a ref (`RefStore::publish` still requires a signature from this operator's own signer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintainerTrustPolicy {
    /// Every currently adopted key, in the order it was adopted.
    pub keys: Vec<AdoptedMaintainerKey>,
}

impl MaintainerTrustPolicy {
    fn find(&self, key_id: &str) -> Option<&AdoptedMaintainerKey> {
        self.keys.iter().find(|key| key.key_id == key_id)
    }
}

/// Adopt a MAINTAINER key (DC-78 §D2/§D5): add it if the key id is new; succeed idempotently,
/// changing nothing, if it is already adopted with the *same* public key; **refuse** if it is
/// already adopted with a *different* public key. This is DC-78's TOFU enforcement — "a changed key
/// for a known key id is refused, not re-prompted" — not just the create path for a fresh key id.
/// Returns the adopted key and whether this call actually wrote anything.
pub fn add_trusted_maintainer(
    layout: &RepositoryLayout,
    key_id: &str,
    public_key_hex: &str,
) -> Result<(AdoptedMaintainerKey, bool)> {
    layout.require_current_format()?;
    let _active_lock = ActiveLock::acquire(layout)?;
    crate::refs::ensure_no_incomplete_publication(layout)?;
    Signature::validate_key_id(key_id)?;
    let public_key = decode_public_key_hex(public_key_hex)?;
    let keys_dir = layout.repository_relative(&layout.maintainer_trust_keys_dir())?;
    ensure_directory_required(layout.repository_mutation_root(), &keys_dir)?;

    // Runs before read_existing_key, and unconditionally, so a case-insensitive filesystem's own
    // folding (APFS) can never stand in for this check: read_existing_key("dev-maintainer") returning
    // "Dev-Maintainer"'s file would otherwise be indistinguishable from a genuine idempotent re-add,
    // silently conflating two key ids under one physical file (found on macOS CI after Stage 1 merged
    // — dc72_path_safety_collisions.rs::maintainer_key_id_rejects_case_insensitive_collision).
    // Excludes exact self-matches, so a real idempotent or TOFU-refusal re-add of the same key id is
    // unaffected.
    validate_no_maintainer_key_id_collision(layout, &keys_dir, key_id)?;

    match read_existing_key(layout, key_id)? {
        Some(existing) if existing == public_key => {}
        Some(_) => {
            return Err(PrikkError::InvalidSignature(format!(
                "maintainer key id {key_id} is already adopted with a different public key"
            )));
        }
        None => {
            let key_path = layout.maintainer_trust_key_path(key_id)?;
            let key_relative = layout.repository_relative(&key_path)?;
            let public_key_text = format!("{}\n", to_hex(&public_key));
            write_file_atomically(
                layout.repository_mutation_root(),
                &key_relative,
                public_key_text.as_bytes(),
            )?;
        }
    }

    let mut key_ids = load_policy_key_ids(layout)?;
    let adopted = AdoptedMaintainerKey {
        key_id: key_id.to_string(),
        public_key,
    };
    if key_ids.iter().any(|existing| existing == key_id) {
        return Ok((adopted, false));
    }
    key_ids.push(key_id.to_string());
    let policy_text = format!(
        "[maintainer]\nrequired = 1\nkeys = [{}]\n",
        key_ids
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let policy_relative = layout.repository_relative(&layout.trust_policy_path())?;
    write_file_atomically(
        layout.repository_mutation_root(),
        &policy_relative,
        policy_text.as_bytes(),
    )?;
    Ok((adopted, true))
}

/// Read `key_id`'s adopted public key from its own key file, if one has already been written.
fn read_existing_key(
    layout: &RepositoryLayout,
    key_id: &str,
) -> Result<Option<[u8; ED25519_KEY_LEN]>> {
    let key_path = layout.maintainer_trust_key_path(key_id)?;
    let key_relative = layout.repository_relative(&key_path)?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &key_relative)? else {
        return Ok(None);
    };
    let text = String::from_utf8(bytes).map_err(|err| {
        PrikkError::Integrity(format!(
            "adopted maintainer key {key_id} is not valid UTF-8: {err}"
        ))
    })?;
    Ok(Some(decode_public_key_hex(text.trim_end_matches('\n'))?))
}

/// The current policy's key ids, in on-disk order — empty if no policy file exists yet (the first
/// key ever adopted in this repository).
fn load_policy_key_ids(layout: &RepositoryLayout) -> Result<Vec<String>> {
    let policy_relative = layout.repository_relative(&layout.trust_policy_path())?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &policy_relative)?
    else {
        return Ok(Vec::new());
    };
    let policy_text = String::from_utf8(bytes).map_err(|err| {
        PrikkError::Integrity(format!(
            "publication trust policy is not valid UTF-8: {err}"
        ))
    })?;
    parse_policy_keys(&policy_text)
}

/// Reject a maintainer key id whose ASCII-folded form collides with an existing key file other than
/// itself (DC-72). Runs unconditionally, before any filesystem lookup keyed on `key_id` — a
/// case-insensitive filesystem (APFS) can fold a genuinely different id onto an existing key's file,
/// which must never be mistaken for that id's own idempotent or TOFU-refusal re-add. Re-adopting
/// `key_id` unchanged (same exact string) is not a collision with itself; every other case-insensitive
/// match is. Folds through `prikk_object::ascii_fold`, the one shared folding definition (DC-72 design
/// ruling, `rfcs/accepted/DC-72-PATH-SAFETY-CONFORMANCE.md` §3.5) — see its doc comment for the
/// recorded NFC/NFD limitation this inherits.
fn validate_no_maintainer_key_id_collision(
    layout: &RepositoryLayout,
    keys_dir_relative: &std::path::Path,
    key_id: &str,
) -> Result<()> {
    let folded = ascii_fold(key_id);
    for entry in list_directory(layout.repository_mutation_root(), keys_dir_relative)? {
        if entry.kind != EntryKind::Regular {
            continue;
        }
        let Some(name) = entry.name.to_str() else {
            continue;
        };
        let Some(existing_id) = name.strip_suffix(".pub") else {
            continue;
        };
        if existing_id != key_id && ascii_fold(existing_id) == folded {
            return Err(PrikkError::InvalidName(format!(
                "case-insensitive maintainer key id collision involving: {existing_id}"
            )));
        }
    }
    Ok(())
}

/// Load and validate the repository-local set of adopted MAINTAINER keys.
pub fn load_maintainer_trust_policy(layout: &RepositoryLayout) -> Result<MaintainerTrustPolicy> {
    let policy_relative = layout.repository_relative(&layout.trust_policy_path())?;
    let policy_text = String::from_utf8(read_file_required(
        layout.repository_mutation_root(),
        &policy_relative,
    )?)
    .map_err(|err| {
        PrikkError::Integrity(format!(
            "publication trust policy is missing or unreadable: {err}"
        ))
    })?;
    let key_ids = parse_policy_keys(&policy_text)?;
    let mut keys = Vec::with_capacity(key_ids.len());
    for key_id in key_ids {
        let key_path = layout.maintainer_trust_key_path(&key_id)?;
        let key_relative = layout.repository_relative(&key_path)?;
        let public_key_text = String::from_utf8(read_file_required(
            layout.repository_mutation_root(),
            &key_relative,
        )?)
        .map_err(|err| {
            PrikkError::Integrity(format!(
                "trusted maintainer key {key_id} is missing or unreadable: {err}"
            ))
        })?;
        let public_key = decode_public_key_hex(public_key_text.trim_end_matches('\n'))?;
        keys.push(AdoptedMaintainerKey { key_id, public_key });
    }
    Ok(MaintainerTrustPolicy { keys })
}

/// Verify that the signer matches one of the repository-local trust policy's adopted keys.
pub fn verify_signer_trusted(
    layout: &RepositoryLayout,
    signer: &impl MaintainerSigner,
) -> Result<MaintainerTrustPolicy> {
    let policy = load_maintainer_trust_policy(layout)?;
    let Some(matched) = policy.find(signer.key_id()) else {
        return Err(PrikkError::InvalidSignature(format!(
            "maintainer signer key id {} is not trusted by policy",
            signer.key_id()
        )));
    };
    let signer_public_key = signer.public_key_bytes();
    if signer_public_key != matched.public_key {
        return Err(PrikkError::InvalidSignature(format!(
            "maintainer signer public key does not match trusted key {}",
            matched.key_id
        )));
    }
    Ok(policy)
}

/// Verify a publication envelope against the current repository-local trust policy. Returns the
/// adopted key id whose signature matched (DC-78 §D3): the sealer's identity already lives inside
/// the envelope's own signature, non-strippably — this is reporting that fact, not new state.
pub fn verify_trusted_publication_envelope(
    policy: &MaintainerTrustPolicy,
    envelope: &ObjectEnvelope,
) -> std::result::Result<String, PublicationTrustIssue> {
    let object_id = envelope.object_id();
    envelope
        .signatures
        .iter()
        .find_map(|signature| {
            verify_trusted_signature(policy, envelope, signature, object_id)
                .ok()
                .map(|()| signature.key_id.clone())
        })
        .ok_or_else(|| {
            PublicationTrustIssue::new(
                "PRIKK-TRUST-PUBLICATION-UNTRUSTED",
                format!(
                    "{} {} has no trusted MAINTAINER signature",
                    envelope.object_type, object_id
                ),
            )
        })
}

fn verify_trusted_signature(
    policy: &MaintainerTrustPolicy,
    envelope: &ObjectEnvelope,
    signature: &Signature,
    object_id: prikk_object::ObjectId,
) -> Result<()> {
    if signature.algorithm != SignatureAlgorithm::Ed25519 {
        return Err(PrikkError::InvalidSignature(
            "publication signature is not Ed25519".to_string(),
        ));
    }
    if signature.signer_role != SignerRole::Maintainer {
        return Err(PrikkError::InvalidSignature(
            "publication signature role is not MAINTAINER".to_string(),
        ));
    }
    // O(adopted keys) string comparisons to find which key this signature claims, then at most one
    // Ed25519 verification (DC-78 §D7.4) — never one verification per adopted key.
    let Some(matched) = policy.find(&signature.key_id) else {
        return Err(PrikkError::InvalidSignature(
            "publication signature key id is not trusted".to_string(),
        ));
    };
    let preimage = Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        envelope.object_type,
        object_id,
        SignerRole::Maintainer,
        &signature.key_id,
    )?;
    verify_ed25519(&matched.public_key, &preimage, &signature.signature_bytes)
}

/// Parse the policy file's `[maintainer]` / `required = 1` / `keys = [...]` lines into an ordered
/// list of key ids. Still hand-rolled and fixed-shape (DC-11): exactly 3 non-empty lines, the first
/// two literal, only the `keys` line's bracket contents grow from DC-78 — split on `", "` and
/// individually validated, never a general list grammar. `Signature::validate_key_id` restricts key
/// ids to ASCII alphanumeric/`-`/`_`, so no valid key id can ever contain the `", "` separator or a
/// `"` character, making the split-then-validate order safe.
fn parse_policy_keys(policy_text: &str) -> Result<Vec<String>> {
    let lines: Vec<&str> = policy_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() != 3 {
        return Err(PrikkError::MalformedData(
            "trust policy must contain exactly [maintainer], required, and keys lines".to_string(),
        ));
    }
    let Some(section) = lines.first() else {
        return Err(PrikkError::MalformedData(
            "trust policy is empty".to_string(),
        ));
    };
    if *section != "[maintainer]" {
        return Err(PrikkError::MalformedData(
            "trust policy must start with [maintainer]".to_string(),
        ));
    }
    let Some(required) = lines.get(1) else {
        return Err(PrikkError::MalformedData(
            "trust policy missing required line".to_string(),
        ));
    };
    if *required != "required = 1" {
        return Err(PrikkError::MalformedData(
            "trust policy must set required = 1".to_string(),
        ));
    }
    let Some(keys_line) = lines.get(2) else {
        return Err(PrikkError::MalformedData(
            "trust policy missing keys line".to_string(),
        ));
    };
    let Some(rest) = keys_line.strip_prefix("keys = [") else {
        return Err(PrikkError::MalformedData(
            "trust policy keys line must be keys = [\"<key-id>\", ...]".to_string(),
        ));
    };
    let Some(inner) = rest.strip_suffix(']') else {
        return Err(PrikkError::MalformedData(
            "trust policy keys line must be keys = [\"<key-id>\", ...]".to_string(),
        ));
    };
    if inner.is_empty() {
        return Err(PrikkError::MalformedData(
            "trust policy keys list must not be empty".to_string(),
        ));
    }
    let mut key_ids: Vec<String> = Vec::new();
    for candidate in inner.split(", ") {
        let Some(key_id) = candidate
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        else {
            return Err(PrikkError::MalformedData(
                "trust policy keys entries must be double-quoted".to_string(),
            ));
        };
        Signature::validate_key_id(key_id)?;
        if key_ids.iter().any(|existing| existing == key_id) {
            return Err(PrikkError::MalformedData(format!(
                "trust policy lists key id {key_id} more than once"
            )));
        }
        key_ids.push(key_id.to_string());
    }
    Ok(key_ids)
}

fn decode_public_key_hex(hex: &str) -> Result<[u8; ED25519_KEY_LEN]> {
    if hex.len() != ED25519_KEY_LEN * 2 {
        return Err(PrikkError::MalformedData(format!(
            "maintainer public key must be {} lowercase hex characters",
            ED25519_KEY_LEN * 2
        )));
    }
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PrikkError::MalformedData(
            "maintainer public key contains non-hex bytes".to_string(),
        ));
    }
    if hex.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(PrikkError::MalformedData(
            "maintainer public key must use lowercase hex".to_string(),
        ));
    }
    let mut out = [0_u8; ED25519_KEY_LEN];
    for (slot, pair) in out.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
        let hi =
            hex_value(pair.first().copied().ok_or_else(|| {
                PrikkError::MalformedData("truncated public key hex".to_string())
            })?)?;
        let lo =
            hex_value(pair.get(1).copied().ok_or_else(|| {
                PrikkError::MalformedData("truncated public key hex".to_string())
            })?)?;
        *slot = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_value(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(PrikkError::MalformedData(
            "maintainer public key must use lowercase hex".to_string(),
        )),
    }
}

// DC-71: every test here sets up its scenario via real repository mutation (RepositoryLayout::init
// or equivalent), which is Linux-only; the module never compiles a non-Linux-meaningful test.
#[cfg(all(test, target_os = "linux"))]
mod tests;
