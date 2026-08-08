# Merge

DC-74 adds `prikk merge`, the first command that executes a merge rather than only reporting on one.
It builds on the same read-only evidence [merge evidence](merge-evidence.md) and
[merge plan](merge-plan.md) already report — see those pages, and
[Patch Algebra and Merge Evidence](../reference/patch-algebra.md), for the underlying confluence
concepts.

```sh
prikk merge --allow-no-audit \
  --baseline-block BLOCK \
  --into REF \
  --from REF \
  [path]
```

- `--baseline-block` is required and names the sealed baseline block confluence is proven against.
- `--into` is the ref the merge advances. It must currently be published and must be the branch
  the caller has maintainer signing authority to seal.
- `--from` is the ref merged in. Its patches since the baseline are what get adopted.
- `--allow-no-audit` is required, matching `seal`'s own flag: this command signs and publishes new
  sealed history, and audit plugins are not implemented.
- The optional positional argument is the repository root.

## What a merge does — and does not do

**A merge authors nothing.** `--from`'s patches since the baseline are adopted **verbatim**: the
exact same canonical bytes, the exact same `ObjectId`, the exact same author signature as when they
were originally sealed. `prikk merge` never decodes, re-derives, or re-signs a patch. Only the new
`Block`, `RefState`, and `RefUpdate` are signed — with the maintainer key, exactly as an ordinary
`seal` signs them.

The two sides must be **proven confluent** from the given baseline — the same evidence
`merge-evidence`/`merge-plan` already compute, reused rather than duplicated. Any outcome other than
`Confluent` (`Conflict`, `Deferred`, `NotConfluent`, `Unsupported`, `OrderedDependency`,
`EvidenceFailure`, `InvalidCandidate`) refuses the merge. **Refusal writes nothing**: no object, WAL
record, or ref update of any kind is created until confluence is confirmed, so a refused merge leaves
`--into` exactly where it was.

## What gets recorded — read this before relying on it

**Merge blocks are `BlockKind::Normal`.** `BlockKind::Merge` exists in the object format, but
format-2's shape validator (`block_state.rs`) rejects both a `Normal` block with more than one
parent and any block of kind `Merge`, so this command seals single-parent `Normal` blocks — a merge
is, on disk, indistinguishable from an ordinary commit.

**There is no patch DAG to fall back on.** `PatchPayload.parent_patch_ids` exists in the wire format,
but every construction site sets it to empty, including the ordinary authoring path, and nothing
reads it. So a merge sealed under single-parent blocks leaves **no structural record that a merge
happened at all** — not in the block's parentage, not in the patch DAG. The only trace is `--from`'s
own ref history, which `branch close` can later remove. Authorship is unaffected (the adopted
patches still carry their original author's signature), but a later verifier cannot re-derive *what
the merge was checked against* — the baseline and the two sides — from sealed history alone.

This is why `MILESTONES.md` attaches a release condition to this command: **merge execution does not
ship until sealed history structurally records a merge**, re-checkable by a later verifier. That
condition gates release, not this command's use in development — build and merge normally. See
`rfcs/proposed/DC-75-MERGE-BLOCK-LINEAGE.md` for the increment that would close the gap (multi-parent
block lineage; not authorized, not built).

## Conflicts

Detection only. A resolution is itself a signed patch — a trust question `prikk merge` does not
decide. `patch_algebra`'s conservative subset (DC-16, DC-18) governs what can be proven confluent at
all; if it is too narrow to merge something that should be mergeable, that is its own finding, not
something this command works around.

## Compatibility

`verify`, `doctor`, `rollback-preview`, and DC-64's incremental lifecycle cache all continue to work
against a repository containing a merged block — tested, not argued
(`crates/prikk-cli/tests/dc74_merge_execution.rs`).

## Deferred

- Automatic merge-base discovery — `--baseline-block` stays explicit.
- Multi-parent block lineage and the structural merge record it would provide (DC-75, proposed).
- Conflict arbitration / resolution.
- Widening `patch_algebra`'s conservative subset.
- Merging more than two sides in one command.
