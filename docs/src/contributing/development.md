# Development

The implementation follows the design-first sequence:

1. Requirements and RFCs.
2. External design.
3. Foundational Design Documents.
4. Program design.
5. Implementation.
6. Testing and evidence.

Run the standard checks before submitting a source drop:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
