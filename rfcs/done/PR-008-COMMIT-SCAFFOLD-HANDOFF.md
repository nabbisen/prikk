# PR-008 Commit Scaffold Handoff

## Scope

PR-008 adds the first user-visible active commit path without implementing real diff capture or
seal. The command is deliberately narrow:

```sh
prikk commit --allow-empty -m "message"
```

It creates a signed patch envelope, acquires the default active-session lock, appends the envelope
to `active/default/queue.wal`, fsyncs through the WAL layer, and returns the assigned WAL sequence.

## Design Boundaries

- This is a storage/WAL integration increment, not the final VCS commit model.
- The command requires `--allow-empty` to avoid implying real worktree diff capture exists.
- The generated patch payload contains no operations and uses a development-only precondition entry
  to make the payload identity message-sensitive.
- The placeholder author signature is structural only. Real Ed25519 signing belongs to the crypto
  identity increment.

## Acceptance / QA

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Manual smoke test:

```sh
cargo run -p prikk -- init /tmp/prikk-pr008-smoke
cd /tmp/prikk-pr008-smoke
cargo run -p prikk -- commit --allow-empty -m "first scaffold"
cargo run -p prikk -- status
cargo run -p prikk -- verify .
```

Expected behavior:

- `commit` reports a patch ID and WAL sequence `1` on a new repository.
- `status` reports one active WAL record.
- `verify` reports one checked WAL record and no trailing partial bytes.

## Deferred

- real worktree diff capture
- durable patch parent dependency selection
- cryptographic key management / Ed25519 signing
- seal transaction
- patch apply/inverse/commutation
