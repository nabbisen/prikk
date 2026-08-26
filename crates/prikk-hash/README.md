# prikk-hash

Wraps the audited `sha2` crate for the SHA-256 hashing Prikk's object identities and canonical
encoding depend on, rather than shipping a first-party implementation. This crate is not meant to
be used as a dependency on its own; its Rust API may change without notice before `prikk` reaches
1.0. If you're looking for the tool itself, that's the `prikk` CLI.
