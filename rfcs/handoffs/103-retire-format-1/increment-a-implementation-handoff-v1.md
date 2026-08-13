# RFC 103, Increment A — Implementation Handoff v1

**Cleared to implement Increment A only.** Design: `design-v1.md`, owner-cleared 2026-08-13.
**Increment B (collapsing the single-variant `RepositoryFormat` plumbing) is not authorized.**

## 1. Read first

`design-v1.md`, then RFC 103 as amended, then your own §8 prerequisite report — its corrections are
already folded into both.

## 2. What Increment A is

1. **`b"1\n"` becomes an error** in `read_repository_format` (`layout.rs:339-346`).
2. **Delete `RepositoryFormat::LegacyV1`.**
3. Fix every resulting compile error.
4. Remove the format-1 machinery §3 of the design lists — including the pieces carrying **no `LegacyV1`
   token**: `PublicationState::LegacyLogLeading` and the refused reconstruction subsystem.
5. Update DC-95's classified inventory.

**Deleting the variant is the method, not a side effect.** Every format-1 branch becomes a compile error
rather than a dead branch, so the work is visible instead of discoverable. Do not keep the variant "for
detection" — detection reads bytes.

## 3. Do not delete these

**Two survivors, both of which a removal sweep takes by mistake:**

1. **`created_at == 0`** — stops being format-conditional, becomes unconditional malformed-data
   detection. **Load-bearing**, DC-95 round 9. Simpler, not weaker.
2. **Rollback WAL wrong-signature-length** — becomes **provably unreachable**, because format-1 was its
   only reachable path. **Keep it, untested, with the argument recorded** — round 6's ruling. Unreachable
   today is not unreachable by design.

**And `signature_diagnostics.rs` stays.** Its logic is load-bearing and it has no `RepositoryFormat` gate
at all — only its doc comment and issue-message text mislabel it as format-1 machinery. **Correct the
framing; keep the code.**

## 4. Establish the version anchor before writing the message

The design's rejection message says *"format-1 support was removed after 0.19.0"* — **that is a
placeholder I did not verify.** Establish the last release that actually supported format-1 from the
release record.

**Do not ship a message naming a version that has not shipped.** An inaccurate remedy is worse than a
vague one; if the anchor cannot be established, say so and propose wording that does not depend on it.

## 5. Acceptance criteria

1. **No format-1-specific machinery remains** — *not* "no `LegacyV1` token." Your own §8 report
   established the token misses identifiers and dead stubs; it is one instrument, not the definition.
2. **A format-1 repository is rejected at `RepositoryLayout::open`** with the design's contract, proven
   against a **real** format-1 fixture. `build_legacy_fixture`
   (`prikk-cli/tests/format_transition_support/fixture.rs:3`) exists — **retain it long enough to prove
   the rejection**, then remove it with the rest of the scaffolding.
3. **`created_at == 0` still fires, unconditionally** — probed the DC-95 way: disable, observe the
   specific failure, restore, confirm no residual diff.
4. **Rollback wrong-signature-length retained and documented as unreachable.**
5. **DC-95's classified inventory updated in the same increment** — three rows change status:
   `LEGACY-LOG-LEADS` deleted, rollback wrong-length load-bearing → unreachable, and
   `legacy_state_roots_unverifiable` deleted.
6. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.

## 6. Standing

- A stop-and-report is a complete outcome. If removing a piece turns out to break something the design
  did not anticipate, report it rather than working around it.
- **Increment A merges before Increment B is scoped.**
- RFC 102 §6.4–§6.6 is also open; neither track blocks the other.
