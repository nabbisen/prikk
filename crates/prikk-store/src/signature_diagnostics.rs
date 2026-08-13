//! Signature-envelope shape/order diagnostics: malformed algorithm shape, duplicate signature
//! tuples, and non-canonical ordering, reported as separate `SignatureEnvelopeIssue`s alongside
//! (not instead of) the hard rejection `ObjectEnvelope::validate_strict` already performs for the
//! same three conditions. See `classify_signature_envelope`'s own doc for why, post-RFC-103, that
//! makes this layer's non-empty-result path provably unreachable through `verify_repository`'s
//! pipeline, and why the code stays regardless (DC-95 Stage 1 round 6's ruling on unreachable
//! checks).

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

/// RFC 103: every one of this function's three call sites (`verify.rs`, `verify/objects.rs`,
/// `refs/verify.rs`) runs it immediately after a `crate::format::validate_read_schema(layout.format(),
/// &envelope)?` on the same envelope, propagating on `Err` before this function is ever reached.
/// `validate_read_schema` under `RepositoryFormat::CurrentV2` calls `envelope.validate_strict()`,
/// which independently checks the exact same three conditions this function classifies
/// (`signature_issues()`'s `malformed_shape`/`duplicate`/`noncanonical_order`) and hard-errors on any
/// of them. With format-1 retired -- whose `validate_read_schema` branch checked only
/// `schema_version`, never calling `validate_strict()` -- `CurrentV2` is the only format left, so any
/// envelope this function would flag was already rejected one call earlier. **Provably unreachable
/// through `verify_repository`'s pipeline**, the same shape as the rollback wrong-signature-length
/// check (DC-95 Stage 1 round 11), but not a downgrade of a blocking check: Stage 1 already classified
/// `signature_envelope_issues` from every source as "Excluded" -- it never backed a blocking
/// predicate, for any source, even before this. Kept, untested, with the argument recorded (round 6's
/// ruling on unreachable checks), since a caller could still construct an envelope directly and read
/// its issues without going through `verify_repository` at all.
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
            "envelope contains a signature with malformed algorithm shape",
        ));
    }
    if conditions.duplicate {
        issues.push(issue(
            "PRIKK-VERIFY-SIGNATURE-DUPLICATE",
            &source,
            "envelope contains a duplicate signature tuple",
        ));
    }
    if conditions.noncanonical_order {
        issues.push(issue(
            "PRIKK-VERIFY-SIGNATURE-NONCANONICAL-ORDER",
            &source,
            "envelope signatures are not in canonical order",
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
