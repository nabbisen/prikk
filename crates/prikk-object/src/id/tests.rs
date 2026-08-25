use super::{ObjectId, ObjectType};

#[test]
fn object_id_is_deterministic() {
    let a = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"payload");
    let b = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"payload");
    let c = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"payload");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(
        a.to_hex(),
        "5f8711b3f84991d60b65221d66ed5ec260d28cc19c5c4ed3c1fe44d334265fe6"
    );
}

#[test]
fn hex_roundtrip() {
    let id = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"payload");
    let text = id.to_hex();
    let parsed = text.parse::<ObjectId>();
    assert_eq!(parsed, Ok(id));
}

/// Repository-identity settlement handoff v1 §3: `0x0A` (formerly `ProjectGenesis`) must be
/// refused with a message naming the retirement, not the generic "unknown code" every other
/// never-assigned number gets -- otherwise a retired code and a merely-unassigned one look
/// identical to a caller, and there is no record that reuse was ever considered a mistake.
#[allow(clippy::expect_used)]
#[test]
fn a_retired_code_is_refused_with_a_distinct_message() {
    let error = ObjectType::from_code(0x0A).expect_err("0x0A must not decode to anything");
    let message = error.to_string();
    assert!(
        message.contains("retired") && message.contains("project-genesis"),
        "expected a retirement-specific message, got: {message}"
    );
    let never_assigned =
        ObjectType::from_code(0xFF).expect_err("0xFF has never been assigned and must also fail");
    assert!(
        !never_assigned.to_string().contains("retired"),
        "an ordinary unassigned code must not read as a retirement: {never_assigned}"
    );
}
