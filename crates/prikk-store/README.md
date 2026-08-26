# prikk-store

Everything `prikk` does to a repository on disk lives here: layout, object storage, durability,
verification, merge evidence, and patch replay. This crate is not meant to be used as a dependency
on its own; its Rust API may change without notice before `prikk` reaches 1.0. If you're looking
for the tool itself, that's the `prikk` CLI.
