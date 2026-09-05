#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Shared error taxonomy for Prikk crates.

use core::fmt;

/// Shared result type.
pub type Result<T> = core::result::Result<T, PrikkError>;

/// Error type used by the initial implementation crates.
///
/// `#[non_exhaustive]` (RFC 132 increment 1): `prikk-error` is published, and until this attribute
/// landed, adding any new variant was a breaking change for every downstream match. Verified free to
/// add before landing it: no exhaustive `match` on a `PrikkError` value exists anywhere in this
/// workspace.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrikkError {
    /// Canonical encoding failed because input data violates the frozen schema contract.
    CanonicalEncoding(String),
    /// An object identifier had an invalid form.
    InvalidObjectId(String),
    /// A signature had an invalid form or did not match its envelope context.
    InvalidSignature(String),
    /// A path-like name failed Prikk path/ref validation.
    InvalidName(String),
    /// A persistent object had an unexpected type.
    ObjectTypeMismatch {
        /// The object type required by the caller.
        expected: String,
        /// The object type actually found in the stored envelope.
        actual: String,
    },
    /// The persistent format version is unsupported.
    UnsupportedFormatVersion(u32),
    /// A persisted object or record has malformed bytes.
    MalformedData(String),
    /// A persisted object was found at a path that does not match its computed ID.
    Integrity(String),
    /// A lock could not be acquired because another writer may be active.
    LockConflict(String),
    /// A caller precondition for the requested operation is not satisfied. Distinct from
    /// [`Self::LockConflict`], which means another writer may hold a lock: nothing here is
    /// transient and waiting does not help — the caller must change what they asked for.
    Precondition(String),
    /// The requested object type cannot be persisted in the requested store.
    UnsupportedObjectType(String),
    /// An I/O failure. `kind` is `Some` only when this value was built from a real
    /// `std::io::Error` via [`From`] -- every explicit construction site elsewhere in the workspace
    /// (a caller-precondition violation, a platform-capability refusal, or a validation failure
    /// wearing this variant rather than one that describes it) sets `kind: None`, which is the
    /// truth, not a placeholder to "tidy" into something non-optional. RFC 132 increment 2 is
    /// expected to move those sites onto variants that describe them and narrow this field.
    Io {
        /// The underlying `std::io::ErrorKind`, when this was built from a real `std::io::Error`.
        kind: Option<std::io::ErrorKind>,
        /// Human-readable context. Alone carries the full `Display` message -- see that impl.
        context: String,
    },
}

impl fmt::Display for PrikkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanonicalEncoding(msg) => write!(f, "canonical encoding error: {msg}"),
            Self::InvalidObjectId(msg) => write!(f, "invalid object id: {msg}"),
            Self::InvalidSignature(msg) => write!(f, "invalid signature: {msg}"),
            Self::InvalidName(msg) => write!(f, "invalid name: {msg}"),
            Self::ObjectTypeMismatch { expected, actual } => {
                write!(f, "object type mismatch: expected {expected}, got {actual}")
            }
            Self::UnsupportedFormatVersion(version) => {
                write!(f, "unsupported format version: {version}")
            }
            Self::MalformedData(msg) => write!(f, "malformed persisted data: {msg}"),
            Self::Integrity(msg) => write!(f, "integrity error: {msg}"),
            Self::LockConflict(msg) => write!(f, "lock conflict: {msg}"),
            Self::Precondition(msg) => write!(f, "precondition not met: {msg}"),
            Self::UnsupportedObjectType(msg) => write!(f, "unsupported object type: {msg}"),
            Self::Io { context, .. } => write!(f, "i/o error: {context}"),
        }
    }
}

impl std::error::Error for PrikkError {}

impl From<std::io::Error> for PrikkError {
    fn from(value: std::io::Error) -> Self {
        Self::Io {
            kind: Some(value.kind()),
            context: value.to_string(),
        }
    }
}
