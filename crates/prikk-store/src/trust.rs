//! Minimal repository-local publication trust store.
//!
//! DC-11 deliberately supports only one trusted MAINTAINER key with `required = 1`. The parser is
//! strict and fixed-shape; this module is not a general TOML implementation.

use prikk_crypto::{ED25519_KEY_LEN, verify_ed25519};
use prikk_error::{PrikkError, Result};
use prikk_hash::to_hex;
use prikk_object::{ObjectEnvelope, Signature, SignatureAlgorithm, SignerRole};

use crate::fsutil::{ensure_directory_required, read_file_required, write_file_atomically};
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

/// Minimal maintainer policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintainerTrustPolicy {
    /// The single trusted maintainer key id.
    pub key_id: String,
    /// The trusted Ed25519 public key.
    pub public_key: [u8; ED25519_KEY_LEN],
}

/// Add or replace the single trusted MAINTAINER key and DC-11 policy.
pub fn add_trusted_maintainer(
    layout: &RepositoryLayout,
    key_id: &str,
    public_key_hex: &str,
) -> Result<MaintainerTrustPolicy> {
    let _active_lock = ActiveLock::acquire(layout)?;
    crate::refs::ensure_no_incomplete_publication(layout)?;
    Signature::validate_key_id(key_id)?;
    let public_key = decode_public_key_hex(public_key_hex)?;
    let keys_dir = layout.repository_relative(&layout.maintainer_trust_keys_dir())?;
    ensure_directory_required(layout.repository_mutation_root(), &keys_dir)?;
    let key_path = layout.maintainer_trust_key_path(key_id)?;
    let key_relative = layout.repository_relative(&key_path)?;
    let public_key_text = format!("{}\n", to_hex(&public_key));
    write_file_atomically(
        layout.repository_mutation_root(),
        &key_relative,
        public_key_text.as_bytes(),
    )?;
    let policy_text = format!("[maintainer]\nrequired = 1\nkeys = [\"{key_id}\"]\n");
    let policy_relative = layout.repository_relative(&layout.trust_policy_path())?;
    write_file_atomically(
        layout.repository_mutation_root(),
        &policy_relative,
        policy_text.as_bytes(),
    )?;
    Ok(MaintainerTrustPolicy {
        key_id: key_id.to_string(),
        public_key,
    })
}

/// Load and validate the repository-local MAINTAINER trust policy.
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
    let key_id = parse_policy_key_id(&policy_text)?;
    Signature::validate_key_id(&key_id)?;
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
    Ok(MaintainerTrustPolicy { key_id, public_key })
}

/// Verify that the signer matches the current repository-local trust policy.
pub fn verify_signer_trusted(
    layout: &RepositoryLayout,
    signer: &impl MaintainerSigner,
) -> Result<MaintainerTrustPolicy> {
    let policy = load_maintainer_trust_policy(layout)?;
    if signer.key_id() != policy.key_id {
        return Err(PrikkError::InvalidSignature(format!(
            "maintainer signer key id {} is not trusted by policy",
            signer.key_id()
        )));
    }
    let signer_public_key = signer.public_key_bytes();
    if signer_public_key != policy.public_key {
        return Err(PrikkError::InvalidSignature(format!(
            "maintainer signer public key does not match trusted key {}",
            policy.key_id
        )));
    }
    Ok(policy)
}

/// Verify a publication envelope against the current repository-local trust policy.
pub fn verify_trusted_publication_envelope(
    policy: &MaintainerTrustPolicy,
    envelope: &ObjectEnvelope,
) -> std::result::Result<(), PublicationTrustIssue> {
    let object_id = envelope.object_id();
    let trusted = envelope
        .signatures
        .iter()
        .any(|signature| verify_trusted_signature(policy, envelope, signature, object_id).is_ok());
    if trusted {
        Ok(())
    } else {
        Err(PublicationTrustIssue::new(
            "PRIKK-TRUST-PUBLICATION-UNTRUSTED",
            format!(
                "{} {} has no trusted MAINTAINER signature",
                envelope.object_type, object_id
            ),
        ))
    }
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
    if signature.key_id != policy.key_id {
        return Err(PrikkError::InvalidSignature(
            "publication signature key id is not trusted".to_string(),
        ));
    }
    let preimage = Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        envelope.object_type,
        object_id,
        SignerRole::Maintainer,
        &signature.key_id,
    )?;
    verify_ed25519(&policy.public_key, &preimage, &signature.signature_bytes)
}

fn parse_policy_key_id(policy_text: &str) -> Result<String> {
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
    let Some(keys) = lines.get(2) else {
        return Err(PrikkError::MalformedData(
            "trust policy missing keys line".to_string(),
        ));
    };
    let Some(rest) = keys.strip_prefix("keys = [\"") else {
        return Err(PrikkError::MalformedData(
            "trust policy keys line must be keys = [\"<key-id>\"]".to_string(),
        ));
    };
    let Some(key_id) = rest.strip_suffix("\"]") else {
        return Err(PrikkError::MalformedData(
            "trust policy keys line must contain exactly one key".to_string(),
        ));
    };
    Signature::validate_key_id(key_id)?;
    Ok(key_id.to_string())
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

#[cfg(test)]
mod tests;
