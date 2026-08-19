# DC-53 Stage 2 follow-up — a multi-key import can leave author key material behind after refusing

**Found 2026-08-19** by the dev team during RFC 115 Checkpoint 1, reported and correctly left unfixed as
out of that scope. **Verified independently by the architect.** Live on `main`.

## 1. The defect

`import_bundle` records transported author key material in a loop:

```rust
let active_lock = ActiveLock::acquire(layout)?;
for entry in &author_keys {
    record_author_key_material(layout, &entry.key_id, entry.public_key, &active_lock)?;
    ...
}
```

**`record_author_key_material` refuses a key that conflicts with this repository's existing material.**
With `m > 1` transported keys, a conflict at entry *k* propagates via `?` — **after entries `1..k-1`
have already been durably appended** to a container with **no prune, no compaction, and no repair**.

**The consequence is not untidiness.** A receiver can be permanently pinned to an attacker-supplied
public key for an author, **from an import that failed** — after which the genuine author's patches are
refused forever, and nothing in the product can undo it.

## 2. Why the existing guards do not catch it

DC-53 Stage 2 built **two** rejection layers, and they sit on opposite sides of the writes:

- **Layer 1 — bundle-internal consistency** (`bundle.rs:362-374`): two different keys for one `key_id`
  *within the bundle*. **Checked before any write.** Correct.
- **Layer 2 — local conflict** (inside `record_author_key_material`, called at `:407`): a transported
  key disagreeing with material this repository already holds. **Checked during the write loop.**

**So the design already knows the discipline and applies it unevenly.** Layer 1 was written pre-write
precisely so a refused bundle leaves nothing behind; layer 2 does the same job one entry at a time,
while writing.

**The existing test does not reach it.** `import_rejects_a_transported_key_conflicting_with_local_material`
uses a single signer, so `m = 1` and there is no `1..k-1` to leave behind.

## 3. The fix

**Validate the whole transported key set against local material before recording any of it, inside the
lock already held.**

```
acquire ActiveLock
  for each entry: check against lookup_author_key_entries -> refuse the whole import on any conflict
  for each entry: record
release
```

**Both passes must be inside the same `ActiveLock`.** A validate-then-record split across the lock
boundary is a check-then-act race — the same defect Step 1's C1 fixed on the rollback-draft path, and it
would reintroduce it here.

**Reuse `lookup_author_key_entries`; do not write a second notion of conflict.** DC-53 Stage 2's own C2
ruling was that the idempotency rule exists once — the same reasoning applies to this check.

**Idempotent re-records stay idempotent:** an identical `(key_id, public_key)` pair is not a conflict and
must not become one.

## 4. What is deliberately not in scope

- **Objects written before the key loop** (`:392`) may remain after a refused import. **Leave them.**
  They are content-addressed, unreferenced, and harmless — a retry writes the same ids. **Do not
  "fix" this**: making object writes transactional is a much larger change with no defect behind it.
- **No change to layer 1**, which is correct as written.
- **No format change.** This is import-side logic only.

## 5. Tests

1. **The regression test that does not exist**: a bundle carrying `m > 1` transported keys where a
   later entry conflicts with local material. Assert the import is refused **and that
   `lookup_author_key_entries` returns empty for the earlier, non-conflicting `key_id`s** — the
   container, not the object store, is what this guard protects.
2. **A negative control**: revert the pre-validation pass, watch that test fail, restore. **Report the
   failing output.** A guard nobody has seen fire is not evidence.
3. **Idempotence preserved**: a bundle re-imported unchanged still succeeds and records nothing new.
4. Keep the existing single-key test — it covers a real path and its passing must not depend on the fix.

## 6. Reporting

Report before pushing, with the negative control's output and the full gate set **run against the fixed
commit** — not carried forward from an earlier run.

**One line worth writing into the module doc while you are there:** that layer 2's check is pre-write
*because* a refused import must leave the container untouched. The reason is what stops someone
"simplifying" it back into the loop later.
