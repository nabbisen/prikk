# Worktree Status

`prikk worktree-status` reports read-only worktree status against the same replay-derived baseline
`prikk commit` would author against (RFC 122, `replay-baseline-handoff-v1.md`): the sealed
node-addressed lineage for the selected ref, with any already-queued (unsealed) patches folded on
top. It answers "what would the next commit author?", not merely "what differs from the last seal."

```sh
prikk worktree-status [path] [--ref REF]
```

It reports missing, modified, untracked, and unsupported paths. It does not write the worktree.
Patch generation is handled separately by `prikk commit --from-worktree`.

The scanner is intentionally conservative:

- `.prikk/` metadata is ignored;
- existing path-safety validation is reused;
- non-ASCII paths remain unsupported until Unicode NFC normalization is implemented;
- no writes are performed.

For the exact repository path validator rules, see
[path and worktree safety](../reference/path-safety.md). A path matched by a `.prikkignore` rule at
the repository root never appears in the untracked list at all — see
[Ignoring Worktree Paths](ignore.md).

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| `worktree-status` compares the worktree against the replay-derived baseline `commit` shares — the sealed lineage with any already-queued patches folded on top — not a stored snapshot Blob. | [`worktree_status.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/worktree_status.rs), [`patch_replay.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/patch_replay.rs) |
| It writes nothing and reports missing, modified, untracked, and unsupported-path changes. | [`worktree_status.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/worktree_status.rs) |

## Provenance

This guide covers RFC 122's rewire onto the replay baseline. It does not change repository state,
signing, trust, or the bundle/sync formats.
