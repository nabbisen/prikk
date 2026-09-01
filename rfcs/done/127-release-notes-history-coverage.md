# RFC 127 — A released version's changelog heading was destroyed, and the gate cannot see it

**Status.** **COMPLETE, 2026-09-01.** `0.23.0`'s heading restored and the gate landed (`1edde4c`),
the pre-convention tags ruled to an exemption list (§3.3), and that exemption made self-guarding
(`e8a10d5`). CI green on all 15 jobs — including the four that now fetch tags, which is what proves
the gate actually runs in CI rather than passing vacuously there.

**Two things worth carrying forward.** The gate's blind spot was **structural, not careless**:
`release_notes::assemble` validated only the tag being cut, and every release edits the top of the
file — so the check was blind exactly where the damaging edit happens. **A gate that validates only
the thing being changed cannot detect damage to the things that are not.** And §3.3's exemption is
self-guarding for the same reason the gate exists: an allowlist entry that has become untrue must
break the build until someone deletes it.

Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-2.md` V3). **Independently confirmed, including the destroying commit.**

**Tracks.** Release integrity. Touches `CHANGELOG.md` and `tools/release-policy`.

---

## 1. The defect, with the diff that caused it

I checked the eight most recent released tags for a matching `## X.Y.Z — DATE` heading:

```
0.27.1 → 1   0.27.0 → 1   0.26.0 → 1   0.25.0 → 1
0.24.0 → 1   0.23.0 → 0   0.22.1 → 1   0.22.0 → 1
```

`0.23.0` has none.

**Correction, 2026-09-01 — this said "every released tag" and it was eight of forty-four.** The
implementing increment checked all 44 and found **two more failures, both pre-convention and both
unrelated to the 0.23.0 defect**: `0.0.1` has no heading of any shape, and `0.1.1`'s reads
`## 0.1.1 Housekeeping` — no ` — `, no date. §3.3 records the ruling. **A range-bound claim stated as
exhaustive is the same error this project has caught twice in sweeps**, and it was mine here. The cause is `5964ad6` ("release: bump workspace to 0.24.0 and add the changelog
entry"), which **replaced** the heading instead of inserting above it:

```diff
-## 0.23.0 — 2026-08-23
+## 0.24.0 — <cut date, set by the owner at tag time>
```

**Consequence:** 0.23.0's entire body — including the whole `prikk sync` feature — now reads as part
of 0.24.0. A reader asking "what shipped in 0.23.0?" gets a wrong answer from the project's own
release record, and a reader asking "what shipped in 0.24.0?" gets an inflated one.

## 2. Why the gate did not catch it, and cannot

`release_notes::assemble(root, tag, dist_dir)` (`release_notes.rs:57-95`) extracts **only the section
for the tag being cut** and fails the release if that one heading is missing. It never looks at any
other version.

So the gate's guarantee is exactly *"the version being released has a heading"* — and the failure
mode that actually occurred is *"a version released earlier stopped having one."* The gate is not
wrong; **its scope has a blind spot precisely where the edit that damages it happens**, because every
release edits the top of this file.

## 3. Two changes, in this order

**3.1 Restore the heading.** Insert `## 0.23.0 — 2026-08-23` above the point where 0.23.0's body
begins, and verify by reading `b6cd309` (the 0.23.0 tag) that the split lands where the original
section started — **derive the boundary from the tagged content, not from where the prose looks like
it changes topic.**

**3.2 Extend the gate to cover history.** The check becomes: *every tag reachable in this repository
has exactly one matching heading in `CHANGELOG.md`* — subject to §3.3, which the literal wording here
did not anticipate. Once that holds, a future bump that replaces
instead of inserts fails at the next release rather than silently.

**Design note worth deciding explicitly:** the gate needs a tag list. Reading `git tag` makes the
gate depend on repository state and on tags being fetched; a checked-in list is derived data that can
itself go stale. **Recommendation: read the tags, and fail loudly when none are present** rather than
passing vacuously — a gate that silently checks nothing when tags are missing would repeat this
finding's own shape.

### 3.3 Two pre-convention tags, and the ruling on them — added 2026-09-01

Checking all 44 tags surfaces `0.0.1` (no heading at all) and `0.1.1` (`## 0.1.1 Housekeeping`,
no dated form). Both predate the `## X.Y.Z — DATE` convention, which begins holding at `0.1.2`.
Three options were put to the architect: **A** name them in an exemption list, **B** backfill the two
headings, **C** bound the gate to a version cutoff.

**RULED BY THE ARCHITECT 2026-09-01: A, with a self-guard.**

- **The gate exists to catch a regression *against the convention*.** These two tags were never in
  the convention's shape, so there is no regression to catch — naming that fact is more truthful than
  manufacturing conformance for it.
- **A is this project's own idiom for exactly this shape**, three times over: `UNSAFE_EXEMPT_CRATES`,
  `DECLARED_UNDOCUMENTED`, `RFC114_ADMITTED_BUT_UNWRITTEN` — allowlists whose honesty lives in the
  requirement that every entry state a real reason. RFC 130 §4.1 ruled the same shape for the
  coupling gate two hours earlier; ruling differently here would contradict it.
- **B writes history that was never written** — a date for `0.0.1` that no one recorded, and the
  deletion of "Housekeeping" from `0.1.1`'s heading. §6's "no retroactive editing of any release
  body" exists for that instinct.
- **C draws an arbitrary line** and asserts a cutoff this RFC never established.

**The self-guard is the amendment.** An exemption that merely says "do not check these" goes stale
silently: if `0.0.1` ever gained a conforming heading, the gate would keep skipping it and the list
would quietly mean nothing. **Each exempt tag must be verified to genuinely lack a conforming
heading**, so that fixing one *fails the gate* until the entry is removed — the same reasoning
`unsafe_boundary.rs` states as *"a control the controlled party can silently remove is a convention,
not a control."*

**Observation, not a defect:** `0.1.2`'s heading is `## 0.1.2 — DC-09 Phase 4.3 / 4.4 internal
node-model groundwork`. It satisfies the `## X.Y.Z — ` shape and therefore the gate, but its suffix
is prose rather than a date. The gate deliberately matches `release_notes::changelog_section`'s own
pattern — **the two must agree about what a heading is**, and tightening one without the other would
be worse than this. Recorded so no one later assumes the suffix parses as a date.

## 4. Why this is worth an RFC rather than a one-line fix

The restoration is one line. The gate change is small. **What makes it RFC-shaped is that both touch
the release lane**, which is owner-authorized territory: the gate is what stands between a mistaken
edit and a published artifact, and widening its scope changes what can block a cut. That is a
decision to record, not a fix to slip in.

It also sets a precedent worth stating once: **a gate that validates only the thing being changed
cannot detect damage to the things that are not.** The Open-Work Index gate (RFC 120) got this right
by checking both directions; this one did not.

## 5. Non-goals

No changelog format change. No automation of changelog authoring. No retroactive rewriting of any
release body — only the missing heading is restored, and the text beneath it is left exactly as it
was written.
