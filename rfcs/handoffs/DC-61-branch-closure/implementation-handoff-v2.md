# DC-61 Branch Closure - Handoff v2

**Cleared to start.** Accepted by the project owner on 2026-07-30, at
`rfcs/accepted/DC-61-BRANCH-CLOSURE.md`. All three verification obligations were discharged at design review
before acceptance; their results are §3 of the RFC and are scope, not open questions.
**Authored by** the architect.

**This supersedes handoff v1. Do not work from v1** — it stated the wrong call-site count and set a
falsification test you could not have satisfied. See §"What changed from v1" before anything else.

**Size:** medium. One payload field, schema-aware decoding through **18** call sites, one CLI subcommand, one
list filter.
**Touches:** `prikk-object` (`RefStatePayload`, encode + decode), **18 non-test decode call sites — including
three inside `verify`**, `prikk-cli` (`branch close`, `branch list --all`).

## What changed from v1

**Two corrections, both mine, both found before you started rather than by you mid-implementation.**

**1. The call-site count was wrong: 10, actually 18.** v1 listed ten and I wrote "10" into the RFC as well. I
enumerated files from memory instead of counting them, and missed six call sites across four files. DC-63
then added two more after v1 was written. The corrected list is in §Step 2. **Your scope is ~80% larger than
v1 said** on this axis — still mechanical, but budget for it.

**2. The falsification test contradicted the work it asked for.** v1 said "**Not** `verify`" and told you to
stop and report if you needed to modify it. But `verify` contains three of the decode call sites
(`verify/ref_publication.rs:68`, `:109`, `refs/verify/scan.rs:248`), so threading a schema parameter through
`decode_canonical` *mechanically requires* touching `verify`. Following v1 literally, you would have hit that
within an hour and stopped to report a contradiction I had authored.

The distinction v1 failed to draw, and which now governs:

| | `verify`, `publish`, `recoverable_missing_ref`, `doctor` |
|---|---|
| **Logic, control flow, classification, outcomes** | **Must not change.** This is the design's falsification test and it stands. |
| **Mechanical signature propagation** — passing `schema_version` into a decode call that already has the envelope in hand | **Expected and in scope.** Not a refutation of anything. |

Obligation 2's finding is unaffected: it concluded those four need no *behavioural* change, and that remains
true and verified. RFC criterion 4 has been corrected to match.

## What this is, and what it is not

**It is closure. Nothing is deleted.** The ref pointer stays, its history stays, its objects stay. Disk usage
does not drop.

The command is **`prikk branch close`**, not `delete`. That naming is deliberate and not negotiable: a command
called `delete` that deletes nothing is a lie to the user.

**Why not deletion.** DC-60 tried it. Removing the pointer while retaining the log produces
"pointer absent, log present" — and the system does not merely classify that as corruption, it has a
**repair function** for it: `refs.rs:210` `recoverable_missing_ref` detects it, `refs.rs:271`
`reconstruct_missing_ref_from_log` rebuilds the pointer, and `doctor.rs:174` offers that repair to users. So
`doctor` would have offered to resurrect every deleted branch. Deletion also bricked repository-wide commits
at every record count. Read `.git-exclude/reviewed/prikk-dc60-delete-divergence-ruling-v1.md` if you want
the full trace.

## The falsification test — read this before writing code

DC-61's argument is that closure is cheap **because** `verify`, `publish`, `recoverable_missing_ref`, and
`doctor` need no **behavioural** change. Obligation 2 confirmed that:

- `verify`'s `classify_ref_state` takes its ordinary `(Some, Some)` arms — pointer and log both present.
- `publish` treats closure as an ordinary CAS update, arm 1. There is **no same-target restriction**, so
  reusing the target object id is fine.
- `recoverable_missing_ref` returns `None` at `refs.rs:211-213` because the pointer is present.
- `doctor` only ever sees `RefRecoveryRepair` from the missing-pointer path, which does not arise.

**If you find yourself changing what any of those four *decides* — a new branch, a new arm, a different
classification, a different outcome for any input — stop and report it.** That refutes the design rather than
complicating it, and the choice between closure and tombstones has to be reopened. That is a stop-work
finding, not something to work around.

**Passing a schema argument through their existing decode calls is not that.** Nor is any pure-mechanical
edit forced by the signature change. The test is about decisions, not about whether a file appears in the
diff.

If you are unsure which side of that line an edit falls on, report it rather than deciding alone — that
judgment is the architect's, and getting a spurious report costs far less than a missed refutation.

## Step 1 — the payload field, and the identity trap

Add a closed marker to `RefStatePayload` as **field tag 7**.

### The trap that would break every existing ObjectId

`RefStatePayload::encode_canonical` omits absent optional fields entirely — look at the existing pattern:

```rust
if let Some(previous) = self.previous_ref_state_id {
    writer.field_object_id(4, &previous)?;
}
```

**Field 7 must be emitted only when the ref is closed.** If it is written unconditionally — say as
`closed: bool` always encoded, even `false` — then every ordinary ref state's payload changes, and **every
existing RefState ObjectId moves.** That is an identity break of exactly the class DC-55 spent an entire
increment proving it had avoided.

So: ordinary ref states emit tags 1,2,3,[4],5,6 and remain **byte-identical to today**. Closed ref states
emit 1,2,3,[4],5,6,7.

### Canonical encoding must have exactly one representation

Because the field is omitted when absent, an explicitly-encoded "not closed" would be a *second* encoding of
the same logical state. Canonical encoding does not permit that.

**Decode must reject an encoded field 7 that means "not closed."** Absent means open; present must mean
closed. Do not accept both spellings.

### Schema gating

Closed ref states are **schema 2**; everything else stays **schema 1**.

`schema_version` is a per-envelope `u32` (`prikk-object/src/envelope.rs:26`) and part of the ObjectId
preimage (`envelope.rs:143`). Ref-state envelopes are built by the caller and handed to `publish` as
`RefPublication.ref_state` (`refs.rs:354`), so `branch close` chooses schema 2 for its own object without
affecting any other publication path.

## Step 2 — schema-aware decoding, across 18 call sites

`RefStatePayload::decode_canonical(bytes: &[u8])` (`payload/refs.rs:56`) is **schema-blind** and rejects
unknown field tags unconditionally:

```rust
other => return Err(PrikkError::MalformedData(format!("unknown RefState field tag: {other}")))
```

So field 7 cannot simply be added — the decoder must know the schema. Thread it through. **This is in scope
and was identified at design review; it is not a surprise to negotiate.**

The **18** non-test call sites, verified by grep against `3ee3163`, not from memory:

```
crates/prikk-cli/src/seal/support.rs:102, :166
crates/prikk-cli/src/branch.rs:162
crates/prikk-cli/src/tag.rs:61, :201                        <- added by DC-63, after v1 was written
crates/prikk-store/src/rollback_draft.rs:187
crates/prikk-store/src/refs/publication.rs:131              <- was :128 in v1; DC-63 shifted it
crates/prikk-store/src/merge_evidence.rs:109
crates/prikk-store/src/history.rs:126
crates/prikk-store/src/checkout.rs:165
crates/prikk-store/src/refs.rs:326
crates/prikk-store/src/patch_inverse/read.rs:30
crates/prikk-store/src/patch_replay/read.rs:34              <- missing from v1
crates/prikk-store/src/refs/evidence.rs:29, :61             <- missing from v1
crates/prikk-store/src/verify/ref_publication.rs:68, :109   <- missing from v1; inside verify
crates/prikk-store/src/refs/verify/scan.rs:248              <- missing from v1; inside verify
```

**29 including tests.** Re-derive both counts yourself before you start and report what you find — line
numbers move, and v1's figure was wrong precisely because it was not re-derived.

Each site has the envelope in hand, so `schema_version` is available at every one — this is mechanical, not
architectural. **The three sites inside `verify` are mechanical too**; see the falsification test.

**Field 7 must be rejected at schema 1.** A schema-1 payload carrying it is malformed, and saying so is what
makes the transition meaningful rather than decorative.

## Step 3 — format transition

An older reader encountering a schema-2 closed ref state will reject it — `decode_canonical` rejects unknown
tags and `RefKind::from_code` rejects unknown codes, so there is no forward-compatible route and this is a
real break. That is expected and accepted; DC-40 established the transition machinery.

Follow DC-40's format-1/format-2 rules and **evidence the behaviour when an older reader meets a closed ref
state** (criterion 8). "It breaks" is an acceptable answer; an unevidenced assumption is not.

## Step 4 — command surface

**`prikk branch close <name>`** publishes the closing ref state. Maintainer-signed, like every other ref
state — `signature.rs:49-50`, `Maintainer = 2`, "publishing/sealing a block or ref state". Reuse
`maintainer_signer_from_env`; do not add a signing path.

Fail closed when:

- `<name>` does not exist
- `<name>` is already closed
- `<name>` owns a **non-empty** active WAL — reuse `require_active_ref_for_non_empty_wal` and cite
  **DC-13 goal 4**; do not restate the rule

**No "last remaining branch" guard.** Unlike deletion, closure leaves everything reachable, so a repository
whose only branch is closed is recovered by reopening it. Do not add a guard the design does not need.

**`prikk branch list`** hides closed refs by default; **`--all`** shows them, marked. Obligation 3 confirmed
this filter has exactly one home: `refs.rs:177` `list_ref_pointers` is the only enumerator besides `verify`'s
`read_pointers`, and every other path resolves refs by name via `DEFAULT_CHECKOUT_REF`.

**Reopening** is an ordinary CAS update from the closed state. Permitted.

**Output must say nothing was reclaimed and the branch remains recoverable.** A user typing what they think
is delete should learn otherwise immediately, from the command, not from documentation.

## Traps

- **Changing what any of the four falsification-test functions decides.** Stop and report instead. Passing a
  schema argument through them is not this.
- **Emitting field 7 unconditionally.** Moves every existing RefState ObjectId. The single worst outcome
  available here.
- **Accepting both an absent and an explicitly-false field 7.** Two encodings of one state; canonical
  encoding forbids it.
- **Accepting field 7 at schema 1.**
- **Naming it `delete`**, or writing output that implies space was freed.
- **Adding a config file, a marker file, or an unsigned closure path.** An unsigned marker was considered and
  rejected: it would let a branch be hidden by a plain file write, bypassing the authority every other ref
  mutation requires.
- **Adding a last-remaining-branch guard.**
- **Trusting this handoff's line numbers.** v1's were wrong and incomplete. Re-derive.

## Definition of done

Field 7 added, emitted only when closed, rejected at schema 1, with exactly one canonical representation;
schema-aware decoding through **all 18** call sites; `branch close` publishing a maintainer-signed schema-2
ref state with the pointer intact; the three fail-closed conditions; `branch list` filtering with `--all`;
reopening working as an ordinary CAS update; output stating nothing was reclaimed; format-transition
behaviour evidenced; **`verify`, `publish`, `recoverable_missing_ref`, and `doctor` behaviourally
unchanged** — mechanical schema propagation excepted and expected.

## Submit with

The diff; **explicit confirmation that the four falsification-test functions are behaviourally unchanged, and
a list of any mechanical edits made to them** so the reviewer can check each is signature propagation and
nothing more; evidence that an ordinary (non-closed) ref state's payload bytes and ObjectId are unchanged —
this is the identity claim and it should be asserted by test, not by inspection; a commit to an unrelated ref
succeeding after a closure (criterion 2, the DC-60 regression); corruption detection still reported and
blocking for pointer-absent-log-present at every record count (criterion 3, tested by simulating pointer loss
as `seal_rejects_missing_pointer_with_ref_log_history` does); **the call-site count you actually found**; the
format-transition evidence; test counts per touched crate before and after; and the full gate set from
`rfcs/EXECUTION-ORDER.md` §6 rule 9 run on a **clean checkout of the commit**, stated as such.

## Standing request

Three RFCs in this program have been redesigned or scoped down because implementation found something design
review missed — DC-57, DC-60, and DC-61 itself. Each time the report was worth more than the code would have
been. **v1's two errors are a fourth instance, caught on my side this time rather than yours.** If something
here contradicts a shipped RFC, an accepted requirement, or the four functions the design depends on leaving
alone, stop and report it.
