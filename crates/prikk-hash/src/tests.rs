//! SHA-256 test vectors for `prikk-hash`.
//!
//! Every expected digest below is provenance-labelled. Expected values are never derived by
//! running `sha256` from this crate on the same input — that would make the assertion
//! tautological. "Published" vectors are quoted from FIPS 180-2 / RFC 6234. "Independently
//! computed" vectors were produced by Python's `hashlib`, whose method was itself validated
//! against all four published vectors before use (see DC-41 stage-2 evidence note).

use super::{sha256, to_hex};

// --- Published vectors (FIPS 180-2 / RFC 6234) ---

#[test]
fn sha256_empty_matches_standard_vector() {
    assert_eq!(
        to_hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn sha256_abc_matches_standard_vector() {
    assert_eq!(
        to_hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// 56-byte published multi-block vector; exactly the first padding-block transition (448 bits),
/// so it needs no independently computed expectation.
#[test]
fn sha256_56_byte_published_string_matches_standard_vector() {
    assert_eq!(
        to_hex(&sha256(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

/// 112-byte published multi-block vector.
#[test]
fn sha256_112_byte_published_string_matches_standard_vector() {
    assert_eq!(
        to_hex(&sha256(
            b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
        )),
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
    );
}

// --- Independently computed boundary vectors (input: b'a' repeated n times) ---
//
// Expected digests were computed with Python's `hashlib`, not with `sha256` from this crate.

fn repeated_a(n: usize) -> Vec<u8> {
    vec![b'a'; n]
}

/// n=55: the last length whose padding still fits entirely in the first 64-byte block.
#[test]
fn sha256_55_bytes_matches_independently_computed_vector() {
    assert_eq!(
        to_hex(&sha256(&repeated_a(55))),
        "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
    );
}

/// n=56: the first length forcing a second padding block (mandated boundary).
#[test]
fn sha256_56_bytes_matches_independently_computed_vector() {
    assert_eq!(
        to_hex(&sha256(&repeated_a(56))),
        "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
    );
}

/// n=63: one byte below exact block size.
#[test]
fn sha256_63_bytes_matches_independently_computed_vector() {
    assert_eq!(
        to_hex(&sha256(&repeated_a(63))),
        "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34"
    );
}

/// n=64: exact block size; padding falls wholly in the second block.
#[test]
fn sha256_64_bytes_matches_independently_computed_vector() {
    assert_eq!(
        to_hex(&sha256(&repeated_a(64))),
        "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
    );
}

/// n=65: first genuine multi-block input beyond one 64-byte block.
#[test]
fn sha256_65_bytes_matches_independently_computed_vector() {
    assert_eq!(
        to_hex(&sha256(&repeated_a(65))),
        "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0"
    );
}

/// n=119 (55 + 64): the same first-block padding transition as n=55, one block later. Exercises a
/// different path through the length-accumulator and block loop than the first-boundary vectors.
#[test]
fn sha256_119_bytes_matches_independently_computed_vector() {
    assert_eq!(
        to_hex(&sha256(&repeated_a(119))),
        "31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb"
    );
}

/// n=120 (56 + 64): the same second-block-forcing transition as n=56, one block later.
#[test]
fn sha256_120_bytes_matches_independently_computed_vector() {
    assert_eq!(
        to_hex(&sha256(&repeated_a(120))),
        "2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c"
    );
}
