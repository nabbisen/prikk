# PR-019 Worktree Patch Draft Handoff

## Scope

PR-019 adds a minimal worktree-to-patch draft path:

- `prikk commit --from-worktree -m <message>`
- baseline: the current snapshot-backed `heads/main` ref by default
- operations emitted:
  - `CreateFile` for untracked regular files
  - `DeleteFile` for missing tracked files
  - `ReplaceBinary` for modified tracked files
- generated Blob objects are written before the signed Patch envelope is appended to the active WAL

## Non-goals

- no rename detection
- no text-span `EditText` generation
- no patch replay checkout
- no merge/commutation algebra
- no audit plugin execution
- no remote sync

## Review focus

- generated operation order is deterministic and uses contiguous `op_seq`
- unsafe paths are rejected by the existing path/status checks
- operation-referenced blobs are persisted before the patch WAL append
- the commit path remains WAL-durable and active-lock protected

## Suggested checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
