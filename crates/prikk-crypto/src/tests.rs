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

/// RFC 135 §2.3: `generate_seed` is the CSPRNG entry point `prikk key generate` uses. Two draws
/// must differ -- the whole claim that the OS entropy source is genuinely wired, not a fixed or
/// zeroed buffer.
#[test]
fn generate_seed_draws_differ() {
    let a = Ed25519KeyPair::generate_seed().expect("OS CSPRNG must be available in this test env");
    let b = Ed25519KeyPair::generate_seed().expect("OS CSPRNG must be available in this test env");
    assert_ne!(a, b, "two independent draws must not produce the same seed");
}

/// `generate_seed` and `generate` must agree: deriving a keypair from the returned seed is the
/// same keypair `generate` would have produced from that same random draw, so `generate`'s
/// refactor onto `generate_seed` changed no behaviour.
#[test]
fn generate_seed_derives_the_same_keypair_shape_as_from_seed() {
    let seed =
        Ed25519KeyPair::generate_seed().expect("OS CSPRNG must be available in this test env");
    let a = Ed25519KeyPair::from_seed(&seed);
    let b = Ed25519KeyPair::from_seed(&seed);
    assert_eq!(a.public_key_bytes(), b.public_key_bytes());
}

#[test]
fn from_seed_is_deterministic() {
    let a = Ed25519KeyPair::from_seed(&[42_u8; 32]);
    let b = Ed25519KeyPair::from_seed(&[42_u8; 32]);
    assert_eq!(a.public_key_bytes(), b.public_key_bytes());
    assert_eq!(a.sign(b"x"), b.sign(b"x"));
}

/// DC-80 criterion 4's negative control, run through the real product code (not a throwaway
/// harness) so it stands as evidence on both sides of the dependency bump: run once before
/// touching `ed25519-dalek`'s pinned version, and once after, both against this same test. The
/// "silent direction" the upgrade must not introduce is a strict verifier quietly accepting a
/// malleable signature it used to reject.
///
/// `S' = S + L` (`L` the ed25519 group order, little-endian) is the same underlying scalar residue
/// mod `L` as a valid signature's own `S`, encoded non-canonically. Critically, `S+L` here does
/// **not** set the top 3 bits of the last byte, so the naive legacy "check the high bits" heuristic
/// would miss it — only strict verification's `Scalar::from_canonical_bytes` catches it. A verifier
/// that silently started accepting this would have quietly downgraded from strict to permissive.
#[test]
fn verify_rejects_a_non_canonical_scalar_signature() {
    // 2^252 + 27742317777372353535851937790883648493, little-endian.
    const L_BYTES: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];

    let keypair = Ed25519KeyPair::from_seed(&[11_u8; 32]);
    let message = b"dc-80 non-canonical scalar negative control";
    let signature = keypair.sign(message);
    verify_ed25519(&keypair.public_key_bytes(), message, &signature)
        .expect("the unmodified signature must verify, establishing the baseline");

    let mut s_bytes = [0_u8; 32];
    s_bytes.copy_from_slice(&signature[32..64]);
    let mut malleated_s = [0_u8; 32];
    let mut carry: u16 = 0;
    for i in 0..32 {
        let sum = u16::from(s_bytes[i]) + u16::from(L_BYTES[i]) + carry;
        malleated_s[i] = (sum & 0xff) as u8;
        carry = sum >> 8;
    }
    assert_eq!(
        carry, 0,
        "S + L must not overflow 32 bytes for this test's seed/message"
    );
    assert_eq!(
        malleated_s[31] & 0xe0,
        0,
        "the malleated scalar must not trip the naive high-bit heuristic, or this test would not \
         exercise the subtle case strict verification exists for"
    );

    let mut malleated_signature = signature;
    malleated_signature[32..64].copy_from_slice(&malleated_s);
    assert!(
        verify_ed25519(&keypair.public_key_bytes(), message, &malleated_signature).is_err(),
        "a non-canonical scalar encoding of the same residue must be rejected, not silently accepted"
    );
}

/// DC-80 criterion 4's small-order-`R` counterpart, run through the real product code. The
/// identity point (compressed: `0x01` then 31 zero bytes) has order 1, which divides 8 — the
/// textbook small-order point strict verification must reject.
#[test]
fn verify_rejects_a_small_order_r_signature() {
    let keypair = Ed25519KeyPair::from_seed(&[13_u8; 32]);
    let message = b"dc-80 small-order R negative control";
    let signature = keypair.sign(message);

    let mut small_order_signature = signature;
    small_order_signature[0] = 1;
    small_order_signature[1..32].fill(0);
    assert!(
        verify_ed25519(&keypair.public_key_bytes(), message, &small_order_signature).is_err(),
        "a signature whose R is a small-order point must be rejected, not silently accepted"
    );
}
