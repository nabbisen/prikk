#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Object identity and canonical payload definitions for Prikk.
//!
//! This crate deliberately does **not** use protobuf bytes for object identity. It implements a
//! small deterministic canonical encoder seed that can be extended as FDD-03 matures.

pub mod canonical;
pub mod envelope;
pub mod id;
pub mod payload;
pub mod signature;

pub use canonical::{CanonicalEncode, CanonicalWriter, WireType};
pub use envelope::ObjectEnvelope;
pub use id::{OBJECT_ID_DOMAIN, ObjectId, ObjectType};
pub use payload::*;
pub use signature::{Signature, SignatureAlgorithm, SignerRole};
