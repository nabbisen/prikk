# RFC 102 — Design v1

**Author.** Architect. **Independence.** Author-reviewed — the standing ceiling. See §11.
**Inputs.** §6.1–§6.7, all accepted. Four of them corrected this RFC's own text; those corrections are
folded in, not restated.
**Status.** Design for review. **No implementation authorized by this document.**

## 1. What is being built

Durability-bearing repository state moves from one-file-per-object into a **fixed set of container files,
every name created at `init`**, each an append-only sequence of checksum-framed records — the shape
`wal.rs` and `refs/log.rs` already run in production.

**Why this closes the Windows gap:** appending to a file that already has a name needs only content
durability, which Windows provides. **Only via `durable_append`/`durable_truncate` — never
`atomic_replace`**, which creates a temp name and renames even over an existing destination (the RFC's §3
correction). That distinction is the design's single most load-bearing constraint and the easiest to lose.

## 2. The container set

**One container per persisted object type**, plus refs, trust, and the existing WAL. Per-type rather than
one merged objects container because `persisted_object_types()` is already a fixed enumeration that
`verify_object_type` already iterates — the container set is then **derived from an existing invariant
rather than newly chosen**, and nothing has to keep the two lists in agreement.

Every name allocated at `init`, including each container's **A-slot and B-slot** (the RFC's §3.2 compaction
requirement) and the index containers. **No name is created after `init`.** That is the RFC's §6.3 acceptance
test and it is this design's own.

`quarantine/` is dead (§6.3a) and is not carried forward.

## 3. Record framing and the read path

Framing reuses the WAL's proven shape: magic, version, sequence, length, body, SHA-256 checksum over the
preimage.

**The read path is the piece with no precedent, and §6.3a requires it: name the failed record and
continue.** Today `decode_records` hard-`Err`s on a mid-stream checksum mismatch — correct for a queue,
a blast-radius regression for a container of unrelated objects (amended constraint 5).

**Design — resynchronise on the magic:**

1. Validate the frame at the cursor. Sound → emit, advance by its length.
2. Corrupt → **emit a finding naming the record's offset**, then scan forward byte-wise for the next
   magic.
3. At each candidate, validate the full frame including checksum. **A false positive — the magic
   appearing inside object bytes — is rejected by the checksum with overwhelming probability, and the scan
   continues.** The checksum is what makes resync safe; the magic only makes it cheap.

   **Stated as probability, not certainty, deliberately.** `record_checksum` covers
   `(sequence, body_len, body)` — **not** the magic or version bytes — so a false positive is rejected
   because forging a matching SHA-256 by accident is negligible, not because the format forbids it. This
   project's standard is invariants over probabilities, so **if a resync guarantee must be absolute, the
   framing has to change to cover the header** — and that would be a format change, which Stage 2 is not.
   Report it if you conclude the probabilistic bound is insufficient.
4. A trailing partial frame at EOF stays *tolerated*, exactly as today.

**Corruption is therefore confined to the records it actually damaged**, and `verify` names them — which
is what constraint 5 demands, stated as a mechanism rather than an intention.

## 4. The index

**Append-only, rebuildable, off the durability path** (§6.7). One framed record per entry: object id,
container, offset, length, checksum.

**Rebuild is not a new operation** — `verify_object_file` already derives an id from a name, recomputes
it from the decoded bytes, and requires a match. A rebuild is that, iterated over a scan.

**Publication is by append, not by flipping a field.** No primitive overwrites bytes at an offset (§6.7,
verified across all eleven `DurabilityContract` methods), so an in-place publish is unbuildable here.
Compaction publishes by appending a *generation* record to a small fixed-name log; readers take the last
complete generation record. **The RFC's §3.2 A/B ruling is about pre-created names and is unaffected** — only the
publish mechanism changes.

## 5. The write protocol — the part no framing can enforce

> **A reader must never observe an index entry as complete unless (a) its own framing is fully present,
> and (b) the object bytes its `(offset, length)` names are already durably present.**

**(b) has no cross-file atomic primitive to lean on**, so the protocol carries it:

1. Append the object record to its container. **Make it durable.**
2. Only then append the index entry.

**A crash between them leaves an object present and unindexed** — recovered by rebuild, and the safe
direction. The reverse ordering lets a reader see a valid, checksummed entry pointing at bytes that are
not there. **This ordering is load-bearing and must be stated at the call site, not only here.**

## 6. The worktree marker — **IMPLEMENTED, Stage 1, merged `6d10185`**

Not containerizable; the danger is the *inference from absence* (T12), not the lost file.

A fixed-name unclean-shutdown marker, **created at `init`, set by appending a sentinel, cleared by
`durable_truncate_to_empty` — never `atomic_replace`** (§6.5 found this is the first thing that would
have been built the unsound way). Set before any worktree write begins; cleared after materialization
completes. While dirty, commit-authoring refuses to infer deletion until the worktree is re-verified
against its baseline.

the RFC's §6.5 confirmed one choke point (`worktree_patch/node_authoring.rs`'s `plan_delete` loop — cited as `:441-446` when this was written, **now `:458` after Stage 1**; cite the symbol, not the line), and that the marker's own
failure modes fall toward *"still dirty"* — a spurious refusal, never a missed dirty state.

## 7. Staging — and the first two stages change no storage format at all

**Stage 1 — the marker, plus WAL-at-`init`. DONE, merged `6d10185` 2026-08-14.** Closes T12's signed-deletion risk and lands RFC 101 §5.1's
orphaned fix (§6.3a). **No container, no format change.**

**Stage 2 — isolate-and-continue reading, on today's WAL and ref log.** Earns §3's read behaviour against
formats already in production, before any container depends on it. **No format change.**

**Stage 3 — the object containers and the index**, with §5's protocol.

**Stage 4 — refs.** **Stage 5 — trust.** **Stage 6 — compaction.**

**Stages 1 and 2 deliver safety before the RFC delivers Windows parity**, and both stand alone if the
rest is never built. That is deliberate: the largest increment in the project should not be all-or-nothing.
Each stage merges before the next is scoped.

## 8. What does not change

B′ adoption semantics and object identity; object-trust/ref-authority separation; format-2's *rejection*
of the ahead-log state; DC-41's audited recoverability, which must be re-earned rather than assumed;
DC-95's classification, which every stage must leave intact.

## 9. Acceptance criteria

1. **No container name is created after `init`** — the test is enumeration, not inspection.
2. **No durability-bearing write uses `atomic_replace`.**
3. **Corruption isolation demonstrated**: a damaged record is named and the scan continues, with every
   other record still readable. Not asserted — shown.
4. **The §5 ordering proven**: a crash between container and index append leaves an object unindexed and
   recoverable, never an entry pointing at absent bytes.
5. **DC-41-grade recoverability re-earned** at the new design's own state count.
6. **Windows parity stated as a property**, not a platform list that happens to pass.
7. Green three-platform CI at every stage.

## 10. Open items the first implementation round must resolve

1. **Container-record ordering within a type** — whether records must be append-ordered by anything, or
   whether the index alone gives ordering. I have not derived this.
2. **What `verify` reports for an unindexed-but-present object.** Rebuildable, so not an error — but it
   is a state that does not exist today and needs a name.
3. **Lookup cost with the index cold**, and whether rebuild-on-open is acceptable for a CLI that runs
   once per command.

## 11. Independence

Author-reviewed. **Four of the seven prerequisites corrected this RFC's own text** — the RFC's §3
durability claim, its §8 non-goal, its §6.7 second publication shape, and §2's machinery framing in the
sibling RFC. That is the base this design is written on.

**Section references in this document:** bare `§N` means *this design*; anything belonging to RFC 102
itself is written "the RFC's §N". I got that wrong in RFC 103's handoff last week and the owner caught
it.

The compensation is that §9's criteria are falsifiable properties, and that §10 names three things I did
not derive rather than presenting the design as complete. **§2's per-type choice and §3's resync scheme
are the two places I would look first for an error** — both are mine, and neither has been checked by
anyone else.

---

## 12. Stage 3 Step 0 rulings — 2026-08-14

**§10's three open items, answered.** §10.1 and §10.2 by the investigation; §10.3 by me, because it is
a design question and they correctly declined to decide it.

**§10.1 — container-record ordering: none required.** Objects are immutable and content-addressed;
`read_object` resolves by computed path per type, never by scan or by recency, and nothing downstream
consults write order. **Containerizing changes physical location, not that property.** The index need
not encode or preserve ordering, because none exists to preserve.

**Their scoping note is the important half and I am adopting it:** this is about *object* containers
only. Ref logs carry real sequence semantics today, so Stage 4 inherits nothing from this ruling.

**§10.2 — the present-but-unindexed state is named `Unindexed`, and is non-blocking.** An
`ObjectItemStatus::Unindexed(ObjectVerification)` carrying the same successful per-object data
`Evaluated` does — the object is sound, only its index entry is missing — explicitly excluded from
`has_item_failure()`. `doctor` reports it at `DoctorSeverity::Info` (*"does not require user action"*),
so the health gate is unaffected. Exact enum shape is implementation judgment; the name and the
non-blocking status are the ruling.

**§10.3 — the index is trusted, and validated at the point of use.**

Every CLI invocation re-verifying the index against its containers would be O(n) per command and would
defeat the point of having an index. Blanket trust is also wrong. **The resolution is neither:**

1. **Ordinary reads trust the index** for location — one index read, one seek. No scan.
2. **The bytes found are validated by recomputing the content hash**, which is free of extra I/O because
   the object must be decoded anyway. Content-addressing makes a wrong location *detectable at use*.
3. **A mismatch is a reported defect, not a silent fallback to scanning.** The index is maintained by
   the write protocol (§5); a mismatch means something is wrong and must be said, not worked around.
4. **`verify` does the full scan.** That is what `verify` is for, and it is where rebuild belongs.

**This is deliberately not the advisory-index option the owner rejected.** That one treated a wrong
answer as routine and fell back to scanning, making O(n) the ordinary case and correctness a
best-effort. Here the index is authoritative, the hash check is a correctness assertion rather than a
performance hedge, and a violation is an error rather than a slower path.

**Open, and not settled by this:** whether a cold index — one that must be rebuilt because it is absent
or truncated — makes first-run cost unacceptable for a CLI. That is a *recovery* path, not steady state,
but it should be measured rather than assumed once containers exist. `dc59_commit_benchmark.rs` is the
existing precedent for taking n=10,000 seriously.

### 12.1 Step 0 item 1 — decided by the project owner, 2026-08-14

**Bump to format 3; reject format-2 at open. No dual-layout bridge.**

Owner's words: *"prikk is in early stage of development and is not in production use. We don't have to
care about such migration yet."* **Every existing format-2 repository becomes unopenable the moment
Stage 3 ships.** That consequence was stated before the decision, not discovered after it.

**Consequences that follow, and are not separately decidable:**

- **One storage mechanism, as the RFC's constraint 1 requires.** Reading both layouts was the only
  alternative and it reintroduces the dual-path shape RFC 103 spent an increment deleting.
- **The rejection reuses format-1's proven shape** (`layout.rs:365-377`): a named arm, a message stating
  the detected and required formats, and a migration route. **The catch-all
  `Err(UnsupportedFormatVersion(0))` is not acceptable for format 2** — it names nothing, and format 2
  is the format essentially every existing repository is in.
- **The message must name a real route.** Format-1's points at `0.19.0` bundle export. Format-2's must
  name whichever release last supported it — **established from the release record at implementation
  time, not guessed**, exactly as RFC 103 Increment A did rather than trusting my placeholder.
- **`init` already refuses a mismatched existing `FORMAT`**; that refusal inherits the bump and needs its
  own message audit, not just a constant change.

**Not decided here:** whether `RepositoryFormat` gains a `CurrentV3` variant or the sole existing variant
is renamed. That is implementation judgment — but note RFC 103 Increment B was abandoned precisely
because `require_current_format`'s disk re-read is live, so the enum's *shape* still carries a real
runtime check and is not free to collapse.
