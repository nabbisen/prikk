# PR-009 Seal Scaffold Handoff

## Scope

PR-009 adds a minimal local seal path for exercising the storage/ref foundation:

- read the active WAL while holding `active.lock`
- reject incomplete trailing WAL records
- persist signed patch envelopes into `objects/patch/`
- create a signed Block envelope
- publish `heads/main` via signed RefState and inline RefUpdate records
- truncate the active WAL only after publication succeeds

The command is deliberately explicit:

```sh
prikk seal --allow-no-audit
```

## Non-Scope

PR-009 does not implement:

- real worktree diff capture
- real state-tree materialization
- policy/audit plugin execution
- attestation checks
- patch apply/inverse/commutation
- remote sync

## Review Notes

Reviewers should focus on whether the PR preserves the durability order:

1. WAL entries already exist and are signed.
2. Patch objects are persisted before the Block references them.
3. The Block object is persisted before RefState targets it.
4. RefState publication uses the existing ref-specific CAS primitive.
5. The WAL is truncated only after ref publication succeeds.

The state Merkle root remains a deterministic scaffold root and must not be mistaken for final
worktree materialization.

## Suggested Checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p prikk -- init /tmp/prikk-pr009-smoke
cd /tmp/prikk-pr009-smoke
../target/debug/prikk commit --allow-empty -m "one"
../target/debug/prikk status
../target/debug/prikk seal --allow-no-audit
../target/debug/prikk status
../target/debug/prikk verify
```
