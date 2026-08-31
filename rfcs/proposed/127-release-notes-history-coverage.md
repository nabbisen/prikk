# RFC 127 — A released version's changelog heading was destroyed, and the gate cannot see it

**Status.** **Proposed.** Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-2.md` V3). **Independently confirmed, including the destroying commit.**

**Tracks.** Release integrity. Touches `CHANGELOG.md` and `tools/release-policy`.

---

## 1. The defect, with the diff that caused it

I checked every released tag for a matching `## X.Y.Z — DATE` heading:

```
0.27.1 → 1   0.27.0 → 1   0.26.0 → 1   0.25.0 → 1
0.24.0 → 1   0.23.0 → 0   0.22.1 → 1   0.22.0 → 1
```

`0.23.0` has none. The cause is `5964ad6` ("release: bump workspace to 0.24.0 and add the changelog
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
has exactly one matching heading in `CHANGELOG.md`.* Once that holds, a future bump that replaces
instead of inserts fails at the next release rather than silently.

**Design note worth deciding explicitly:** the gate needs a tag list. Reading `git tag` makes the
gate depend on repository state and on tags being fetched; a checked-in list is derived data that can
itself go stale. **Recommendation: read the tags, and fail loudly when none are present** rather than
passing vacuously — a gate that silently checks nothing when tags are missing would repeat this
finding's own shape.

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
