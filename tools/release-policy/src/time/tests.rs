use super::parse_utc_second;

#[test]
fn accepts_only_canonical_real_utc_seconds() {
    assert!(parse_utc_second("2026-07-17T12:34:56Z").is_some());
    assert!(parse_utc_second("2024-02-29T00:00:00Z").is_some());
    for value in [
        "2026-07-17T12:34:56+00:00",
        "2026-07-17T12:34:56.0Z",
        "2026-07-17T12:34:56z",
        "2023-02-29T00:00:00Z",
        "2026-07-17T12:34:60Z",
    ] {
        assert!(parse_utc_second(value).is_none(), "{value}");
    }
}
