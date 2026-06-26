#![forbid(unsafe_code)]

//! PRIKK command-line entry point scaffold.
//!
//! The initial binary intentionally exposes only a status banner. Persistent storage, refs,
//! patch algebra, plugins, and synchronization remain gated by their approved FDDs and later PRs.

fn main() {
    println!("prikk 0.1.0-pr002");
    println!("status: initial schema/object-identity scaffold");
    println!("implemented: canonical object IDs, envelopes, newtypes, memory store test boundary");
    println!("not yet implemented: WAL, refs, patch algebra, plugins, sync");
}
