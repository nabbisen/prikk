use super::{authority_valid, fingerprint};

#[test]
fn authority_is_closed_and_sorted() {
    assert!(authority_valid(
        b"schema_version = 1\nauthorized_primary_fingerprints = []\n"
    ));
    assert!(!authority_valid(
        b"schema_version = 1\nauthorized_primary_fingerprints = []\nnote = \"x\"\n"
    ));
}

#[test]
fn fingerprints_are_uppercase_and_fixed_width() {
    assert!(fingerprint(&"A".repeat(40)));
    assert!(!fingerprint(&"a".repeat(40)));
    assert!(!fingerprint("AAAA"));
}
