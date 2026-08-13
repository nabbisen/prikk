# DC-74 §1/§4 — Prerequisite Questions, Answered

**Handoff followed:** `rfcs/handoffs/DC-74-merge-execution/implementation-handoff-v1.md`
**Governing RFC:** `rfcs/done/DC-74-MERGE-EXECUTION.md`

Per criterion 1 ("§4's four questions answered and reported **before** a design is proposed") this
reports before designing anything. No source files are changed by this document. Q1 and Q2 were
answered by constructing and running real scenarios through a temporary `#[test]` (the DC-67/DC-72/DC-73
harness pattern — `Command::current_dir`, binary resolved by `env!("CARGO_BIN_EXE_prikk")`), then
removed; `git status --short` confirms the tree is clean. Q3 is answered by the CLI surface as it exists
today. Q4 is answered by reading every construction site of `ConflictWitnessKind` and cross-referencing
against which operation kinds can actually be authored (established by DC-73: `CreateFile`,
file-`DeleteNode`, `EditText`, `ReplaceBinary`, `ChangePerm` — not `RenamePath`/`CreateSymlink`), with
one live confirmation rather than twelve.

## Q1 — Can a block seal a patch it did not author, with the signature intact? **Yes.**

**This is the question that could have ended the increment. It does not.**

Constructed live: genesis on `heads/main`; branch `heads/topic` from it; author A commits and seals a
new file on `heads/topic`, producing patch P with A's Ed25519 signature. Captured P's exact canonical
bytes and signature. Then, using only the primitives `seal.rs` itself already uses
(`derive_next_state_root`, `BlockPayload`, `RefStore::publish`) — no new code path — constructed a new
`Block` on `heads/main` whose `patch_ids` names P directly, signed the *block* with the maintainer key,
and published `heads/main` to point at it via ordinary CAS.

- `RefStore::publish` accepted it.
- Re-read P afterward: canonical bytes and signatures **byte-identical** to the original capture — no
  re-derivation, no re-signing occurred anywhere in the path.
- `verify_repository` accepted the whole repository, zero issues, both blocks and refs checked.
- `checkout --patch-materialize --ref heads/main` — a command that did not know P came from another
  branch — produced P's file content correctly.

Nothing on the seal or verify path re-derives, re-encodes, or re-signs patch bytes. A block's own
signature (maintainer) and a patch's own signature (author) are independent by construction: `seal.rs`'s
`persist_wal_patches` already writes patch envelopes verbatim without touching them, whether those
envelopes came from this session's own active WAL or, as demonstrated, from anywhere else the object
store can name. **B′ (adoption) is available.** The RFC does not need to return to the owner.

## Q2 — What does `merge-plan` actually emit? Run against a real divergence.

Constructed live: genesis on `heads/main`; branch `heads/topic`; each branch advances independently with
its own new file (worktree managed by hand between the two commits — DC-67's known "no branch-switch
command" gap applies to constructing test fixtures too, not only to real usage). Ran
`prikk merge-plan --baseline-block <genesis> --left-ref heads/main --right-ref heads/topic`.

Non-conflicting divergence output:

```
status: ConfluentSubset
evidence outcome: Confluent
reason: proven_confluent
action: review only; merge execution is not implemented
```

Conflicting divergence (both sides independently create the same path — produced once, unintentionally,
by the worktree-management slip above, and kept as evidence since it's a genuine two-independent-authors
conflict):

```
status: BlockedConflict
evidence outcome: Conflict
reason: pair_conflict
action: inspect evidence; conflict resolution is not implemented
cross:
  left[0] op_seq=1 CreateFile main-only.txt
  right[0] op_seq=1 CreateFile main-only.txt
  outcome: Conflict
  reason: pair_conflict
  phase: classification
```

**`merge-plan` already computes the correctness question — confluent or not — via `patch_algebra`'s
existing evidence machinery (`analyze_merge_evidence`) and simply stops at `action: review only;
merge execution is not implemented`.** It does not currently expose a direct list of "these are the
`ObjectId`s to adopt" — its report is operation-level (via `candidate_sequence`), not patch-level — but
the confluence verdict itself, the thing execution actually needs to gate on, is already sitting there
unused. This matters directly for scope: the RFC's own hint ("execution may turn out to be 'seal what
the plan already computes'") is not speculative — the *decision* is already computed; what's missing is
narrower than a merge algorithm, closer to "resolve the plan's verdict into a patch_ids set, then do what
Q1 just proved works."

## Q3 — Is merge-base discovery separable? **Yes, trivially — it's already manual.**

`--baseline-block` is a required, explicit argument on both `merge-evidence` and `merge-plan` today (used
directly above, with genesis's `ObjectId` read out of the repository by hand). Nothing in either command
attempts discovery. There is no code to separate out because none exists — v1 can keep requiring an
explicit baseline with zero change to the current surface.

## Q4 — What does `patch_algebra` return on conflict, and what's reachable?

All twelve `ConflictWitnessKind` variants have a live construction site somewhere in
`patch_algebra/{classify,create,delete,text_pair,text_preimage,preimage,witness}.rs` — none are dead
code in the "defined but never constructed" sense. Reachability *through two honest authors doing
ordinary work* is narrower, because only five operation kinds are ever authored (DC-73):

**Live-verified:**
- `SamePathCreate` — two independent `CreateFile` at the same path, different `node_id`s. Demonstrated
  above (unintentionally, via the worktree-management slip, then confirmed as genuine).

**Reachable by construction — not individually live-tested this round, but each has a real call site
gated only on same-`node_id` pairs among the five authored kinds** (`classify_same_node` dispatches to
`create.rs`/`delete.rs`/`text_pair.rs` by operation-kind combination, all of which pair up authored
kinds only):
- `ModeMismatch` — two `ChangePerm` on the same node from a common baseline, disagreeing mode.
- `BlobMismatch` / `KindMismatch` — two `ReplaceBinary` (or a `ReplaceBinary` against a same-node
  `EditText`) disagreeing on resulting content/kind.
- `LiveStateMismatch` — the broadest witness; constructed across `create.rs`, `delete.rs`, `preimage.rs`,
  `text_preimage.rs` whenever a preimage a patch expects doesn't match the baseline the other side
  already moved. The single most-constructed variant in the codebase (9 call sites) — almost certainly
  the most commonly reachable conflict in practice, not an edge case.
- `DeleteMutationConflict` — one side deletes a node, the other mutates it (`ChangePerm`, `ReplaceBinary`,
  or `EditText`).
- `TextSpanOverlap` / `TextAnchorStale` — two `EditText` on the same node whose spans overlap or whose
  anchors no longer resolve against the other side's result.

**Defined, constructed, but not reachable through ordinary two-author divergence:**
- `NodeIdReuse` — requires two independently authored creates to mint the *same* `node_id`. `node_id` is
  minted from `getrandom` (cryptographically random, 32 bytes); this is constructible by hand-crafting a
  patch, not by two honest authors working normally.
- `UnsupportedOperation` / `MalformedOperation` — gated by `deferred_reason`, which fires only for
  `RenamePath`/`CreateSymlink` (never authored — DC-73) or a structurally invalid decode. Neither arises
  from ordinary commits.
- `UnknownRelation` — the catch-all fallback when nothing else classifies the pair; not a distinct
  scenario to construct, a residual.

## What this means for scope, reported rather than assumed

- **B′ needs no new cryptographic or object-format mechanism.** Q1 proved the existing seal/verify/
  materialize path already accepts a foreign-authored patch adopted verbatim; the missing piece is code
  that *decides* to build such a block and *which* patches to put in it.
- **The confluence decision already exists** (Q2) — execution's job is narrower than re-deriving
  mergeability, it's turning an existing `Confluent` verdict into a patch_ids set and sealing it via the
  mechanism Q1 already demonstrated works.
- **Merge-base discovery is out of scope for free** (Q3) — nothing to separate, it was never coupled.
- **Conflict refusal (criterion 5) has one dominant, well-covered case to test against**
  (`LiveStateMismatch`, 9 call sites) plus the specific witness kinds each authored-operation-kind pair
  can produce — `NodeIdReuse` does not need a constructed test since it is not reachable through the
  surface merge execution will actually drive.

## Request

Report only, per criterion 1. Not proceeding to design. Requesting confirmation to proceed to §3's scope
(merge execution that adopts verbatim and seals, single-parent blocks, clean-refusing conflicts) before
writing any implementation code.
