#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Shared error taxonomy for PRIKK crates.

use core::fmt;

/// Shared result type.
pub type Result<T> = core::result::Result<T, PrikkError>;

/// Error type used by the initial implementation crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrikkError {
    /// Canonical encoding failed because input data violates the frozen schema contract.
    CanonicalEncoding(String),
    /// An object identifier had an invalid form.
    InvalidObjectId(String),
    /// A signature had an invalid form or did not match its envelope context.
    InvalidSignature(String),
    /// A path-like name failed PRIKK path/ref validation.
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
    /// Placeholder for I/O errors without making this crate depend on std::io::Error in variants.
    Io(String),
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
            Self::Io(msg) => write!(f, "i/o error: {msg}"),
        }
    }
}

impl std::error::Error for PrikkError {}

impl From<std::io::Error> for PrikkError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}
