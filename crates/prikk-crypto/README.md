# prikk-crypto

The single home for Prikk's Ed25519 keypair construction, detached signing, and detached
verification, so authoring, sealing, and verification can never compute a signature two different
ways. This crate is not meant to be used as a dependency on its own; its Rust API may change
without notice before `prikk` reaches 1.0. If you're looking for the tool itself, that's the
`prikk` CLI.
