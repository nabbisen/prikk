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
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run --locked -p prikk-release-policy -- check
```

The workspace declares Rust 1.85 as its minimum supported version. Verify that contract with the exact
minimum toolchain and locked dependency graph:

```sh
cargo +1.85.0 check --workspace --all-targets --locked
cargo +1.85.0 test --workspace --locked
cargo +1.85.0 build --workspace --locked
```

Strict Clippy remains a current-stable quality gate. It is not an MSRV gate because Clippy's lint set
changes with the toolchain.
