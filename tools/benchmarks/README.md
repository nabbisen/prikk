# prikk-benchmarks

Repository-internal performance benchmarks (RFC 126 §5, under the owner's §6 ruling: criterion in
its own workspace member, outside `default-members`). Not part of the shipped product, not built by
`cargo build`/`cargo build --workspace` without `-p`, and not published.

## Running

```sh
cargo bench -p prikk-benchmarks
```

Criterion stores a baseline under `target/criterion/` between runs and reports the change against
it on the next run. The baseline is local to your machine and is not committed.

## Scope

One benchmark today: `commit`, a genesis `prikk commit` against a small fixed-size worktree,
measured in-process (calling `commit_worktree_changes_signed` directly, not spawning the compiled
binary). It measures wall-clock time only -- it does not measure peak RSS, which is a different,
unbuilt mechanism.

This member exists to give criterion somewhere to live without adding it, or its dependency tree, to
any product crate's manifest or to the shipped dependency graph. `cargo test --workspace` still
builds it, the same way it already builds `tools/release-policy`; only `default-members` (and so a
bare `cargo build`) excludes it.
