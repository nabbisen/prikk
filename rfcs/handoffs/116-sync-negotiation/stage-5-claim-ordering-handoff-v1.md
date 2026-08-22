# RFC 116 stage 5 — ordering claims for sealing: implementation handoff

**Design:** `rfcs/handoffs/115-sync-investigation/design-v1.md` §11 (D6) and RFC 116's own N3 —
**the field this increment finally uses was added for exactly this purpose.**
**Base:** current `main` (`194f090`). **Closes the last known gap in the sync loop.**

**Why this exists, stated plainly:** N3 added `parent_block_ids` to the recognition claim so that
inter-claim seal order would be **derivable from signed data by topological sort**. The field is
carried, and compared against a held block for consistency — **and no code anywhere sorts by it.** I
deferred the sort in N3's handoff to "RFC 116 stage 2", stage 2 turned out to be summary/have-list/delta,
and it was never scheduled again. So today a multi-block sync hands the operator N claims and
`sync seal --claim <id>` takes them one at a time **in an order nobody computes.**

**Confirmed while writing this:** the sender emits claims in `ancestors_inclusive`'s `BTreeMap` order —
sorted by `ObjectId`, which is arbitrary with respect to lineage. So the artifact's claim order is
**not** already topological, and cannot be relied on.

---

## 1. The library function

```
order_claims_for_sealing(object_store, claim_ids: &[ObjectId]) -> Result<Vec<ObjectId>>
```

in `prikk-store`, beside `recognition_claim.rs`. Decode each claim, build a graph over the batch's
`block_id`s using `parent_block_ids`, and return the claim ids in an order where **a claim's block is
sealed after every claim in the same batch whose block is one of its parents.**

### 1.1 Rulings

- **Only intra-batch edges matter.** A claim's block may name parents that are not among the batch's
  blocks — those are already-sealed equivalents, or simply absent. **Ignore them for ordering**; do not
  refuse, do not attempt to fetch them. Refusing would break the ordinary incremental case, where the
  parent was sealed by a previous sync.
- **The result must be deterministic.** Independent chains can interleave arbitrarily; break ties by
  `ObjectId` so two runs over the same batch produce the same order. Non-determinism here would be
  untestable and would make two receivers diverge for no reason.
- **A cycle is a refusal, and this is a security property, not a tidiness one.** Blocks are
  content-addressed and genuinely form a DAG, so an honest batch cannot contain a cycle. **But a claim
  is an assertion, and a hostile sender can assert one** — claim P says block B's parent is C, claim Q
  says C's parent is B — and a receiver holding neither block cannot disprove it. **The sort must
  terminate and refuse**, naming both claims. It must not loop, and must not silently drop an edge to
  make progress.

## 2. The CLI, and a consistency fix that is mine to own

`sync accept` prints claim ids to stdout, and `sync seal --claim <id>` consumes them — **the only place
in this binary where one command's output feeds the next command's argument**, and the dev team
correctly flagged that the end-to-end test has to string-split stdout to bridge it.

**That inconsistency is my design error.** Every other step in `sync` moves data by file: summary,
have-list, artifact. Claim ids were the one exception, for no reason.

- **`sync accept <file> --claims-out <file>`** — write the accepted claim ids to a file.
- **`sync seal <ref> --claims <file>`** — read them, order them with §1, and seal each in turn.
- **Keep `sync seal <ref> --claim <id>`** for the single-claim case. Do not remove it.

`seal_from_accepted_claim` itself is **unchanged** — it still takes one claim. The sort orders the calls;
it does not change what a call does.

**Report per claim as you go** — `Sealed` / `AlreadySealed`, in the order executed. A run that stops
partway must leave the operator able to see exactly how far it got.

## 3. Behaviour when a seal in the middle fails

**Ruled: stop at the first failure and report it, leaving the successful seals in place.**

Do not roll back. Each seal is an independent, legitimate act under the receiver's own key — the blocks
already sealed are correct history and unwinding them would be a rewrite, which this project does not
do. Report which claims were sealed, which failed and why, and which were not attempted.

This composes with D7: re-running the same claims file after fixing the cause skips what is already
sealed and continues.

## 4. Tests and controls

Each needs a test **and** an observed-failing control.

| # | Property | Control |
|---|---|---|
| 1 | A two-block batch seals parent-first regardless of input order | Return the input order unsorted → the child-first case fails |
| 2 | Ordering is by `parent_block_ids`, not by artifact or id order | Sort by `ObjectId` instead → the case where lineage and id order disagree fails |
| 3 | Parents outside the batch are ignored, not refused | Refuse on an unknown parent → the ordinary incremental case fails |
| 4 | A hostile cycle is refused, naming both claims | Drop the cycle check → the test hangs or completes with a wrong order |
| 5 | The order is deterministic across runs | Introduce a non-deterministic tie-break → repeated runs disagree |
| 6 | A mid-batch failure stops, reports, and leaves earlier seals intact | Roll back on failure → the earlier-seals-intact assertion fails |
| 7 | End to end: a **multi-block** sync completes through the CLI alone | Remove the sort from `sync seal` → the round trip fails |

**Row 7 is the one that matters**, and it is the case the whole sync arc has never exercised: every
end-to-end test so far has been single-block. Build a sender with **two** sealed blocks where the second
depends on the first, sync to an empty receiver, and assert the receiver's ref tip reaches both patches.

**Row 2 needs care.** Construct a batch where lineage order and `ObjectId` order genuinely disagree,
or the test cannot distinguish a real topological sort from an incidental one. If they happen to
agree, the test proves nothing — the same trap that made the `parent_block_ids` control a no-op two
increments ago.

**Row 4 must not be a slow test.** Assert the refusal, not a timeout.

## 5. Out of scope

- **Changing `seal_from_accepted_claim`.** It stays per-claim.
- **Changing any wire format or the claim schema.** The schema window is closed.
- **Transport.** Still optional, still RFC 116 ruling 2.
- **A JSON output mode for the CLI.** §2's file-based fix removes the need here; a general JSON mode is
  its own question for the whole binary, not this increment.
- **Ordering across refs.** Claims are per-ref by construction (design §1.2 as amended); a claims file
  belongs to one ref, and `sync seal` names that ref.

## 6. What to report

1. Control output for every row of §4 — actual failure text, and the single line mutated.
2. **For row 2:** how you made lineage order and `ObjectId` order disagree, and how you confirmed they
   disagree in the fixture rather than assuming it.
3. **Row 7's end-to-end multi-block run in full**, including what you read back from the receiver.
4. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
5. Test counts before and after, per crate. **`snapshot.txt` must not change.**
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: a cycle turns out to be unconstructible in a test, which would
mean §1.1's threat model is wrong; ordering needs data the claim does not carry, which would mean N3 is
still short a field **and the schema window is closed**, so I need to know immediately; or row 7 cannot
be driven through the CLI alone.
