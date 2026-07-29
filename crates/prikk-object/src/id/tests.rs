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
