# prikk-error

The one place `PrikkError` and its variants are defined, so every other Prikk crate reports
failures through the same taxonomy instead of inventing its own. This crate is not meant to be
used as a dependency on its own; its Rust API may change without notice before `prikk` reaches
1.0. If you're looking for the tool itself, that's the `prikk` CLI.
