//! Format-1 signature-envelope compatibility diagnostics.

use std::fmt;

use prikk_error::Result;
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

/// Persisted source of a signature-envelope diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureEnvelopeSource {
    /// Content-addressed object file.
    Object {
        /// Object type, ordered by its numeric registry code.
        object_type: ObjectType,
        /// Object identifier, ordered by its raw bytes.
        object_id: ObjectId,
    },
    /// Active-session WAL record.
    ActiveWal {
        /// WAL sequence number.
        sequence: u64,
        /// Envelope object identifier.
        object_id: ObjectId,
    },
    /// Inline ref-log record.
    RefLog {
        /// Canonical ref name.
        ref_name: String,
        /// One-based record sequence within the ref log.
        sequence: u64,
        /// RefUpdate object identifier.
        object_id: ObjectId,
    },
}

impl fmt::Display for SignatureEnvelopeSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Object {
                object_type,
                object_id,
            } => write!(formatter, "object {object_type} {object_id}"),
            Self::ActiveWal {
                sequence,
                object_id,
            } => write!(
                formatter,
                "active WAL sequence {sequence} object {object_id}"
            ),
            Self::RefLog {
                ref_name,
                sequence,
                object_id,
            } => write!(
                formatter,
                "ref log {ref_name} sequence {sequence} object {object_id}"
            ),
        }
    }
}

/// One warning-level non-canonical signature-envelope condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureEnvelopeIssue {
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Persisted envelope source.
    pub source: SignatureEnvelopeSource,
    /// Human-readable diagnosis without host paths.
    pub message: String,
}

pub(crate) fn classify_signature_envelope(
    envelope: &ObjectEnvelope,
    source: SignatureEnvelopeSource,
) -> Result<Vec<SignatureEnvelopeIssue>> {
    let conditions = envelope.signature_issues()?;
    let mut issues = Vec::with_capacity(3);
    if conditions.malformed_shape {
        issues.push(issue(
            "PRIKK-VERIFY-SIGNATURE-MALFORMED",
            &source,
            "format-1 envelope contains a signature with malformed algorithm shape",
        ));
    }
    if conditions.duplicate {
        issues.push(issue(
            "PRIKK-VERIFY-SIGNATURE-DUPLICATE",
            &source,
            "format-1 envelope contains a duplicate signature tuple",
        ));
    }
    if conditions.noncanonical_order {
        issues.push(issue(
            "PRIKK-VERIFY-SIGNATURE-NONCANONICAL-ORDER",
            &source,
            "format-1 envelope signatures are not in canonical order",
        ));
    }
    Ok(issues)
}

fn issue(
    code: &'static str,
    source: &SignatureEnvelopeSource,
    message: &str,
) -> SignatureEnvelopeIssue {
    SignatureEnvelopeIssue {
        code,
        source: source.clone(),
        message: message.to_string(),
    }
}
