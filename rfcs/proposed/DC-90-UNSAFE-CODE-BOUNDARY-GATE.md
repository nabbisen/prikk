# RFC (proposed) - DC-90 Unsafe Code Boundary and Gate

**Status.** **PROPOSED** — needs the project owner's acceptance.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The owner's ruling of 2026-08-10: *"`unsafe` is allowed under control with safety and
maintainability preserved."* This increment is what turns "under control" from an intention into a
checked property.
**Target.** 0.20.0, **before the first line of `unsafe` is written** — see §2.

## 1. What this is for

The ruling permits `unsafe`. It does not describe the control. Left unwritten, "under control" decays
into whatever the first increment that needs FFI happens to do, and the boundary becomes a description
of history rather than a constraint on it.

prikk already has the machinery for exactly this shape of problem. DC-51's dependency-placement gate
(`tools/release-policy/src/boundary/placement.rs`) does not ask developers to remember where third-party
crates may live; it fails the build when they land somewhere else, and it covers
`[target.*.dependencies]` so a platform-specific addition cannot slip past. **An unsafe boundary is the
same shape and should be enforced the same way.**

## 2. Why the ordering matters

**A boundary added before the first `unsafe` constrains it. One added after documents it.** If FFI lands
first and the gate second, the gate is written to accept whatever exists — which is not a gate. This is
the same reasoning DC-82 used to collapse the mutation dispatch *before* the Windows backend rather than
after, and it held there.

This increment is small. It should not become the reason Windows work waits, and §6 sequences it so it
does not.

## 3. Candidate shape — to evaluate, not to inherit

Offered so §4 starts from a proposition. **Not a ruling**; the architect's design assertions in this area
have needed correction more than once.

**One crate, named in one place, checked by the tool.** A single workspace crate — the only one without
`forbid(unsafe_code)` — wraps every FFI call and exposes a safe API, the same relationship `rustix`
already has with `prikk-store`. A new `boundary` check would then enforce, at minimum:

- **Exactly one crate may omit `forbid(unsafe_code)`**, and it is named in an allowlist in the tool, not
  inferred. Every other workspace crate must carry the lint. A second crate dropping it fails the gate.
- **The unsafe crate may not depend on any product crate.** It sits at the bottom; nothing about prikk's
  data model or trust rules may be reachable from inside it.
- **Its third-party dependencies are separately allowlisted**, so the exception cannot become a
  side-door around DC-51's placement gate.
- **Every `unsafe` block carries a `SAFETY:` comment.** Mechanically checkable, and the thing most
  likely to rot silently.

**Maintainability, which the ruling names explicitly and a gate cannot check:** the crate's API should be
stated in guarantees, not primitives — the same standard DC-76 set for `DurabilityContract`, and the
standard `durable_directory_entry` failed, which is why DC-88 exists.

## 4. Blocking prerequisites

1. **Can the release-policy tool see what it needs?** It reads manifests today
   (`boundary/placement.rs`). Detecting a missing `forbid(unsafe_code)` and un-commented `unsafe` blocks
   means reading Rust source. Report whether that fits the tool's existing shape or is a genuinely new
   capability — and if the latter, what it costs. **A gate that is expensive to build badly is worse
   than a documented convention honestly labelled as one.**
2. **What is already true?** Every workspace crate's current lint posture, stated as measured fact.
   The architect believes it is `forbid` everywhere via the workspace lint table; confirm it.
3. **Does `cargo-geiger` or an existing tool already do this** better than a bespoke check? Report
   before building. A new dependency in `tools/` is cheaper than one in a product crate, but it is not
   free.
4. **What can a gate not check?** Enumerate honestly. The FFI-ABI-correctness risk is not
   machine-checkable at this layer (see the DC-87 unsafe-surface analysis §7), and the gate must not
   imply otherwise. Anything the gate cannot see needs a stated review obligation instead.

## 5. Acceptance criteria

1. §4 answered and reported before design.
2. **The boundary is enforced by a gate that fails**, demonstrated by a negative control: introduce a
   violation of each rule and show the specific check firing, per DC-86's and DC-76's precedent.
3. **The gate passes on today's tree**, where no crate has an unsafe exception yet — an allowlist of
   zero must be a valid, checked state, not a special case that only starts working once something is
   added.
4. **The rules are documented where a contributor meets them**, not only in the tool.
5. **What the gate cannot check is stated plainly**, in the tool's own documentation, with the review
   obligation that covers it. Criterion 2's standard from DC-87 applies: a passing check is not evidence
   of a guarantee it does not test.
6. Gate set per `EXECUTION-ORDER.md` §6 rule 9.

## 6. Sequencing

**This does not block DC-88, and DC-88 does not block this.** DC-88 is the critical path for Windows;
this increment touches `tools/release-policy` and manifests, which DC-88 does not.

**It must land before the first `unsafe` line**, which today means before DC-87 Stage 2 — and Stage 2 is
already blocked on DC-88, so there is room. If that ordering ever conflicts, the conflict comes back to
the owner rather than being resolved by whichever increment moves first.

## 7. Non-goals

- **Deciding whether prikk writes its own FFI or adopts `cap-std`.** That decision is still open and is
  to be made against measured numbers, per the DC-87 unsafe-surface analysis §8. This increment
  constrains either outcome.
- **Any Windows implementation.**
- **Formal verification.** Verus does not mitigate the FFI risk this boundary exists to contain, and
  coupling them would buy assurance in the wrong place. If it is pursued, it is its own proposal — most
  plausibly against DC-88's crash-consistency state machine.
- **Retroactively auditing `rustix`'s or any other dependency's internal unsafe.** This boundary governs
  code prikk writes.
