use super::lexical;

#[test]
fn rejects_alias_and_nonportable_segments() {
    for value in ["", "/absolute", "a//b", "a/./b", "a/../b", r"a\b", "a/b c"] {
        assert!(!lexical(value), "{value}");
    }
    for value in ["release/oracle/a.json", ".hidden", "...", "a..b"] {
        assert!(lexical(value), "{value}");
    }
}
