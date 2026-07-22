use prikk_crypto::{Ed25519KeyPair, verify_ed25519};
use prikk_object::{ObjectId, ObjectType, Signature, SignatureAlgorithm, SignerRole};

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
