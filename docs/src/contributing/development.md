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
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run --locked -p prikk-release-policy -- check
```
