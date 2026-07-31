# DC-65 §1 — Prerequisite Questions, Answered

Required before designing a fix (RFC §4, handoff §1). All four answered by reading the actual code
paths involved, not by inference from the symptom.

## Q1. Does `checkout`/materialization have the same defect?

**No.** `crates/prikk-store/src/patch_replay/apply.rs`'s `apply_edit_text` — the function checkout
materialization uses — maintains `files: BTreeMap<String, Vec<u8>>`, the actual materialized bytes
for every path, accumulated across the *entire* replay pass from the chain's snapshot/genesis
baseline forward. An `EditText` operation reads `files.get(&live.path)` (the in-memory current text,
already correct because every earlier operation in the same pass already updated it) and writes the
spliced result back into the same map. It never reads a "current blob" by ID at all — the map is
keyed by path, not by blob identity, and the map's up-to-dateness is a property of processing the
chain in order within one pass, not of any object being durably stored.

`crates/prikk-store/src/lifecycle_cache/replay/effect.rs`'s `apply_edit_text` (the lifecycle-state
replay used by the commit path, both DC-64's incremental step and the full-replay fallback) is
narrower but has the same property for the case that matters: its `text_cache: &mut TextCache` is
created once per replay pass and threaded through every block in the chain, so a later block's edit
to an already-edited node hits the cache rather than needing the intermediate blob to be stored. A
cache *miss* falls back to `blob_resolver.blob_content(&current_blob_id)` — but a miss can only reach
a not-yet-cached blob id, and for any patch that could actually have been sealed, the *first* time a
node is edited its `current_blob_id` is real (the create-time blob), so full replay has always been
correct. (DC-64's incremental step creates a *fresh* `TextCache` per step rather than one threaded
across the whole lineage; whether this matters is analyzed under Q4/design below, since no patch that
would expose it can exist while the authoring-side bug stands — see the design document.)

## Q2. Is `ReplaceBinary` affected?

**No.** `crates/prikk-store/src/worktree_patch/node_authoring.rs`'s `plan_replace_binary`
unconditionally calls `write_content_blob(object_store, BlobKind::Binary, new_bytes)` for every
replace — `write_content_blob`'s only two call sites in the whole file are here and fresh `CreateFile`
(`:396`, `:569`). So `new_blob_id`, which becomes the *next* edit's `base.blob_id`, always names a
real, stored `Blob` object. `plan_replace_binary` also never reads old content by blob id for
planning — the worktree-vs-baseline comparison happens via content hash (DC-56's commit-index cache),
never a direct blob read of the *old* value.

## Q3. Do `merge_evidence`/`patch_algebra` read baseline content the same way?

**Yes — affected by the same defect.** `crates/prikk-store/src/patch_algebra/evidence.rs`'s
`StorePatchAlgebraEvidence::baseline_text` (reached from `text_preimage.rs:53`, used by merge-evidence
preimage/conflict validation for text spans) calls `self.read_blob(scope, fact, blob_id)`, which calls
`self.reader.read_object(blob_id)` directly — the same shape of read `plan_edit_text` makes. `blob_id`
here comes from `baseline_state.live_node(&node_id).content`, and `baseline_state` is built through
`replay_derived_state` (line ~50-60 of the same file). For a text node whose most recent recorded
operation was an `EditText`, that `blob_id` is the same computed-but-never-stored identity. **Any
merge-evidence request against a text file with edit history beyond its first edit will fail the same
way**, once such history can exist (see Q4 — it currently cannot, because authoring fails first).

## Q4. Is a node's `blob_id` supposed to name a stored object, or not?

**No — for a `TextFile` node, `blob_id` is a content identity, not necessarily a stored object.**
Reasons, weighed rather than picked by convenience:

1. **`EditText`'s wire shape is a diff** (`replacement_text`, `old_span_hash`, anchor hashes — never a
   full-content field). Requiring every edit to also durably store the full resulting text as a
   `Blob` would mean every text edit's storage cost is O(file size), not O(edit size) — for a file
   edited many times, that is exactly the kind of unbounded-with-history growth this program has
   fought to eliminate elsewhere (DC-56, DC-64). Nothing in `EditText`'s design suggests that was ever
   intended.
2. **Both correct consumers already encode this invariant.** `patch_replay/apply.rs` (checkout) and
   `lifecycle_cache/replay/effect.rs` (lifecycle-state replay, both full and DC-64 incremental) both
   *materialize* current text from the diff chain rather than assuming a stored object. They are not
   working around the defect — they were built this way from the start and have always been correct.
   `plan_edit_text` and `patch_algebra::baseline_text` are the only two places that assume otherwise.
3. **The alternative (materialize-and-store on every edit) would still need a design for *when* — at
   authoring time only, or backfilled for existing history?** — and would move every existing
   `EditText`-descended node's *conceptual* content representation without moving any `ObjectId`
   (blob ids referenced by sealed `EditText` operations are the operation's own recorded identity,
   used for comparison, not dereferenced by replay) — but would add new `Blob` objects nothing
   currently forces to exist. That is a real design surface (retroactive backfill, storage growth) for
   a problem the diff-based design already avoids by construction.

**Chosen invariant, stated for both halves to see:** *a `TextFile` node's `blob_id` is the content
identity current worktree/replayed text would hash to under `BlobPayload::new(BlobKind::Text, …)` —
it is never assumed to name a persisted `Blob` object, and any code that needs the node's actual
current text bytes must materialize them by replaying the node's edit history (as `patch_replay` and
`lifecycle_cache::replay` already do), never by a direct object read.* `CreateFile` and
`ReplaceBinary` blob ids remain real stored objects — this invariant is specific to `TextFile` content
identities born from `EditText`, since that is the only operation whose wire shape is a diff rather
than a full content reference.

The fix therefore brings `plan_edit_text` (`node_authoring.rs`) and `baseline_text`
(`patch_algebra/evidence.rs`) into line with what `patch_replay`/`lifecycle_cache::replay` already do
correctly — reusing the existing materialization pattern, not inventing a new mechanism or a new
persisted format. See the design document for the concrete change at each site.
