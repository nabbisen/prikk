//! RFC 139 increment 1: the measurement-corpus profile format and its extractor.
//!
//! A profile is a small, human-readable TOML document describing a real history's *shape* --
//! never its content, never its paths. Increment 2 adds the builder: a deterministic program that
//! reads a profile and materializes a throwaway prikk repository from it, on demand, driving the
//! same `prikk` CLI a user would. This crate is a library with a thin binary over it (RFC 139 §7)
//! precisely so increment 2's builder, `tools/benchmarks`' criterion benches, and the `#[ignore]`d
//! integration harnesses under `crates/prikk-cli/tests/` can all depend on the same profile types
//! rather than each parsing TOML by hand.

pub mod extract;
pub mod profile;

pub use extract::{ExtractError, ExtractionContext, extract_profile};
pub use profile::{BuilderInputs, OperationKindMix, Profile, Provenance, SCHEMA_VERSION, Shape};
