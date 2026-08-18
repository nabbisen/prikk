use prikk_crypto::{Ed25519KeyPair, verify_ed25519};
use prikk_object::{
    CanonicalEncode, CreateFile, NodeId, ObjectId, ObjectType, Operation, OperationKind,
    PatchPayload, PatchPurpose, Signature, SignatureAlgorithm, SignerRole,
};

use crate::{
    Ed25519AuthorSigner, Ed25519MaintainerSigner, MaintainerSigner, author_signature,
    maintainer_signature,
};

const PUBLIC_KEY: [u8; 32] = [
    0x21, 0x52, 0xf8, 0xd1, 0x9b, 0x79, 0x1d, 0x24, 0x45, 0x32, 0x42, 0xe1, 0x5f, 0x2e, 0xab, 0x6c,
    0xb7, 0xcf, 0xfa, 0x7b, 0x6a, 0x5e, 0xd3, 0x00, 0x97, 0x96, 0x0e, 0x06, 0x98, 0x81, 0xdb, 0x12,
];

const SIGNATURE: [u8; 64] = [
    0x10, 0x2c, 0x73, 0xaf, 0xdf, 0x34, 0xfc, 0xd4, 0x51, 0x7b, 0x9c, 0x47, 0x9a, 0x11, 0xc3, 0x92,
    0xe6, 0x29, 0xda, 0x37, 0xcd, 0xe5, 0x8b, 0x8e, 0x88, 0x2c, 0xc9, 0xb3, 0xae, 0x28, 0x26, 0x19,
    0x4c, 0x3a, 0xb6, 0xbe, 0x87, 0x44, 0x68, 0x65, 0xce, 0x5c, 0xda, 0xef, 0x12, 0xff, 0xc4, 0xed,
    0x8d, 0xd8, 0x7b, 0x1e, 0xc7, 0xf8, 0x7a, 0x8d, 0x8a, 0xe9, 0xe0, 0x2c, 0x5f, 0x1f, 0xb1, 0x0d,
];

fn golden_preimage() -> prikk_error::Result<Vec<u8>> {
    Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::RefUpdate,
        ObjectId::from_bytes(core::array::from_fn(|index| index as u8)),
        SignerRole::Maintainer,
        "maintainer_1",
    )
}

#[test]
fn dc39_literal_vector_signs_and_verifies_through_production_apis() -> prikk_error::Result<()> {
    assert_eq!(
        prikk_object::ED25519_SIGNATURE_LEN,
        prikk_crypto::ED25519_SIGNATURE_LEN
    );
    let preimage = golden_preimage()?;
    assert_eq!(
        prikk_hash::to_hex(&preimage),
        "7072696b6b2e7369672e763100010004000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0002000c6d61696e7461696e65725f31"
    );
    assert_eq!(
        Ed25519KeyPair::from_seed(&[0x42; 32]).public_key_bytes(),
        PUBLIC_KEY
    );
    assert_eq!(
        Ed25519KeyPair::from_seed(&[0x42; 32]).sign(&preimage),
        SIGNATURE
    );
    verify_ed25519(&PUBLIC_KEY, &preimage, &SIGNATURE)
}

#[test]
fn every_signature_preimage_field_is_cryptographically_bound() -> prikk_error::Result<()> {
    let preimage = golden_preimage()?;
    for index in [0, 12, 14, 16, 48, 50, 52] {
        let mut changed = preimage.clone();
        let changed_byte = changed.get_mut(index).ok_or_else(|| {
            prikk_error::PrikkError::Integrity(
                "signature preimage mutation index was missing".to_string(),
            )
        })?;
        *changed_byte ^= 1;
        assert!(verify_ed25519(&PUBLIC_KEY, &changed, &SIGNATURE).is_err());
    }
    Ok(())
}

#[test]
fn production_author_and_maintainer_paths_use_verifiable_shared_preimages()
-> prikk_error::Result<()> {
    let object_id = ObjectId::from_bytes([0x33; 32]);
    let author = Ed25519AuthorSigner::from_seed("author", &[0x11; 32])?;
    let author_signature = author_signature(&author, object_id)?;
    let author_preimage = Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::Patch,
        object_id,
        SignerRole::Author,
        "author",
    )?;
    verify_ed25519(
        &author.public_key_bytes(),
        &author_preimage,
        &author_signature.signature_bytes,
    )?;

    let maintainer = Ed25519MaintainerSigner::from_seed("maintainer", &[0x22; 32])?;
    let maintainer_signature = maintainer_signature(&maintainer, ObjectType::RefState, object_id)?;
    let maintainer_preimage = Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::RefState,
        object_id,
        SignerRole::Maintainer,
        "maintainer",
    )?;
    verify_ed25519(
        &maintainer.public_key_bytes(),
        &maintainer_preimage,
        &maintainer_signature.signature_bytes,
    )
}

// DC-53 Stage 1, design-v1.md §4: committed literal vectors 1-5, the DC-40 precedent applied to
// AUTHOR signatures -- so the verification contract is reviewable independently of whichever
// public-key source Stage 1 ends up using, per `RFC-DC-53-stage1-report-v1.md`.

const DC53_PUBLIC_KEY: [u8; 32] = [
    0xf8, 0x0c, 0xcc, 0xdc, 0xe4, 0xae, 0x1c, 0x07, 0xae, 0x20, 0x8a, 0x2a, 0xdf, 0x99, 0xa3, 0x10,
    0xae, 0x42, 0x07, 0xe0, 0x30, 0x6f, 0xa0, 0x23, 0x61, 0x10, 0xb0, 0x68, 0x27, 0xbb, 0xb8, 0xd0,
];

const DC53_SIGNATURE: [u8; 64] = [
    0xf3, 0xbc, 0xe1, 0x56, 0x2d, 0x08, 0x93, 0x73, 0xc6, 0x9d, 0x62, 0xb1, 0xca, 0x05, 0x06, 0x44,
    0x54, 0xef, 0xe0, 0xaa, 0x81, 0xf6, 0x7d, 0x45, 0xe9, 0xf1, 0xc3, 0x12, 0x8d, 0xe9, 0x4f, 0xb2,
    0xb0, 0xae, 0x5f, 0x32, 0x09, 0x30, 0x0a, 0x9e, 0x56, 0xdc, 0x2b, 0x51, 0xcf, 0x42, 0x24, 0xce,
    0x53, 0x1c, 0x9e, 0x05, 0x2e, 0x7e, 0x0a, 0xc9, 0xcf, 0x0d, 0x31, 0x91, 0x23, 0x4a, 0x3c, 0x09,
];

/// Vector 2: a minimal single-operation Patch payload, literal enough to be reviewed on its own.
fn dc53_patch_payload() -> PatchPayload {
    PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "dc53.txt".to_string(),
                node_id: NodeId::from_bytes([0x53; 32]),
                blob_id: ObjectId::from_bytes([0x44; 32]),
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    }
}

fn dc53_object_id() -> prikk_error::Result<ObjectId> {
    let bytes = dc53_patch_payload().to_canonical_bytes()?;
    Ok(ObjectId::from_canonical_payload(
        ObjectType::Patch,
        1,
        &bytes,
    ))
}

fn dc53_preimage() -> prikk_error::Result<Vec<u8>> {
    Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::Patch,
        dc53_object_id()?,
        SignerRole::Author,
        "dc53-author",
    )
}

/// Vectors 1-3: a known keypair, the known payload's canonical AUTHOR preimage, and a valid
/// signature over it that verifies.
#[test]
fn dc53_vectors_1_to_3_author_signature_signs_and_verifies() -> prikk_error::Result<()> {
    let preimage = dc53_preimage()?;
    assert_eq!(
        prikk_hash::to_hex(&preimage),
        "7072696b6b2e7369672e763100010001aaec00c9e7985cf4592f8afb6bbcc411e90e450f225cb098af6398e902b62acd0001000b646335332d617574686f72"
    );
    assert_eq!(
        Ed25519KeyPair::from_seed(&[0x53; 32]).public_key_bytes(),
        DC53_PUBLIC_KEY
    );
    assert_eq!(
        Ed25519KeyPair::from_seed(&[0x53; 32]).sign(&preimage),
        DC53_SIGNATURE
    );
    verify_ed25519(&DC53_PUBLIC_KEY, &preimage, &DC53_SIGNATURE)
}

/// Vector 4: one bit flipped in an otherwise-valid signature must fail.
#[test]
fn dc53_vector_4_a_mutated_signature_fails() -> prikk_error::Result<()> {
    let preimage = dc53_preimage()?;
    let mut mutated = DC53_SIGNATURE;
    mutated[0] ^= 1;
    assert!(verify_ed25519(&DC53_PUBLIC_KEY, &preimage, &mutated).is_err());
    Ok(())
}

/// Vector 5: a signature genuinely valid over a *different* preimage must still fail against this
/// one -- the vector that catches a preimage-construction error (design-v1.md §4, "the one most
/// likely to be omitted").
#[test]
fn dc53_vector_5_a_signature_valid_over_a_different_preimage_fails() -> prikk_error::Result<()> {
    let other_object_id = ObjectId::from_bytes([0x99; 32]);
    let other_preimage = Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::Patch,
        other_object_id,
        SignerRole::Author,
        "dc53-author",
    )?;
    let other_signature = Ed25519KeyPair::from_seed(&[0x53; 32]).sign(&other_preimage);
    // Sanity: the other signature is genuinely valid over its own preimage -- this is not a
    // malformed-signature test, it is a wrong-preimage test.
    verify_ed25519(&DC53_PUBLIC_KEY, &other_preimage, &other_signature)?;

    let preimage = dc53_preimage()?;
    assert!(verify_ed25519(&DC53_PUBLIC_KEY, &preimage, &other_signature).is_err());
    Ok(())
}
