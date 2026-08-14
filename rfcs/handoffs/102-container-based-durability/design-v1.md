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

### 12.2 The DC-55 frozen fixture — ruled 2026-08-14

Stage 3's format-2 rejection makes `tests/fixtures/dc55_pre_swap_repo` permanently unopenable, and
`dc55_sha256_identity_end_to_end.rs` with it. **Deleting the test was the right immediate action and
accepting the coverage loss is not.**

That test's own doc states what it is: *"the one check that exercises every production call site against
genuinely persisted bytes"*, and specifically the only guard on a changed digest at **`layout.rs`,
`wal.rs` and `refs/log.rs`** — three sites the unit-level equivalence campaign cannot observe.

> **Corrected 2026-08-14 by the Stage 3 re-targeting: it guarded two, not three.** The fixture's
> `queue.wal` is **0 bytes**, and has been since the commit that added it (`01bbefb`) — verified. So
> `wal.rs`'s `record_checksum` was never exercised by this fixture at all, and the deleted test's own
> doc overclaimed. **I repeated that claim from the doc instead of checking the fixture.** It also
says, of the fixture: *"Do not regenerate this fixture … it is the evidence, not a convenience."*

**Ruled:**

1. **Keep the fixture.** Regenerating it as format-3 would destroy exactly the cross-version property it
   exists to prove. It stays frozen.
2. **Re-target the test at the bytes, do not retire the coverage.** The evidence is in the persisted
   bytes, not in `verify` returning `Ok` — so it does not need `RepositoryLayout::open`, which is the
   only thing format-3 takes away.
3. **Demonstrated, not assumed:** `sha256("heads/main")` is exactly the frozen ref filename
   `c316ccb36a95a977918874d43e722a5a7d9ef74b138f3b76078f6993c14a799f`. `layout.rs`'s ref-name storage-key
   digest is therefore recomputable from the frozen filename alone. Object ids are `.pobj` filenames;
   WAL and ref-log record checksums are inside the frozen files. **All three otherwise-unobservable
   sites are reachable without opening the repository.**
4. **Asserting the digest is stronger than the old test was**, which inferred correctness from a clean
   `verify` pass. This is an upgrade taken under duress, not a salvage.
5. **If any site turns out genuinely unrecoverable from the bytes, report it.** Do not absorb the loss
   silently — it is registered in `FINDINGS.md` until the re-targeted test lands.

**Scope note:** this is Stage 3 work because Stage 3 causes it. It is not a licence to touch other
DC-55 material.

### 12.3 What containers retire — ruled 2026-08-14

Stage 3's rewire orphans the loose-file publication mechanism. Three consequences were reported rather
than absorbed, which was right. **All three are ruled the same way: keep, record, decide separately.**

**1. `DurabilityContract::publish_immutable` (G5, "race-safe no-clobber publication") — keep it.**
Confirmed: after the rewire it has **zero production callers** — only the two trait impls and one test.
**But retiring a documented durability guarantee that has been through DC-71, DC-76, DC-81 and DC-82 is
an RFC-level act, not a stage's side effect.** Keeping it with a doc note, and removing only the
zero-caller pass-through wrapper, is the correct split. Registered in `FINDINGS.md` as orphaned.

**2. The duplicate-append property change — accepted, and narrower than it first reads.**

The race is **reachable, not theoretical**: `bundle.rs` writes objects without holding the active lock,
so two concurrent imports can each append a content-identical duplicate record.

**What is genuinely lost:** "exactly one physical copy, ever." That is storage efficiency.

**What is *not* lost, and matters more:** `publish_immutable_file`'s validator errored when a name
resolved to *different* bytes — a same-id-different-bytes detector. **That detection survives under
containers**, by a different mechanism already ruled in §12's item 4: reads validate by recomputing the
content hash, and a mismatch is a reported defect rather than a silent fallback. A shadowed divergent
record is caught at read.

**Ruled: the property change is accepted, and must be stated in the RFC rather than left as a silent
difference.** Two content-identical duplicates are not corruption; two same-id-different-bytes records
are, and are still caught.

**3. `object_temp_paths` / `PRIKK-DOCTOR-OBJECT-TEMP-DEBRIS` — keep, dormant.**
They detect debris from an interrupted loose-file publish, which a format-3 repository can no longer
produce. **Removing diagnostic surface inside the largest and riskiest stage is exactly the bundling
this project avoids** — Stage 3 is already changing the storage format, the read path, and the write
protocol. Retire them with G5, in one pass, or not at all.

**4. `object_store/tests/immutable.rs` and `races.rs` — do not delete.** They test the primitive, which
still exists, and the concurrent-write scenario, which is the very property change ruled in (2). They are
the evidence for what was traded away; deleting them removes the record of the trade.

**Consolidation:** these four are one decision wearing four hats — *does prikk retire loose-file
publication entirely?* It should be asked once, after Stages 4 and 5 have shown whether refs and trust
containerization removes the last uses. **Not now, and not piecemeal.**

---

## 13. Stage 4 Step 0 rulings — 2026-08-14

Three questions asked, four answered. The fourth is the one that matters.

### 13.1 What must be ordered — their finding, sharpened

`validate_log`'s single condition (`scan.rs:298-312`) carries **three** properties, not one:

1. **ref-name uniformity** — every record in this file names the same ref;
2. **the chain link** — `old_ref_state_id == previous`, purely relational;
3. **the positional check** — `update_seq == index + 1`.

**Their split is right and understated it by one.** Under a shared container, (1) becomes *vacuous* —
records of different refs interleave by design, and uniformity holds only within a filtered group,
where it is true by construction of the filter. (2) is unaffected. (3) is positional **only as a
shortcut**, holding today because one file happens to be exactly one ref's subsequence.

**Ruled:** `expected_seq` is computed from a record's position **within its own ref's filtered
subsequence**, never within the container. The container guarantees nothing beyond Stage 3's plain
append-only. **`RefLock` already serializes per-ref publication and does not change.**

### 13.2 One shared container — forced, not chosen

Correct, and the argument is the strong form: **acceptance criterion 1 makes a per-ref container
architecturally impossible.** Ref names do not exist at `init`; `branch create`/`tag create` mint them
later as ordinary recurring operations. There is no second option to weigh.

### 13.3 The candidate mechanism and `refs/tmp/`

**Accepted.** `write_ref_pointer_candidate` exists because a *mutable* per-ref file needs a scratch
name to update crash-safely. An append-only record has no candidate value to stage — the append **is**
the publish. `refs/tmp/` stops being written, unconditionally.

**`PRIKK-VERIFY-REF-CANDIDATE-DEBRIS`: kept, dormant, not pruned** — matching §12.3 item 3 exactly, for
the same reason. It joins the same deferred consolidation.

### 13.4 The fourth question — an index is required, and the key already exists

**This is the round's real finding.** `read_current_ref_state_id`/`replay_log` are called from **13
production sites** — `checkout`, `history`, `merge`, not administrative paths. Today they cost O(1) and
O(this ref's own log). Under a shared container with no further structure they become **O(total history
across every ref)**.

**That is linear scan, which the owner already rejected** — *"bad for maintenance and does not fit a big
project"* — when choosing indexed lookup for objects. **The same decision governs here; it is not a new
one.** An index is required.

**On their "new key shape" concern, which dissolves:** they observe `IndexEntry` is keyed by a fixed
32-byte `ObjectId` while ref names are variable-length UTF-8. **But this project already has a canonical
fixed-width ref key** — `layout.rs:541-543`'s `ref_name_storage_key` is `sha256(ref_name)`, which is
what names every ref file on disk today. **32 bytes, same width, already the project's own convention.**

**Ruled:**

- **A separate ref index container**, keyed by `sha256(ref_name)`. Same *pattern* as `index.rs` —
  append-only, last-entry-wins, rebuildable by scan, off the durability path — with its own type and
  its own container.
- **Do not widen `index.rs`'s schema.** The object index shipped in Stage 3; changing it means another
  format change and re-proving Stage 3's guarantees for no gain.
- **The log has the same question independently**, and the same answer: the ref index records where a
  ref's records live, or every `checkout` pays for the whole repository's ref history.

**They were right to report rather than choose.** The key-shape objection was real on its face and
resolved only by knowing the codebase already had the key — which is exactly the kind of thing that
should be ruled, not guessed at during implementation.

### 13.5 Corruption isolation, promoted to an acceptance criterion

Their §2 raises it unasked: a damaged record belonging to ref A must be attributed **only** to ref A,
matching today's granularity where one ref's file failing never touched another's outcome.

**Added to Stage 4's acceptance criteria**, proven the DC-95 way: damage one ref's record, confirm the
specific ref-scoped failure, confirm every unrelated ref stays clean.

### 13.6 The torn tail under a shared container — ruled 2026-08-14

A fifth question, found while implementing and asked before `publish_locked` was written. **Both
premises verified:** `refs/log.rs:92-96` refuses to append when `trailing_partial_bytes != 0`, and
`index.rs`'s four `trailing_partial` references are all *reporting* — Stage 3's object containers have
no equivalent pre-append refusal at all.

**1. No pre-append refusal on the shared log container. Accepted — and here is why it is safe, which
the proposal did not state.**

**A torn tail contributes no record.** Ref A's interrupted write leaves bytes that do not parse, so
they never enter A's filtered subsequence. A's records remain `[1, 2]`; the retry appends `update_seq
3` and lands at filtered position 2, so `update_seq == index + 1` still holds. **There is no sequence
gap, because the torn bytes were never a position.**

So today's refusal was never protecting sequence integrity. It enforced *hygiene* — truncate, then
retry — which was cheap when one file was one ref, and is neither available nor necessary once the
container is shared. **That is the same reason Stage 3's objects need no such check**, and the
symmetry is the argument, not the convenience.

**Without this, one ref's crash blocks every other ref's publishes** — an availability regression, and
exactly the blast-radius shape amended constraint 5 exists to catch.

**2. Ref-scoped partial-tail attribution via the header-carried `ref_name_key`. Accepted, with one
thing that must be checked rather than assumed.**

An unattributable torn tail — too few bytes to read a `ref_name_key`, or one naming a different ref —
means the classifying ref proceeds as if no partial tail exists. **Correct for writes**, by (1).

**But `classify_state`'s `PointerLeading`-with-partial-tail branch uses that tail as *evidence* that
this ref's own publication was interrupted.** Losing the attribution loses the evidence, which can
shift a classification away from "interrupted publication" toward something else. **That is a
diagnostic-accuracy consequence and must be traced against DC-38's state machine, not assumed benign.**

This is the third time this exact shape has appeared — DC-95 round 10's `require_retained_evidence`
reclassification and Stage 2 Level 1's `trust_is_valid` were the first two. **Each time, the mechanism
was sound and the *reason reported* was wrong.** Check what the classification says, not just whether
it blocks.

**3. Repair path. Agreed, and derive it.** Truncating a container's physical tail to clear one ref's
torn bytes is safe by the definition of "trailing" — but `truncate_incomplete_tail` today truncates a
whole per-ref *file*, and its shared-container equivalent must be derived from the code rather than
assumed identical.

**Asking before building was right.** This is DC-38's machinery, and a wrong call here is the kind that
needs a DC-41-grade proof merely to discover.

### 13.7 The DC-38 proof-suite migration — scope ruled 2026-08-14

**It is in Stage 4, and it cannot be split out.** Gates cannot be green while the tests do not compile,
and **shimming `publication_recovery` would leave DC-38's crash-recovery proof exercising a storage path
that no longer exists** — a suite that proves nothing while appearing to pass. That is the failure DC-95
Stage 1 spent twelve rounds making visible.

**But commit it as its own unit.** Core write/read protocol and proof-suite migration are separately
reviewable, and bundling them means a reviewer cannot tell which half a failure came from — the same
split applied to DC-95 Stage 2's levels and RFC 103's increments.

**The failpoint counts are evidence, not configuration.**

Stage 3's task 131 was fixture *construction*; this is re-deriving DC-41-grade evidence for the most
safety-critical machinery in the product. **A changed count is a finding to explain, not a number to
update.** In particular, the candidate/promote dance disappearing means fewer steps per publish, so
counts will drop — and a drop has two possible meanings:

- **genuinely fewer crash windows**, because the new protocol has fewer places to be interrupted; or
- **a window that existed and is no longer being tested.**

**Those are opposite conclusions and the derivation must say which, per count.** Adjusting a number
until the suite is green would silently convert the second into the first.

**On carrying this across a context boundary:** write the derivation down as it goes, not at the end.
DC-95's classified inventory was required to be *"assembled as Stage 1 goes, not reconstructed at the
end from seven review documents"* for exactly this reason, and it is the reason that inventory survived
twelve rounds. A count whose justification lives only in working memory is a count that will be
re-adjusted rather than re-derived.

### 13.8 The stale cross-reference defect, and a scope correction — 2026-08-14

**The defect is real. The reason given for it is partly wrong, and the difference matters.**

Their report says the comparison can never match because `refs/by-id`/`refs/logs` are *"directories
`init()` no longer creates at all under Stage 4's container model."* **They are still created** —
`layout.rs:380-381` still pushes both into `required_directories()`.

**The correct reason is the second one they give:** `RefFileOutcome::path` is now a display-only
container-offset locator, not a per-ref path, so comparing it against `layout.ref_pointer_path(...)`
cannot match regardless of what exists on disk. **Keying by `ref_name_key_bytes` is the right fix and
is unaffected** — but anyone acting on the stated reason would go and delete two directories that are
still allocated, which is a different change with its own consequences.

**Follow-on this exposes, not yet decided:** if nothing writes under `refs/by-id`, `refs/logs` or
`refs/tmp` after Stage 4, they are dead allocations in the same sense `quarantine/` is (§12.3's own
note). **They join the deferred consolidation** with G5, `object_temp_paths` and
`PRIKK-VERIFY-REF-CANDIDATE-DEBRIS` — not removed as a stage side effect.

**The tracing-only proof is a condition, not a completion.** The fix is verified by hand-tracing and
`cargo check`/`clippy` because the crate's test target does not compile until the migration lands.
**That is acceptable now and must not be forgotten**: `fully_framed_checksum_failure_is_never_truncated`
reporting the correct code is an explicit acceptance criterion for the migration commit, not something
to assume once things go green. **A green suite proves the suite compiles; it does not prove this
particular diagnostic was re-checked.**

**Scope: ~20 files, not 7. Accepted, and the pattern is now a property of this codebase.**

Their 7-file figure came from a narrower grep, corrected by surveying every
`ref_pointer_path`/`ref_log_path`/`ref_tmp_path` call site. **This is the same failure as my own
"22 `LegacyV1` sites" estimate in RFC 103**, which missed `PublicationState::LegacyLogLeading`, a whole
diagnostics module, and a dead subsystem — none of which carried the token.

**Standing consequence: a first-pass grep is a lower bound in this codebase, never a count.** Identifiers
and idioms routinely do not share a token with the concept they implement. Any scope figure that has not
been derived by surveying call sites should be stated as "at least N."

**On `ref_cluster.rs`'s nine candidate-then-rename idioms:** checking per-site rather than retargeting
blindly is right. If the paired rename is dead weight, deleting it is correct — **but confirm each test
still tests what its name claims afterwards.** DC-95 round 9 found a test whose intent had been silently
masked; a mechanical migration is exactly when that happens again.
