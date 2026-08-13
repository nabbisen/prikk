# RFC 103 — Design v1

**Author.** Architect. **Independence.** Author-reviewed — the standing ceiling.
**Inputs.** RFC 103 as amended 2026-08-13, and §8's prerequisite report, which corrected the RFC's own
machinery list and found a module it had missed.
**Status.** Design for review. **No implementation authorized by this document.**

## 1. The central design decision: delete the enum variant, detect the bytes

`read_repository_format` (`layout.rs:339-346`) matches `.prikk/FORMAT`'s exact bytes today: `b"1\n"` →
`Ok(LegacyV1)`, `b"2\n"` → `Ok(CurrentV2)`, else `Err`.

**Design: `b"1\n"` becomes an error at that site, and `RepositoryFormat::LegacyV1` is deleted.**

Not "keep the variant for detection." Detection reads raw bytes; it does not need a variant. **And
keeping the variant is what makes reintroducing a format-1 branch possible** — while it exists, any
`match` on `RepositoryFormat` can grow a legacy arm, and the whole point of this RFC is that the
dual-path shape stops existing. Deleting it makes the compiler the enforcement mechanism rather than a
convention.

**Consequence, stated plainly:** every `if format == LegacyV1` / `match` arm becomes a compile error, not
a silent dead branch. That is the desired failure mode — the work is visible, not discoverable.

## 2. Rejection happens at open, and only there

`read_repository_format` ← `RepositoryLayout::new` ← `RepositoryLayout::open`. Erroring at the first
means every one of the 13 CLI call sites and `seal.rs`'s direct `open` inherit the rejection with no
per-command work.

**Today's behaviour, which this replaces:** a format-1 repository **opens successfully**, prints a
warning to stderr, and is only refused at mutation time by `require_current_format`, rendering as the
bare `"unsupported format version: 1"`.

**So two things move**: the refusal moves from mutation time to open time, and the message stops being
bare.

**The contract**, per the RFC's §4 — it must name the detected format, the required one, the last
supporting version, and the remedy:

```
this repository uses format 1, which prikk no longer supports (this version requires format 2).
format-1 support was removed after 0.19.0. to migrate: use prikk 0.19.0 or earlier to
`prikk bundle export`, then `prikk bundle import` here.
```

**Do not ship a message naming a version that has not shipped.** If 0.19.0 is not the right anchor at
implementation time, correct the text — an inaccurate remedy is worse than a vague one.

## 3. What is removed, what is kept, and the two that will be got wrong

**Removed:** the `LegacyV1` variant and its 22 token sites; `finish_legacy_active_publication_cleanup`;
`authorize_legacy_active_cleanup`; `truncate_empty_for_legacy_recovery`; `legacy_state_roots_unverifiable`
(field, predicate, assignment); `PublicationState::LegacyLogLeading` and its six gated sites; the refused
reconstruction subsystem (`RefRecoveryCandidate`, `RefRecoveryRepair`, `recoverable_missing_ref`,
`reconstruct_missing_ref_from_log`, `DoctorRepairOptions::reconstruct_main_ref` and its refusal branch);
`PRIKK-VERIFY-REF-LEGACY-LOG-LEADS`; `validate_read_schema`'s `LegacyV1` branch; format-1 CLI warnings;
the format-1 test scaffolding.

**Kept — and these are the two a removal sweep takes by mistake:**

1. **`created_at == 0`** — stops being format-conditional, becomes unconditional malformed-data
   detection. **Load-bearing** (DC-95 round 9). Simpler, not weaker.
2. **Rollback WAL wrong-signature-length** — becomes **provably unreachable**, because its only reachable
   path was format-1. Round 6's ruling applies: **keep, untested, argument recorded.** Unreachable today
   is not unreachable by design.

**Also kept, with its framing corrected:** `signature_diagnostics.rs`. Its logic is load-bearing and
carries no `RepositoryFormat` gate at all; only its doc comment and issue-message text mislabel it as
format-1 compatibility machinery.

## 4. Staging — two increments, not one

**Increment A — reject and remove.** Delete the variant, error at `read_repository_format`, fix every
resulting compile error, remove the machinery above, update the DC-95 inventory.

**Increment B — collapse the plumbing, optional and separately decided.** `RepositoryFormat` becomes a
single-variant enum threaded through many signatures. Removing it entirely is a wide mechanical diff with
no behavioural content. **It is not part of A**, and it may never be worth doing.

**A must merge before B is scoped.** Bundling a behavioural change with a workspace-wide signature sweep
is how a reviewer loses track of which half a failure came from — the same reason DC-95 Stage 2 was split.

## 5. Acceptance criteria

1. **No format-1-specific machinery remains** — not "no `LegacyV1` token." §8 established the token
   misses identifiers and dead stubs; the token is one instrument for finding the machinery, not its
   definition.
2. **A format-1 repository is rejected at `RepositoryLayout::open`**, with §2's contract, proven against
   a **real format-1 fixture** — `build_legacy_fixture` exists today and must be retained long enough to
   prove the rejection before it is removed with the rest of the scaffolding.
3. **`created_at == 0` still fires, unconditionally** — probed by the DC-95 method: disable, observe the
   specific failure, restore.
4. **Rollback wrong-signature-length is retained and documented as unreachable**, not deleted.
5. **DC-95's classified inventory updated in the same increment** — three rows change status.
6. Green three-platform CI.

## 6. Open item for the implementation round

**The version anchor in §2's message.** It must name the last release that actually supported format-1,
and I have not verified which that is. Establish it from the release record, not from this document.

## 7. Independence

Author-reviewed, and **§8 already corrected this RFC's own machinery list and found a module it missed** —
so the design above is written on a base that needed correcting once. The compensation is §5's first
criterion, which is deliberately not expressible as a grep.
