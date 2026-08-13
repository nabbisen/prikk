# RFC (proposed) - 103 Retire Format-1

**Status.** **PROPOSED 2026-08-13**, on the owner's direction: design *"clean, simple as possible,
reasonably functional and sophisticated, without concern about migration."*
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The format-1/format-2 duality surfacing as a complication in four consecutive DC-95
rounds, and the owner's ruling that migration from an older prikk need not be preserved.
**Target.** Owner's call. Prerequisite to RFC 102 if that ordering is taken — see §5.

## 1. The decision

**Format-1 is not supported. A format-1 repository is rejected at open, with a message that says so.**

Not read-only support, not automatic upgrade, not a compatibility shim. Each of those keeps every
dual-path branch alive, which is the cost this RFC exists to remove.

## 2. What this removes

Measured, not estimated: **22 `LegacyV1` sites across 13 files**, plus five pieces of machinery that
exist only to serve format-1:

- `active.rs::finish_legacy_active_publication_cleanup` and `authorize_legacy_active_cleanup`
- `wal.rs::truncate_empty_for_legacy_recovery`
- `verify.rs::legacy_state_roots_unverifiable` — a field, its predicate, and its assignment
- `test_support.rs::legacy_rollback_marker_signature`

And three checks whose only reason to exist is the duality:

| Check | DC-95 Stage 1 classification | Effect of this RFC |
|---|---|---|
| `PRIKK-VERIFY-REF-LEGACY-LOG-LEADS` | **Downstream-redundant** (round 10) | Deleted. Its format-2 sibling already catches the same defect — that is what round 10 proved |
| `validate_read_schema`'s `LegacyV1` branch | Load-bearing **via non-blocking sibling** (round 11) | Deleted. Round 11 established the malformed-signature defect it catches is reachable *only* under format-1 |
| `legacy_state_roots_unverifiable` | Precondition fact, not a stage output | Deleted — it can only ever be false |

**Every one of those three is documented, classified, and probed** by DC-95 Stage 1. That is why this
RFC is cheap to review: the coverage question was answered before the removal was proposed.

## 3. What this does *not* remove — and the distinction matters

**The `created_at == 0` check survives, and stops being conditional.**

`refs/verify.rs:46-52` rejects a `CurrentV2` repository containing any ref-log record with
`created_at != 0`. With format-1 retired, that is no longer *"a format-2 repository contaminated by
format-1 records"* — it is simply **malformed data**, and the check becomes an unconditional invariant
rather than a format-conditional one.

**It gets simpler; it does not get weaker.** Anyone reading §2's list and assuming "legacy checks go"
would delete it, and that would remove real malformed-data detection. Stage 1 classified it
**load-bearing**.

Similarly out of scope: DC-40's state-merkle format *design* stays. What goes is the machinery for
tolerating repositories that predate it.

## 4. The rejection contract

A format-1 repository must fail at open with a message that is **actionable, not merely accurate**:

- name the detected format and the required one,
- name the last prikk version that supported format-1,
- state the remedy — export via bundle from that version, import here.

**A bare `malformed persisted data` is not acceptable.** The one user this affects is a user upgrading,
and they will hit it exactly once, with no context. Detection must be by the `FORMAT` file's own
content, not by a downstream decode failure.

## 5. Consequence for RFC 102, which may be the larger prize

RFC 102's constraint 6 requires **"a format migration must exist for repositories already written in the
current format."** That constraint was written before this direction and is the single largest cost item
in a container-based storage redesign.

**If the owner's "without concern about migration" extends to RFC 102, constraint 6 relaxes and the
container work becomes materially cheaper.** I am flagging that rather than assuming it: dropping
migration for a *retired* format is a different decision from dropping it for the *current* one, and the
second is much bigger. **It needs its own ruling and I am not taking it here.**

## 6. Consequence for DC-95's classified inventory

Three of the 41 classified rows change status. **The inventory must be updated in the same increment,
not left to drift** — it is the map a future reader consults, and DC-95 Stage 1 spent twelve rounds
making it trustworthy.

## 7. Non-goals

- **Read-only support for format-1.** Rejected in §1; it preserves the duality.
- **Automatic in-place upgrade.** That is a migration tool, which is what the direction removes.
- **Changing format-2 itself.** This RFC deletes the alternative, not the survivor.

## 8. Blocking prerequisites

1. **Enumerate every format-1 site independently** — the 22 measured here are a starting figure from one
   grep, not a derived set. Four consecutive investigations this month found the architect's counts
   narrower than the code.
2. **Confirm each of §2's three checks is genuinely format-1-only**, from Stage 1's classification and
   the code, not from this table.
3. **Establish what a format-1 repository looks like at open today** — which code path first notices, and
   what it currently reports. §4's contract cannot be written against a guess.

## 9. Acceptance criteria

1. **No `RepositoryFormat::LegacyV1` remains in production code.** The enum variant itself may stay only
   if detection requires naming the rejected format.
2. **A format-1 repository is rejected at open with §4's message**, proven by a test using a real
   format-1 fixture, not a hand-built one.
3. **The `created_at == 0` check still fires**, unconditionally — proven by the DC-95 method: disable it,
   observe the specific failure, restore.
4. **DC-95's classified inventory updated** in the same increment.
5. Green three-platform CI.

## 10. The risk, stated rather than absorbed

**Prikk has shipped releases. Format-1 repositories may exist in the wild, and this RFC makes them
unopenable by any future version.** The remedy — bundle export from an older release — requires the user
to still have that release, or to fetch it.

**The owner has directed that migration need not be preserved, and this is that decision's cost.** It is
recorded here rather than left implicit, per the register's rule that significant risk is never silently
accepted. If the owner wants it reduced, the cheapest mitigation is a **detection-only** stub retained
indefinitely: enough format-1 knowledge to recognise and explain, never enough to read. That is §4's
contract and it is already the minimum this RFC requires.
