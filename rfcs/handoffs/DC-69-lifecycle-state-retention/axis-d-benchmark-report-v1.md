# DC-69 §3.4 — Axis D: Cost at Long History, Small Tree

Tree size held fixed at **20 files** across every point. The varying quantity is history depth: the number of sealed churn generations (delete oldest tracked file, create one new file at a fresh path — net live tree size unchanged) before the timed commit. `3` independent repositories per depth. See `crates/prikk-cli/tests/dc59_commit_benchmark.rs`'s DC-69 section for the full method and why churn (not edits) is required to exercise `seen_ids`/`latest_tombstone_by_id` growth.

| History depth | Live tree size | Median | Min | Max |
|---:|---:|---:|---:|---:|
| 10 generations | 20 files | 2.66 ms | 2.55 ms | 2.84 ms |
| 50 generations | 20 files | 5.47 ms | 5.47 ms | 6.05 ms |
| 100 generations | 20 files | 9.01 ms | 8.97 ms | 9.51 ms |
| 200 generations | 20 files | 17.91 ms | 16.04 ms | 20.99 ms |

**Reading this table:** if the timed commit's cost at depth 200 is materially higher than at depth 10, with live tree size identical (20 files) at every row, that cost is attributable to cumulative history — `seen_ids`/`latest_tombstone_by_id`'s unbounded growth — not to repository size, which no prior DC-59/62/64 axis isolates.

## Reproduction

```
cargo test -p prikk --locked --test dc59_commit_benchmark -- --ignored --nocapture axis_d_long_history_small_tree
```
