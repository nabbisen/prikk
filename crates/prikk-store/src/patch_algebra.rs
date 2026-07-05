//! Internal patch-algebra classifier foundation (DC-16).
//!
//! This module intentionally exposes no public API. It classifies pairs of already-decoded patch
//! operations against a replayed lifecycle baseline, records deterministic reasons/witnesses, and
//! keeps deferred operation families fail-closed as `Unknown` instead of assuming commutation.

mod classify;
mod commutation;
mod create;
mod delete;
mod evidence;
mod evidence_types;
mod facts;
mod preimage;
mod replay_oracle;
mod report;
mod text_pair;
mod text_preimage;
mod types;
mod witness;

#[cfg(test)]
mod tests;
