# Development

The implementation follows the design-first sequence:

1. Requirements and RFCs.
2. External design.
3. Foundational Design Documents.
4. Program design.
5. Implementation.
6. Testing and evidence.

Release preparation follows the separate
[release, versioning, and compatibility](../reference/release-compatibility.md) policy. A listed gate is
not passing evidence unless it was observed for the exact commit or release under review.

Run the standard checks before submitting a source drop:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo run --locked -p prikk-release-policy -- check
```

## Building the documentation

The book uses Mermaid diagrams, which are rendered by the `mdbook-mermaid` preprocessor. Both tools are
needed to build it:

```sh
cargo install mdbook --no-default-features --features search --vers "^0.5" --locked
cargo install mdbook-mermaid --vers "^0.17" --locked
mdbook build docs
```

`mdbook build` fails with a clear message if the preprocessor is missing, so a stale toolchain cannot
silently produce diagrams as code blocks. The Mermaid assets are vendored under `docs/`, so the built
book renders offline and fetches nothing.

The workspace declares Rust 1.85 as its minimum supported version. Verify that contract with the exact
minimum toolchain and locked dependency graph:

```sh
cargo +1.85.0 check --workspace --all-targets --locked
cargo +1.85.0 test --workspace --locked
cargo +1.85.0 build --workspace --locked
```

Strict Clippy remains a current-stable quality gate. It is not an MSRV gate because Clippy's lint set
changes with the toolchain.
