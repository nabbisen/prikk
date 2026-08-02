# DC-66 Prerequisite Questions — Answered Before Design

Per the RFC §4 and the handoff §2: all four answered by reading the code and one probe, before any
design was proposed. Line references are against `250ad54` (the DC-65 candidate accepted as of this
increment's start).

## 1. Does the second queued patch author against the first, or against the last sealed state?

**Today, if the three guards were simply deleted and nothing else changed: against the last sealed
state — and that is wrong.** Read `resolve_worktree_baseline`
(`crates/prikk-store/src/patch_replay.rs:205-242`): it derives `WorktreeBaseline` purely from
`RefStore::read_current_ref_state_id`, i.e. the last **published** ref state. It has no knowledge of
`active_replay` (the WAL replay) at all — `author_inner` computes `active_replay` only to run the
guard, then discards it. `resolve_baseline_state`/DC-64's incremental cache is likewise keyed only off
`(baseline_block_id, horizon_id)`, both derived from the same published-only source.

**Why "last sealed state" is not merely a different design point but actually broken:** consider a
worktree file created in queued commit 1 (not yet sealed), then commit 2 runs before any seal. Commit
2's `baseline_files` (built from `baseline_state.live_nodes()`) would **not** include the file commit 1
created, because that state was never folded in. Commit 2 would see the path as "no baseline node" —
the genuinely-new-file branch (`node_authoring.rs:364-384`) — and mint a **second, different**
`node_id` for the same path via a second `CreateFile`. Two live nodes would then claim the same path
within one queue. That is a direct violation of node identity (RFC criterion 3) and of `NodeLifecycleState`'s
own `path_to_id` invariant, not a stylistic choice between two valid designs.

**Established rule: queuing is a chain.** The second queued patch must author against the first's
result, and so on transitively. Node identity, DC-65's text materialization, and conflict behaviour
all follow from this, exactly as the RFC's own framing anticipated. The **baseline-for-the-next-queued-patch
rule** (RFC criterion 2) is: the effective baseline for commit *k+1* is the sealed baseline
(`resolve_baseline_state`, unmodified, still DC-64-accelerated) with commits `1..k`'s own operations
folded on top, in WAL append order, via the same `apply_state_effect` fold every other replay path
uses. Full rule and its DC-65 interaction: see `queuing-baseline-design-v1.md`.

## 2. Does `require_active_ref_for_non_empty_wal` still express the right invariant with N?

**Yes, unchanged.** Read `crates/prikk-store/src/active.rs:180-197`: the function checks the active
ref metadata file's recorded ref name against the ref being targeted — it is entirely record-count-
agnostic. It answers "does everything currently in the WAL belong to one ref," which is exactly as true
for N queued records as for 1: the metadata is written once, on the *first* append to an empty WAL
(`prepare_empty_active_ref_for_append`, called by both `active.rs:74` and `node_authoring.rs:505`), and
checked on every subsequent append regardless of how many records already exist. No change needed to
this function. What changes is only the caller: today a non-empty WAL means "reject," under queuing it
means "call this guard, then proceed to author against the chained baseline" instead of returning an
error.

## 3. What does `doctor --repair-wal-tail` mean when truncation discards one of N?

**The mechanism already generalizes correctly; only the reporting needs to say more.** Read
`decode_records` (`crates/prikk-store/src/wal.rs:285-326`): it is a `while offset < bytes.len()` loop
that decodes every complete record from the start, stopping only when fewer than `WAL_HEADER_LEN` bytes
or a short body remain — at that point the **remaining** bytes (not "the last record") are reported as
`trailing_partial_bytes`. `truncate_trailing_partial` (`wal.rs:163-192`) truncates the file to
`current_len - trailing_partial_bytes`, i.e. it removes only the torn tail and preserves every complete
record that decoded successfully, whatever their count. This was already true before DC-66 and needed
no change for correctness — DC-38's crash-recovery design already built the *mechanism* for a
multi-record log even though nothing above it ever produced more than one record.

**What was actually missing, matching handoff §3 and RFC criterion 5:** `WalRepair` reports only
`preserved_records: usize` and `truncated_bytes: usize` — counts, not identities. For N = 1 this was
adequate ("the one record" is unambiguous). For N > 1, "3 records preserved" does not tell an operator
*which* three authors' work survived, and criterion 5 requires the repair to **say what it did**, not
just do the safe thing silently. This increment extends the report with the preserved records' patch
ids (and, where derivable, their author key ids) so a repair against a queue is auditable, not just
safe. See §5 of `queuing-baseline-design-v1.md`.

## 4. Can DC-64's `apply_one_block` handle a block with N patch ids today?

**Yes, already, with no code change required for this specific question.** Read
`crates/prikk-store/src/lifecycle_cache/replay.rs:320-340`: `apply_one_block` constructs one
`TextCache` and calls `apply_patch_ids(reader, &block.patch_ids, ...)` — and `apply_patch_ids`
(`replay.rs:389-404`) is `for patch_id in patch_ids { ... for operation in &operations { apply_state_effect(...) } }`,
a plain loop over however many ids the block carries. `BlockPayload.patch_ids: Vec<ObjectId>` was
never `Option<ObjectId>` or a fixed-size field. Nothing here assumes exactly one patch. The shared
`TextCache` spans the whole loop, which is actually the mechanism that makes a **sealed** batch of N
chained text edits resolve correctly in one incremental step: patch 2 in the same block can read
patch 1's materialized text straight out of that call's own cache, no different from how it already
resolves a multi-operation patch's own internal ordering.

**What this does *not* answer, and what actually needs new code:** this question is about the
**sealed** path (`apply_one_block`, invoked from `try_incremental_step` once a block already exists
with N patch ids — i.e. after `seal`). The **unsealed** path — computing the effective baseline for
queued commit *k+1* while commits `1..k` are still sitting in the WAL as bare envelopes, not yet
written as `Patch` objects (`persist_wal_patches`, `crates/prikk-cli/src/seal/support.rs:11-24`, runs
only at `seal` time) — has no equivalent today, because nothing before DC-66 ever needed to fold
unsealed operations into a baseline at all. That is the real content of RFC §1's "single most important
question," designed in `queuing-baseline-design-v1.md`.
