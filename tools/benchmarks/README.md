# prikk-benchmarks

Repository-internal performance benchmarks (RFC 126 §5, under the owner's §6 ruling: criterion in
its own workspace member, outside `default-members`). Not part of the shipped product, not published,
and built only by a command that passes `--all-targets` — see the note at the end of Scope.

## Running

```sh
cargo bench -p prikk-benchmarks
```

Criterion stores a baseline under `target/criterion/` between runs and reports the change against
it on the next run. The baseline is local to your machine and is not committed.

**A baseline is only comparable to a run with `$TMPDIR` on the same filesystem.** The benchmark's
timed region includes the fixture's `commit`, which performs real durability work (`fsync`), so the
backing filesystem's own fsync latency dominates the number far more than the code being measured
does. Measured on one machine, same commit, same code:

| `$TMPDIR` | Filesystem | `commit_genesis_10_files` |
|---|---|---|
| unset (falls back to `/tmp`) | tmpfs | `[184.69 µs 185.52 µs 186.59 µs]`, ~15k iterations |
| `<repo>/.git-exclude/tmp` | btrfs | `[15.721 ms 15.781 ms 15.852 ms]`, 100 iterations |

**~85× apart, from `$TMPDIR` alone.** `rfcs/EXECUTION-ORDER.md` §6 rule 9 already tells you to move
`$TMPDIR` to `.git-exclude/tmp` when `/tmp` is read-only — if you do that after a baseline was taken
on `/tmp`, the next run reports criterion's own verdict as a regression this large (`+8000%` or more
is typical, not a rounding artifact) against code that has not changed at all. **Pin `$TMPDIR`
deliberately before comparing two runs, and discard the stored baseline (`rm -rf
target/criterion/commit_genesis_10_files`) whenever you change it** — a baseline taken under a
different `$TMPDIR` is not wrong, it is simply not the same measurement.

## Scope

One benchmark today: `commit`, a genesis `prikk commit` against a small fixed-size worktree,
measured in-process (calling `commit_worktree_changes_signed` directly, not spawning the compiled
binary). It measures wall-clock time only -- it does not measure peak RSS, which is a different,
unbuilt mechanism.

This member exists to give criterion somewhere to live without adding it, or its dependency tree, to
any product crate's manifest or to the shipped dependency graph.

**What actually builds it is `--all-targets`, not workspace membership.** This package has no lib and
no bin -- a single `[[bench]]` target is all there is -- and cargo does not build bench targets
without `--all-targets` (or `cargo bench`). Measured by planting a syntax error in
`benches/commit.rs` and running each command:

| Command | Compiles the bench? |
|---|---|
| `cargo build` | no |
| `cargo build --workspace` | no |
| `cargo build -p prikk-benchmarks` | **no** -- naming the package does not help |
| `cargo test --workspace --locked` | no |
| `cargo build --workspace --all-targets` | yes |
| `cargo clippy --workspace --all-targets ...` | yes |
| `cargo +1.85.0 check --workspace --all-targets --locked` | yes |

**`default-members` is not what excludes it** from `cargo build --workspace`: that flag overrides
`default-members` entirely. `default-members` governs only a bare `cargo build`, and even there it is
moot, since this package has no target a plain build would produce. **The distinction matters because
it is the reason the standing gate set names the `--all-targets` commands** -- `cargo test
--workspace` passing says nothing about whether this member compiles.
