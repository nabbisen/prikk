//! Negative validators — rejection tests. Seeded in Phase 0; later DC-09 phases
//! (codec value_type, payload discriminators, node-id lifecycle) extend this.

use crate::id::{ObjectId, ObjectType};

#[test]
fn unknown_object_type_code_is_rejected() {
    assert!(ObjectType::from_code(0).is_err());
    assert!(ObjectType::from_code(99).is_err());
}

#[test]
fn object_id_rejects_malformed_hex() {
    // non-hex character
    assert!("zz".repeat(32).parse::<ObjectId>().is_err());
    // uppercase is rejected (canonical form is lowercase)
    assert!("A".repeat(64).parse::<ObjectId>().is_err());
    // wrong length
    assert!("510ab866".parse::<ObjectId>().is_err());
}
