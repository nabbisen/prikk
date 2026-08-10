# System Architecture

An overview of how Prikk is put together: which crate owns what, which way dependencies point, and
where the boundaries that matter are enforced.

For the objects themselves and how they change over time, see
[Data Model Relationships and Lifecycle](./data-model-lifecycle.md).

## Crate graph

Seven published crates. Dependencies point strictly downward — there are no cycles, and each crate
depends only on layers beneath it.

```mermaid
graph TD
    CLI["<b>prikk</b><br/>CLI surface"]
    STORE["<b>prikk-store</b><br/>repository, WAL, refs, verify, merge"]
    REPLAY["<b>prikk-replay</b><br/>node lifecycle state"]
    OBJECT["<b>prikk-object</b><br/>canonical encoding, object identity"]
    CRYPTO["<b>prikk-crypto</b><br/>Ed25519"]
    HASH["<b>prikk-hash</b><br/>SHA-256"]
    ERROR["<b>prikk-error</b><br/>error taxonomy"]

    CLI --> STORE
    CLI --> OBJECT
    CLI --> HASH
    CLI --> ERROR
    STORE --> REPLAY
    STORE --> CRYPTO
    STORE --> OBJECT
    STORE --> HASH
    STORE --> ERROR
    REPLAY --> OBJECT
    REPLAY --> HASH
    REPLAY --> ERROR
    CRYPTO --> ERROR
    OBJECT --> HASH
    OBJECT --> ERROR
```

| Crate | Owns | Does **not** own |
|---|---|---|
| `prikk-error` | The error taxonomy every layer returns | Anything else — it has no dependencies |
| `prikk-hash` | SHA-256, first-party since DC-55 | Object identity rules |
| `prikk-crypto` | Ed25519 signing and verification | Who is trusted, or when to sign |
| `prikk-object` | Canonical encoding, `ObjectId` derivation, payload shapes and their validation | Storage, I/O, policy |
| `prikk-replay` | Node lifecycle state — what exists, what is tombstoned | Where state is stored |
| `prikk-store` | The repository: object store, WAL, refs, verify, patch algebra, merge, filesystem durability | Command-line parsing and presentation |
| `prikk` | CLI surface, argument parsing, output | Any rule — it delegates every decision downward |

## Dependency boundary, enforced not documented

`prikk-store` may depend on exactly **`getrandom` and `rustix`**; `prikk-crypto` on **`ed25519-dalek`
and `getrandom`**; `prikk-hash` on **`sha2`**. Every other product crate has **no** third-party
dependencies at all.

This is not a convention. It is checked by `prikk-release-policy boundary-check`, which resolves the
real package graph from the root manifest and fails the build on any addition. Adding a dependency to a
product crate is therefore a reviewed decision, not an implementation detail.

## The mutation pipeline

Every change to sealed history follows the same path. Each stage is separately durable, and the
repository is consistent if the process stops between any two of them.

```mermaid
flowchart LR
    WT["Worktree<br/><i>ordinary files</i>"]
    WAL["Active WAL<br/><i>uncommitted patches</i>"]
    OBJ["Object store<br/><i>content-addressed</i>"]
    REF["Ref<br/><i>published tip</i>"]

    WT -- "commit<br/>author signs" --> WAL
    WAL -- "seal<br/>maintainer signs" --> OBJ
    OBJ -- "publish<br/>compare-and-swap" --> REF
```

- **commit** turns worktree differences into a signed patch in the active WAL. The **author** signs.
- **seal** persists the WAL's patches as objects and builds a block over them. The **maintainer** signs
  the block. Multiple commits may be queued and sealed together.
- **publish** advances the ref by compare-and-swap against its expected previous state, so a concurrent
  writer cannot be silently overwritten.

The two signatures are separate roles by construction: an author cannot seal, and a maintainer sealing
another author's work never re-signs that author's patches.

## Repository layout

Under `.prikk/`:

| Directory | Holds | Trust |
|---|---|---|
| `objects/` | Content-addressed objects, named by `ObjectId` | **Authoritative** |
| `refs/` | Ref states, the ref log, and recovery notes | **Authoritative** |
| `trust/` | Maintainer trust store — which keys may seal | **Authoritative** |
| `logs/` | Ref-log journal and log records | **Authoritative** |
| `cache/` | Rebuildable derived state | **Never a root of trust** |

The last row is a requirement, not an observation: **NFR-PERF-04** states that caches are rebuildable
and never roots of trust. `BlockSummaryCache` uses the canonical codec for reproducibility but is
explicitly excluded from block identity.

## Where the platform boundary sits

Read-only commands run on Linux, macOS, and Windows, verified continuously in CI. **Mutation is Linux
and macOS** — each platform's durability implementor lives behind one gated dispatch point
(`ACTIVE_DURABILITY`, DC-82), so a third platform is one more arm there, not a rewrite of the mutation
layer. Windows resolves to a stub implementor today and mutating there fails at runtime, not at build
time.

That is deliberate. DC-37 requires anchored opens that refuse symlink traversal, atomic replacement, and
explicit file and directory durability; those guarantees were implemented against Linux primitives
first (`LinuxDurability`) and macOS second (`MacosDurability`, DC-81), and have not yet been
re-established on any other platform. See [Platform Support](./platform-support.md).

## Where the unsafe-code boundary sits

Every crate in the workspace carries `#![forbid(unsafe_code)]`, applied uniformly through the root
`Cargo.toml`'s `[workspace.lints.rust]` table (`unsafe_code = "forbid"`) and each member's own
`[lints]` / `workspace = true`. The owner's ruling (DC-90) permits at most one workspace crate to be
named as an exception — never inferred from what a crate happens to do — and no crate is named today:
prikk writes no `unsafe` code of its own yet, even though it already *runs* some (`rustix`'s own
internal FFI on Linux and macOS, which `forbid(unsafe_code)` governs code prikk writes, not code it
depends on).

**The boundary is a gate, not a convention.** `release-policy boundary-check`
(`tools/release-policy/src/boundary/unsafe_boundary.rs`) fails the build if a second crate is ever
named exempt, if any non-exempt crate drops workspace lint inheritance, or — the rule that makes an
eventual exemption self-guarding — if the one exempt crate opts out of inheritance without locally
re-declaring `clippy::undocumented_unsafe_blocks = "deny"` in its own manifest. That lint is enabled
once, at the workspace root, specifically because the crate permitted to write `unsafe` is also the
one crate that could otherwise switch its own SAFETY-comment requirement off by deleting a line.

**What the gate cannot see, and the review obligation that covers it instead**, is documented in full
in `unsafe_boundary.rs`'s own module doc — read that before relying on a green `boundary-check` as
proof of anything it doesn't test. In short: FFI-ABI correctness (whether a foreign function
declaration actually matches the real platform ABI) and `SAFETY:` comment *content* are both human
review judgments, not machine-checkable properties, and comment *staleness* — a comment that no longer
justifies the code beneath it after an edit — degrades silently behind a gate that stays green either
way.

## Verification is the trust boundary

`prikk verify` re-derives rather than trusts: object ids are recomputed from canonical bytes, block state
roots are re-derived from lineage, and a merge block's recorded baseline is re-checked as a genuine
common ancestor of both parents.

Two limits are worth stating plainly, because they define what verification means here:

- Verification confirms **structural and cryptographic** validity. It does not re-derive that a change
  was semantically the *right* change — that rests on the maintainer's signature, uniformly, for merges
  exactly as for ordinary commits.
- Repository-wide **author** trust verification is not yet implemented, so a patch's author signature is
  carried and preserved but not checked repository-wide by `verify`.

## Known architectural costs

| Cost | Status |
|---|---|
| `prikk verify` is roughly **O(N³)** in sealed block count — 34 s at 160 blocks | Tracked, unowned |
| Node lifecycle state grows with cumulative history, not the current tree | Tracked; the project has no theory of forgetting yet |
| Mutation is Linux and macOS only, not Windows | Windows unimplemented, blocked on DC-88 (durability contract requirement shape) |
| Commit cost is not yet bounded independently of repository size (NFR-PERF-01) | Reduced, still missed |
| Merge complexity scoped to active block size (NFR-PERF-03) is **argued, not benchmarked** | Unowned |

These are recorded in `FINDINGS.md` in the repository rather than left implicit.

## What the block design trades, and what it does not

Patch-theoretic systems have a known failure mode: **Darcs's exponential merge**, which arises because
its patches are *context-dependent*. Reordering two of them requires **commuting** one into an equivalent
that applies in the other's context, and resolving conflicts means searching those orderings.

**Prikk cannot have that failure mode, by construction.** Its operations are context-free — every
operation names a stable `NodeId`, and `EditText` identifies its span by content anchors with
`presentation_hint_line` explicitly excluded from algebraic identity. A patch transports between
lineages **without transformation**, which is also why a merge can adopt patches byte-identically with
their author signatures intact. There is no commutation search to explode.

The second half is deliberate refusal rather than cleverness: the patch algebra proves confluence only
for a **conservative subset** it can prove, and returns a typed conflict witness for everything else.
**Cost is bounded by refusing hard cases, not by exploring them.** Sealing history into immutable blocks
then keeps that reasoning confined to the active working set, which is itself capped (NFR-PERF-02).

**But the trade is real, and it is worth stating plainly rather than leaving for someone to discover:**

> **The mechanism that bounds patch cost is the one that creates prikk's actual cost.** History is sealed
> into a chain carrying state roots, and `verify` re-derives that chain **from genesis, for every
> block** — which is exactly the O(N³) term above.

**Prikk did not inherit Darcs's problem. It has a different one, and it lives in the verification path
rather than the merge path.** That distinction matters strategically: verification is this project's
central claim in a way that merge throughput is not, so the cubic cost is a dependency of the claim
rather than a performance ticket beside it.

The fix is known and does not require a design change — memoize the lineage walk and reuse the
accumulated state across the per-block loop.
