# prikk-object

Defines Prikk's object identity and canonical payload encoding with a small, deterministic
encoder of its own rather than protobuf bytes. This crate is not meant to be used as a dependency
on its own; its Rust API may change without notice before `prikk` reaches 1.0. If you're looking
for the tool itself, that's the `prikk` CLI.
