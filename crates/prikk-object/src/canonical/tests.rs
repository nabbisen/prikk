use super::CanonicalWriter;

#[test]
fn rejects_decreasing_tags() {
    let mut writer = CanonicalWriter::new();
    assert!(writer.field_u32(2, 1).is_ok());
    assert!(writer.field_u32(1, 1).is_err());
}
