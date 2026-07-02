//! Ed25519 sign/verify round-trip and tamper tests.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use super::{Ed25519KeyPair, verify_ed25519};

#[test]
fn sign_then_verify_round_trips() {
    let keypair = Ed25519KeyPair::from_seed(&[7_u8; 32]);
    let message = b"prikk.sig.v1 preimage bytes";
    let signature = keypair.sign(message);
    verify_ed25519(&keypair.public_key_bytes(), message, &signature)
        .expect("valid signature must verify");
}

#[test]
fn verify_fails_on_tampered_message() {
    let keypair = Ed25519KeyPair::from_seed(&[9_u8; 32]);
    let signature = keypair.sign(b"original");
    assert!(verify_ed25519(&keypair.public_key_bytes(), b"tampered", &signature).is_err());
}

#[test]
fn verify_fails_against_wrong_public_key() {
    let signer = Ed25519KeyPair::from_seed(&[1_u8; 32]);
    let other = Ed25519KeyPair::from_seed(&[2_u8; 32]);
    let message = b"message";
    let signature = signer.sign(message);
    assert!(verify_ed25519(&other.public_key_bytes(), message, &signature).is_err());
}

#[test]
fn verify_rejects_malformed_signature_length() {
    let keypair = Ed25519KeyPair::from_seed(&[3_u8; 32]);
    assert!(verify_ed25519(&keypair.public_key_bytes(), b"m", &[0_u8; 10]).is_err());
}

#[test]
fn from_seed_is_deterministic() {
    let a = Ed25519KeyPair::from_seed(&[42_u8; 32]);
    let b = Ed25519KeyPair::from_seed(&[42_u8; 32]);
    assert_eq!(a.public_key_bytes(), b.public_key_bytes());
    assert_eq!(a.sign(b"x"), b.sign(b"x"));
}
