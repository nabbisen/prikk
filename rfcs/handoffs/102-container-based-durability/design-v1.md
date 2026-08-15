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

### 13.9 Two fixture-migration dispositions — ruled 2026-08-14

**Question 1 — cross-crate test access: a `test-support` cargo feature, not a public method.**

The five `prikk-cli` integration tests need to drop one ref's pointer entry while others survive.
`remove_pointer_entries_for_test` does exactly that and is `pub(crate)` + `#[cfg(test)]`, invisible to a
dependent crate.

**Rejected — a genuinely `pub` method.** That ships production API whose only purpose is to damage a
repository, in a product whose entire claim is that history is not silently lost. Discoverable,
supported, and permanently in the surface. **The cost is not the code; it is what the code being public
says.**

**Rejected — moving the tests in-crate.** `branch_create_fails_closed_on_surviving_log_with_no_live_
pointer` asserts *CLI* behaviour: `run_create`'s refusal via `recoverable_missing_ref`. Moving it keeps
the setup and loses the thing under test.

**Ruled: a non-default `test-support` cargo feature on `prikk-store`**, exposing the existing helper,
enabled by `prikk-cli`'s dev-dependencies. Standard idiom, absent from the shipped build, and the gate
set's `--all-features` keeps it compiled and linted rather than rotting.

**This is the workspace's first cargo feature, and it carries a trap worth naming:** because every gate
runs `--all-features`, **a build *without* the feature is never exercised**, so feature-gated code could
silently become load-bearing for ordinary compilation and nobody would notice. **Condition: nothing
outside `#[cfg(test)]` or the feature-gated module may reference it.** That is checkable by review and
must be checked, not assumed.

**Question 2 — `candidate_issues`/`refs/tmp/`: keep, and register a finding the proposal did not name.**

Keeping is consistent with §12.3 and §13.3 — dormant diagnostics are not retired as a stage side
effect. Both tests are rewritten to plant debris directly, with corrected doc comments saying the state
is no longer reachable through any real crash.

**But the two cases are not symmetric, and this is the part that matters.** `object_temp_paths` is
*non-blocking*. `candidate_issues` pushes a non-blocking issue **into `publication_issues`**, and
`ensure_no_incomplete_publication` refuses on **any** non-empty `publication_issues`, blocking or not.

**So a stray file in a directory nothing writes to would refuse every mutation, permanently, with
nothing left that clears it** — the candidate-cleanup path went with the mechanism. A dormant
*diagnostic* is harmless; a dormant *wedge* is not.

**Ruled: keep the scan, and register the blocking asymmetry as a finding for the deferred
consolidation.** Do **not** change `ensure_no_incomplete_publication`'s semantics inside Stage 4 — that
is DC-38 machinery and a behaviour change of its own. The consolidation decides whether the check is
retired, made non-refusing, or given a clearing path.

### 13.10 Two rulings from the migration checkpoint — 2026-08-14

**1. The second `test-support` capability is allowed, on the same terms.**

`seal_recovery.rs::seal_rejects_format2_log_lead` needs to restore a pointer's raw bytes so a ref points
backwards — deferred rather than added unilaterally, correctly.

**Allowed, behind the same feature gate.** The reasoning that rejected a genuinely `pub` method was
about *what ships*, and the feature boundary already answers that; a second helper behind it does not
move the boundary. That the state it creates is "valid but wrong" rather than "missing" makes it more
dangerous to *misuse*, not more dangerous to *ship* — and misuse is what the gate prevents.

**Condition, from DC-95's own precedent:** name it so the hazard is legible at the call site. The
fake-signed helper cost four rounds because `publish_ref_to_new_block` read as ordinary. A helper that
rewinds a ref to a false-but-valid state should say so in its name.

**2. `state_matrix/fixture.rs`'s vacuous comparisons are the checkpoint's most important find.**

`state_bytes()` was reading `ref_pointer_path`/`ref_log_path` — locations nothing writes to after
Stage 4 — so three downstream tests' before/after snapshot comparisons were **`None == None`,
vacuously true**. They compiled, passed, and asserted nothing.

**This is DC-95 round 9's pattern, arriving through a different door.** There, containment exposed a
`doctor` test whose intent had been masked; here, a storage change turned three comparisons into
tautologies. **Both were silent, and in both cases the suite stayed green.**

**Standing consequence for the rest of this migration:** a fixture that reads a *path* is a fixture that
can go vacuous when the path stops being written. **Every remaining migrated comparison must be checked
for whether it can still fail**, not merely for whether it passes. A test that cannot fail is worse than
a deleted one, because it reports coverage.

### 13.11 Bucket R disposition — approved 2026-08-14

**All four retirements and both redesigns are approved. The categorization was checked, not accepted.**

**Verified independently:** `state_matrix/fixture.rs:56-60` builds every state except `LegacyLogLeading`
via `root_publication` — so the matrix's `PointerLeading` is the **root/sequence-1 case only**, and the
existing-ref/sequence-2 path through `classify_ref_state`'s arm 2 is genuinely uncovered. And its three
tests are verify/doctor read-only, production retry, and **one representative** command mutation per
state.

**Both of those facts are what make the categorization correct**, and they are the two that would have
made it wrong if they had gone the other way:

- The four retirements are redundant **because the matrix covers their end state at the root case**,
  which is the case they build.
- `existing_ref_pointer_lead_finishes_but_format2_ahead_log_refuses` survives **because the matrix does
  not build a second publication** — a genuinely different path, not a different name for the same one.
- `candidate_failure_warns_and_retry_publishes_once` survives **because the matrix checks one
  representative mutation**, while this test proves the wedge reaches `append_patch`,
  `add_trusted_maintainer` *and* `repair_repository`.

**On the near-miss, which is the more important half of the report.** The first read categorized that
test by name and opening lines and would have retired it, quietly narrowing coverage of the wedge while
believing the migration was neutral. It was caught by reading the whole body.

**That is the third time in this migration that a name matched and the body did not** — the
`seal_truncates_only_partial_tail` pattern-match, `state_bytes()`'s vacuous comparisons, and now this.
**Standing condition for the remaining buckets: categorize from the body, never from the name plus the
opening lines.**

**And an explicit answer to the process question: silence is not consent for deleting a test.** Every
retirement needs an affirmative ruling. The near-miss is the argument — a categorization that looked
obvious was wrong, and the only thing that caught it was someone being required to justify it out loud.

### 13.12 The two removed ref-path checks — ruled 2026-08-15

**Check 1 — `verify_repository_detects_noncanonical_ref_pointer_path`: redesign approved, and widen it.**

Their analogue is real and verified: `scan.rs:151-153`'s
`ref_name_key_bytes(&entry.ref_name) != entry.ref_name_key` is the same property the old check proved —
*identifier and content disagree* — with filename-vs-content replaced by header-vs-content. **It has no
test at all.**

**Widen, because there are two, not one.** `scan.rs:311` carries the same coherence check on the
**log-container** side — *"ref container record header ref_name_key does not match its own envelope"* —
and it is **equally untested**. They named only the pointer-index one. **Cover both.**

So this is not coverage preserved through a rewrite; it is **coverage that does not exist today**, on
two checks the container rewrite introduced. That makes the redesign strictly better than what it
replaces, and it is the reason to do it now rather than file it.

**Check 2 — `verify_repository_detects_every_ref_path_shape_violation`: not yet. Establish the claim
first.**

The retirement rests on *"every malformed-record shape converges on already-tested decode-failure
coverage"* — and they say plainly they checked only checksum corruption, not a too-short or too-long
framed record. **That is the claim, and it is unestablished by their own account.**

**Round 6's precedent governs**: `ensure_ref_path_shape` was ruled downstream-redundant *and kept*,
because redundancy was demonstrated rather than assumed. The same standard applies to its successor's
absence. **Before retiring, show what covers each malformed shape:**

- a record whose framed length is shorter than its header claims;
- one longer than its header claims;
- a truncated header — fewer bytes than the fixed header size.

For each: does it produce a tested failure, and is it **attributed to the right ref**? Attribution
matters here specifically — §13.6 established that an unattributable torn tail is not this ref's
problem, and a malformed record that fails to decode may land in the same unattributed bucket.

**If all three converge on tested coverage, retire it and say which test covers each. If any does not,
that is a gap the old test was inadvertently holding**, and it needs coverage rather than a deletion.

### 13.13 Check 2's convergence, and the pointer-index gap — ruled 2026-08-15

**The hold was worth it: the convergence claim is true on one side and false on the other.**

**Log side — genuinely redundant, retire it.** All three shapes traced to tested, *attributed* coverage:
truncated header and short-body both reach `FrameAttempt::TrailingPartial` with best-effort attribution
(`own_torn_tail_is_attributed_and_repairable`, `foreign_torn_tail_does_not_block_or_misattribute_an_
unrelated_ref`); a complete frame with a bad checksum reaches `FrameAttempt::Invalid` carrying
`claimed_ref_name_key`, covered with attribution by
`isolates_a_damaged_record_and_reads_every_sound_record_around_it_across_refs`. **Attribution shown, not
assumed** — which is the part §13.12 said not to take on trust.

**Pointer side — not redundant, because there is nothing to be redundant with.** Verified:
`pointer_index/tests.rs` has **2 tests**, both happy-path round-trip, against `container/tests.rs`'s
**7**. Nothing constructs a truncated entry, a checksum mismatch, or checks attribution.

**Write the coverage now. This is option 1, and it is not discretionary.**

**Amended constraint 5 already decides it**: *"corruption isolation must not regress… demonstrated, not
asserted."* The pointer index **replaces per-ref pointer files**. The old path-shape test's `by-id/`
sub-case was — unintentionally — the only thing in the suite touching pointer-index decode-failure
behaviour. **Retiring it and filing the gap would ship the replacement with less corruption coverage
than the thing it replaced**, which is precisely what constraint 5 forbids.

**Why filing it is the wrong instrument here, specifically.** The deferred list — G5,
`object_temp_paths`, the `refs/tmp` wedge, the dead `refs/` directories — is entirely *dormant or
retired mechanisms*, where deferral costs nothing operationally. **This is live new code with no
corruption coverage.** Different category, and the list's length is an argument against adding a
different kind of item to it, not for.

**Scope: mirror `container/tests.rs`'s shape** — truncated entry, checksum mismatch with attribution,
isolate-and-continue across multiple refs. A known shape, not novel design. **If it turns out
`decode_pointer_index_records` does not have the same three-shape structure, that is a finding**, and a
more important one than the tests.

### 13.14 `ref_cluster.rs` closed, and the fail-closed asymmetry — ruled 2026-08-15

**Accepted, with one thing reframed.**

The third test is the round's real contribution: `read_pointers` **fails closed on the whole read** for
any damaged pointer-index entry, rather than isolating it as the log container does. Their reasoning for
why that is correct is right — the pointer index is *last-entry-wins*, so skipping a damaged **latest**
entry could let an older entry for the same ref resolve as current. **Silent staleness is worse than
unavailability**, and nothing proved this end to end before.

**But it is not an inherent property, and calling it one would bank a regression as a design choice.**

Verified: `container.rs:203-205`'s `FrameAttempt::Invalid` carries `claimed_ref_name_key:
Option<[u8; 32]>`; `pointer_index.rs:159-161`'s carries **only `message`**. **Fail-closed is forced by
that missing field**, not by last-entry-wins. With the key present, a damaged entry could fail *its own
ref* while every other ref resolves — exactly the container side's behaviour.

**So this is a blast-radius regression against amended constraint 5**: one corrupt per-ref pointer file
used to affect one ref; one corrupt index entry now blocks every ref. **Accepted for Stage 4** — the
behaviour is the safe direction and the alternative risks silent staleness — **but recorded as a known
regression with a known fix, not as an accepted permanent property.** Registered in `FINDINGS.md`.

**Not to be fixed inside Stage 4.** Adding the field changes `pointer_index`'s frame handling while the
proof-suite migration is still open, and Stage 4 is already the largest stage in this RFC.

**On the assertion they got wrong first:** assuming a hard top-level `Err` and finding a `Refs`-stage
`Failed` with downstream `NotEvaluated { blocked_by: Refs }` is the stage-containment shape DC-95
Stage 2 Level 1 built. **Fixing the test to match reality rather than the production code to match the
guess** is the correct direction, and leaving the wrong assumption visible in the doc comment is better
than erasing it.

### 13.15 Two dropped production checks — ruled 2026-08-15

**Both confirmed, and one is worse than "a check went missing."**

- `created_at` appears **zero** times in `refs/container.rs`. The check dates to `8f565f2` — **DC-39's
  implementation of a DC-34 ruling.** Dropping it regresses a corrective-program guarantee, not a test.
- `refs/container.rs` has **zero** `validate_read_schema`/`validate_strict` calls; `object_store.rs` has
  one. **The refs/objects asymmetry is itself the bug** — objects kept the check, refs lost it.

**Placement 1 — `created_at`: in `append_ref_container_record`, not `publish_locked`.**

The rule is the choke point no caller can bypass. The old check sat in `append_log_record`, the single
write path; its successor is the append function. **`publish_locked` is the wrong layer** — any other
caller of the append escapes the check, which is exactly how it was lost.

**Their "makes a generic container function ref-log-aware" worry does not apply.** `refs/container.rs`
is already ref-specific — 65 references to `ref_name_key`/`RefContainerRecord`. There is no generic
container being contaminated; the module is the ref-log container.

**Placement 2 — `validate_strict`: after frame acceptance, NOT inside `parse_frame_at`.**

This is the part worth getting right. `parse_frame_at` is on the **resync path** (`container.rs:284`,
inside the decode loop that `resync_to_next_magic` drives). **Frame validity and envelope validity are
different questions**, and conflating them makes resynchronisation behave on semantic grounds:

> A frame whose checksum is correct but whose envelope shape is malformed is **a real frame containing
> a bad record** — not a false magic match. If `parse_frame_at` rejected it, resync would scan **past a
> genuine frame boundary** looking for the next magic, and every record after it could be misattributed
> or lost.

**So validate in the decode loop's `FrameAttempt::Record` arm** (`container.rs:285-294`), where a frame
has already been accepted as structurally sound, and record the failure as a per-record outcome — the
same shape `require_signed_type` failures already take. **Checksum decides whether it is a frame;
envelope validation decides whether the record is admissible.**

**And the systemic point, which matters more than either fix.** DC-95's classified inventory covers
`verify_repository`'s checks. **These two were write-path and decode-path checks — outside its scope
entirely**, which is why a rewrite could drop them and every gate stay green. Both were found by tests
that happened to survive, not by any systematic guard. **Registered in `FINDINGS.md`: there is no
inventory for validation outside `verify`.**

---

## 14. Stage 5 — scope, derived 2026-08-15

Stage 4 merged at `94219cf` with green three-platform CI, so Stage 5 may be scoped. §7 recorded it as
three words — **"Stage 5 — trust."** Deriving what that actually covers found that the phrase is wrong,
and two defects behind it.

### 14.1 Stage 5 as named reaches two of five sites — the RFC cannot meet its own criterion 2

§9 criterion 2 is a whole-RFC claim: **"No durability-bearing write uses `atomic_replace`."** Seven
production `write_file_atomically` calls remain after Stage 4 removed `refs/pointer.rs:51`. Classified by
reading each one, not by name:

| Site | Writes | Durability-bearing? |
|---|---|---|
| `trust.rs:106` | maintainer trust key, **one new name per key id** | **Yes** — see §14.1.1 |
| `trust.rs:132` | trust policy | **Yes** — see §14.1.1 |
| `active.rs:122` | active-WAL ref metadata (current branch) | **Yes** |
| `received.rs:107` | received ref pointer, **one new name per received ref** | **Yes** |
| `layout.rs:138` | `FORMAT`, at `init` only | **Yes**, but init-only — see §14.2 |
| `commit_index.rs:80` | commit index | **No** — its own header says *"a rebuildable, non-authoritative cache"*, and `load` is `unwrap_or_default` |
| `lifecycle_cache/incremental.rs:175` | lifecycle cache | **No** — `load` returns `Option`, and `:160` discards the save result entirely (`let _ = save(...)`) |

**Stage 5 as named covers two. Stage 6 (compaction) covers none.** `active.rs:122` and `received.rs:107`
belong to no stage at all, so the staging as written terminates with criterion 2 unmet.

#### 14.1.1 Trust fails closed, and I had the direction backwards

My first draft of the table above said a lost maintainer key *"silently reduces a signing threshold."*
**That is wrong twice, and the correction changes what Stage 5 must prove.**

1. **There is no threshold.** `trust.rs:2-4`: `required = 1` keeps its DC-11 meaning regardless of how
   many keys are adopted — a block needs *one* trusted signature, never a threshold of several
   (`rfcs/done/DC-78-HISTORY-EXCHANGE.md` §D2).
2. **Loss is neither silent nor weakening.** `load_maintainer_trust_policy` (`:208-237`) reads the policy
   and **every** key it names through `read_file_required`. A single missing key file is a hard
   `PrikkError::Integrity` that fails the entire load — so one lost key does not drop one signer, it
   **fails verification of every publication in the repository.**

So the risk is not privilege escalation; **it is the opposite, and more total than a threshold model
would be.** Losing trust state renders all previously-valid history unverifiable. That is a durability
and availability defect, and it is a *stronger* reason to containerize trust than the one I first wrote —
the per-key-id new-name surface at `trust.rs:106` is precisely where Windows would lose it.

**Consequence for Stage 5's criteria:** do not write a test that trust loss "fails safely" and call it
done — it already does, loudly. **Prove the state survives.** The failure this stage prevents is a
repository that can no longer verify its own history, not one that accepts a forged signature.

**Ruling: Stage 5 is "the remaining durability-bearing replacements", not "trust".** Trust is its largest
and most security-sensitive part, which is why it was the name that stuck — but naming a stage after its
most interesting member is how the other two went unscoped. The two rebuildable caches stay on
`atomic_replace` deliberately, and criterion 2's wording is what permits that: *durability-bearing*, not
*all*. **That exemption must be asserted by a test, not left as prose** — "this is only a cache" is a claim
about behaviour, and an unguarded claim is the failure mode this RFC has hit repeatedly.

### 14.2 `FORMAT` becomes durable before the containers it certifies exist

`layout.rs:138` writes `FORMAT` and fsyncs it; container allocation begins at `:146`. So a crash between
them leaves **a repository that reads as valid format-4 with containers absent** — and this is not
Windows-specific, it is the ordering on every platform.

Probed on a real repository (init, then delete all 16 container files):

- `prikk status` — reports the repository normally, **exit 0**
- `prikk verify` — **all 12 stages `evaluated`, exit 0**
- `prikk doctor` — same, **exit 0**

Nothing observes that a container name is missing, because **an absent container is indistinguishable
from an empty one** and the probe repository was empty. Harmless there; the point is that no check exists.

**`FORMAT` must be written last**, after every container name, so its presence certifies that init
completed. That is the only reason the init exemption is sound at all — §7 assumes new names at `init`
are acceptable but never argues it, and the argument is *"an interrupted init leaves nothing to lose,
provided the incomplete state is detectable."* Written in the current order, it is not detectable.

### 14.3 Every append creates the file first — so criterion 1 is enforced by nothing

`durable_append` (`anchored/linux.rs:51`) calls `open_append_regular`, which is:

```rust
match open_new_regular(directory.as_fd(), name) {
    Ok(fd) => Ok(fd),
    Err(rustix::io::Errno::EXIST) => open_existing_regular(directory, name, WRONLY | APPEND),
    Err(error) => Err(io_error(error)),
}
```

**It attempts creation first and falls back on `EEXIST`.** Every container append in the system routes
through it — objects (`index.rs:393`, `:412`), refs (`refs/container.rs:389`, `:392`), the WAL
(`wal.rs:174`, `:195`), the marker (`worktree_marker.rs:44`).

Three consequences:

1. **§9 criterion 1 — "no container name is created after `init`" — has no runtime guard.** The append
   path creates names. Criterion 1's test is *enumeration at `init`*, which proves init creates every
   name and is structurally incapable of proving nothing else does.
2. **It silently repairs the §14.2 state**, converting a detectable broken repository into an
   undetectable one.
3. **It is the RFC's own hole, in the RFC's own primitive.** A create-first append is a new-name event
   per append. On Linux that is a wasted `EEXIST` syscall; on Windows there is no way to make a new
   directory entry durable, which is the entire reason this RFC exists.

**Not a live defect** — Windows mutation does not exist yet (DC-37), so no shipped code depends on it.
**It is a live trap**: the Windows `DurabilityContract` implementation that Stages 5–6 are meant to make
possible would inherit this pattern by default and void the guarantee at the moment of arrival.

**Ruling: `durable_append` must require an existing file**, matching `durable_truncate` at `:61-64`, which
already uses `open_existing_regular`. The asymmetry between the two is unexplained and looks accidental.
Whether the strict version needs a separate create-at-`init` primitive is Stage 5 Step 0's to answer.

### 14.4 What Stage 5 Step 0 must report before any production code

1. **Trust's shape.** Maintainer keys are one file per key id — a per-name surface, exactly what Stage 4
   faced with refs. Does the ref-container answer transfer, or does trust's read pattern (verify a
   threshold across N keys) need something else? Derive it; do not assume Stage 4's shape fits.
2. **Whether `active.rs` and `received.rs` belong in Stage 5 or a Stage 7.** §14.1 puts them in scope;
   if their shapes are unrelated to trust's, splitting may be cleaner. A reasoned split is acceptable —
   silence is not.
3. **The `FORMAT`-last reordering**, and what re-`init` on a partially initialized repository must do.
4. **The `durable_append` strictness change**, and every caller that depends on create-on-append today.
   The one caller that looks like a counter-example is not: `wal.rs:174` appends an **empty slice** on the
   idempotent re-append path (last record's envelope equals the one being appended), returning the
   existing `seq`. It is a **durability barrier, not a creation** — zero bytes written, and the file and
   its parent re-synced so a duplicate request re-establishes durability. A strict `durable_append` does
   not break it, since the file necessarily exists by then, **but the strict version must keep zero-byte
   appends meaningful** — an implementation that skips the sync on an empty write would silently remove
   that barrier. Enumerate the real callers rather than reasoning from this one.
5. **How the two cache exemptions get asserted** rather than described.

### 14.5 Stage 5 Step 0 rulings, 2026-08-15

Step 0 returned three items for sign-off. Two are ruled against, and both were the ones the report was
most confident about.

**`policy.toml` does not collapse into container append order — rejected.** The proposal rested on
*"`add_trusted_maintainer` never removes a key, so the adopted set is every key id ever appended, in
append order."* True of the CLI — there is no `revoke` subcommand — and **false of the system.**
Adoption is derived *exclusively* from `policy.toml`'s `keys = [...]` list: `load_maintainer_trust_policy`
(`trust.rs:219`) parses the file and loads only the ids it names, and **the keys directory is never
enumerated** (`:83` builds a path and nothing more). The `.pub` files are key material; `policy.toml` is
the authority. **Removing a key id from it revokes that key while its `.pub` stays on disk — the only
working revocation mechanism prikk has**, and `docs/src/reference/repository-layout.md:195,216` documents
the file as exactly that authority. An append-only container with no removal record cannot express "no
longer adopted" at all.

**Ruling:** containerize the policy artifact — move the write off `atomic_replace`, which is all criterion
2 requires — and keep an explicit *current adopted set* record. Append-order semantics may be revisited
only with a removal/tombstone record variant designed in from the start, under its own ruling. The
owner's acceptance of pre-production format churn does not apply here: **losing a security capability is
a different class of decision from deferring a format bump.**

**`active.rs` may not be converted on the marker pattern as proposed — blocking.** The plan was
pre-allocate empty at `init`, then append to set and truncate-to-empty to clear, *"the exact mechanism
`worktree_marker.rs` already implements."* Trace an empty file through `read_active_ref_metadata`
(`active.rs:98-115`): absent yields `ActiveRefMetadata::Missing`, but **empty yields `Some(b"")` →
`validate_local_branch_ref("")` → `Err` (`refs.rs:399-404`) → `ActiveRefMetadata::Invalid`.** So the
cleared state stops being `Missing` and becomes `Invalid`, and **every freshly-`init`ed repository reports
invalid active-ref metadata from creation.** That enum is matched at ten production sites, including
`verify.rs:1265`'s `(wal_is_empty, metadata)` verify stage and `require_active_ref_for_non_empty_wal`,
where `Invalid` is an integrity signal.

The marker analogy misleads because the marker has two states and treats empty as clean; `active.rs` has
three and empty maps to the wrong one. **The shape matches, the state machine does not.**

**Ruling:** `read_active_ref_metadata` must treat empty as `Missing`, and all eleven call sites re-checked
for anything distinguishing "file absent" from "no active session," before this unit is written.

**Approved:** no separate lookup index for trust (every trust read is a whole-set load — though the report
counted four such sites and there are five; `verify/trust.rs:40` was missed, without affecting the
conclusion); `received.rs` on the refs container+pointer-index pattern; the three-way split into three
committed units; `FORMAT`-last with re-`init` completing an interrupted init; `durable_append` strictness
— safe because `create_empty_file_once` (`layout.rs:518-524`) creates names through
**`create_new_file_required`**, never through `durable_append`, a fact the report's conclusion needed and
did not supply.

### 14.6 `active.rs` plan sign-off, 2026-08-15

The corrected plan is **approved**, subject to one condition, and its own count corrected.

**`write_active_ref_metadata` must truncate-to-empty then append, internally.** It is `pub`
(`active.rs:118`) and re-exported (`lib.rs:81`), so it is public API, and today it is
`write_file_atomically` — **replace semantics, idempotent, safe to call twice.** A bare append makes a
second call concatenate two ref names into one file, which reads back as `Invalid`: a public function
that is idempotent today would silently corrupt state tomorrow, with the requirement living only in the
discipline of its single caller. Truncating inside the function keeps the public contract unchanged and
makes the single-entry invariant structural rather than conventional. It adds no crash exposure —
`prepare_empty_active_ref_for_append` already opens an empty-file window between clear and write; this
only moves it inside, where a crash leaves `(true, Missing)` on an empty WAL, which is not debris.

**§14.5 said the enum is matched at "eleven production sites." It is ten** — corrected above. The
developer's own sweep covered nine, missing `active.rs:176`
(`require_active_ref_for_non_empty_wal`), which distinguishes `Missing` from `Invalid` with two
different `Integrity` messages and is therefore in the class the sweep existed to find. It is unaffected,
for the same reason `seal.rs:106` is, so **the conclusion stands** — but the ruling that told the
developer to re-check every site carried a wrong count, and their re-check was itself incomplete. Both
are instances of the standing `FINDINGS.md` row on searches reported as exhaustive.

**Scheduling, which neither the plan nor §14.5 caught:** step 1 pre-allocates
`default_active_ref_name_path()` (`active/default/ref-name`, `layout.rs:281-282`) at `init`, and `init`'s
`create_empty_file_once` list (`:139-169`) does not include it today. **That is a new name at `init`, so
this unit blocks on the format-bump decision exactly as `received.rs` and the trust key container do** —
three units, not two. The developer's own format-bump argument did not generalise to their own unit.

### 14.7 The format bump — decided by the project owner, 2026-08-15

**Bump to format 5; reject format-4 at open. No dual-layout bridge.**

**Every existing format-4 repository becomes unopenable the moment Stage 5 ships.** That consequence was
stated before the decision, not discovered after it — the same discipline §12.1 recorded for the 2→3
bump.

**What it cost, established rather than assumed.** In-tree: nothing. The CI fixture is authored fresh
each run (`ci.yml:110-112` invokes `prikk init`), the only on-disk fixture repository
(`crates/prikk-cli/tests/fixtures/dc55_pre_swap_repo`) is format 2 and has been unopenable since Stage 3,
and no other `FORMAT` file exists in the tree. The exposure was limited to format-4 repositories held
outside the repository, which is owner knowledge and was put to them as such.

**Why no middle path was offered.** Creating missing names on open reintroduces name-creation after
`init` — the property §14.3 just closed and criterion 1 tests for. Detecting missing names without
bumping makes "format 4" denote two different on-disk shapes distinguished by a side check, which is what
a format version exists to prevent. The real choice was **bump, or stop Stage 5 with criterion 2
permanently unmet**, and it was framed that way.

**Implementation.**

1. `CURRENT_FORMAT_VERSION` becomes `5`; `RepositoryFormat::CurrentV4` becomes `CurrentV5`; a
   `LEGACY_FORMAT_4_VERSION` arm is added to `read_repository_format`.
2. **The rejection message follows format 3's precedent, not format 2's — this is the part most likely to
   go wrong.** The Stage 5 question proposed *"name the last release that supported format 4, sourced
   from the release record."* **No such release exists.** The latest tag is `0.19.0`, which was format 2
   (`layout.rs:549-553`); formats 3 and 4 both landed after it and neither was ever tagged. Format 3's
   own arm (`:560-572`) therefore names no version deliberately, and says so in a comment. **Format 4's
   arm must do the same.** Naming a release here would be guessing, which the discipline governing these
   messages forbids — and the release record is what settles it, not memory.
3. **The bump lands once**, folded into whichever `init`-name unit ships first.

**Three units depend on this, not two:** `received.rs`'s I/O and index wiring, the trust key container,
and `active.rs` (§14.6 — its step 1 pre-allocates `active/default/ref-name`, a name `init` does not
create today).

### 14.8 `durable_truncate_to_empty` is the other half of §14.3, 2026-08-15

**Found by the developer in Stage 5 round 2, by reading `linux.rs`'s two truncate primitives side by side
rather than assuming they were symmetric.** They were not:

- **`durable_truncate` (`:61-71`)** — `open_existing_directory_required` + `open_existing_regular`. Strict.
- **`durable_truncate_to_empty` (`:74-88`)** — `prepare_directory_required` +
  `open_existing_or_create_regular`. **Creates the directory and the file if either is absent.**

`d8f5240` hardened `durable_append` per §14.3 and left its neighbour untouched. **So criterion 1 — "no
container name is created after `init`" — is still not met**, through the other durability primitive, and
nothing reports it.

**Three production surfaces, not the two first reported:** `worktree_marker.rs:51` (Stage 1, merged),
`active.rs:138`/`:160` (Stage 5), and **`wal.rs:283-284`**, which is the worst of them — it calls
`ensure_directory_required` explicitly *and then* the creating truncate, so draining a WAL whose
`queue.wal` or `active/default/` had gone missing silently reconstructs both. `queue.wal` is a name `init`
allocates (`layout.rs:140`).

**Ruling: fixed inside RFC 102, as its own Stage 5 unit.** Not optional hardening — the same defect
§14.3 closed, in the other primitive, and leaving it lets Stage 5 report criterion 1 as satisfied when it
is not. It becomes a separate unit rather than a side effect of `active.rs` because it touches merged
Stage 1 code.

**Required:** swap both primitives to the strict pair; enumerate every caller and establish that none
depends on creation, as `d8f5240` did; remove `wal.rs:283`'s `ensure_directory_required` or justify it
(`default_active_dir()` is already in `required_repository_directories()`, `layout.rs:389`, so it looks
vestigial — establish that, do not assume it); and when the three matrix tests that reach
`DirectoryCreate`/`CreatedDirectoryParentSync` through this path stop doing so, retire them documented in
place and **state what directory-creation failpoint coverage remains.** If the answer is none, that is a
finding to report.
