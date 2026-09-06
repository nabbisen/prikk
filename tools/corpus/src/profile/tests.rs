//! Handoff §6 control 6: the committed prikk profile round-trips -- parsed, and the parse agrees
//! with the file. This is the check that the format is actually readable by the thing that will
//! read it, before increment 2 depends on it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

const COMMITTED_PROFILE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/profiles/prikk-self.toml"
));

#[test]
fn control6_the_committed_prikk_profile_round_trips() {
    let parsed: Profile = toml::from_str(COMMITTED_PROFILE)
        .expect("the committed profile must parse as this crate's own Profile type");
    assert_eq!(parsed.schema_version, SCHEMA_VERSION);
    assert_eq!(parsed.provenance.source_repository, "prikk (self)");
    assert!(
        !parsed.provenance.extraction_commands.is_empty(),
        "provenance must name at least one extraction command"
    );
    assert!(parsed.shape.commit_count > 0);

    let re_rendered =
        toml::to_string_pretty(&parsed).expect("a parsed profile must re-render as TOML");
    let re_parsed: Profile = toml::from_str(&re_rendered)
        .expect("the re-rendered profile must itself parse as this crate's own Profile type");
    assert_eq!(
        parsed, re_parsed,
        "parse -> render -> parse must agree with the original parse"
    );
}
