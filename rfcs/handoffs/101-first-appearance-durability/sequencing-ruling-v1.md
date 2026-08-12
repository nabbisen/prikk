# RFC 101 / DC-95 Sequencing — Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-rfc101-dc95-sequencing-question-v1.md`.

**Right to raise it, and the question is better than its three options.** None of them accounts for a
technical interaction between the two increments. Ruled below: **option 1, narrowed** — RFC 101's §5.1
through §5.3 run first, then DC-95 round 9 resumes, with §5.4–§5.6 alongside it.

## 1. The interaction none of the three options names

DC-95's classified inventory puts the eleven remaining rows at **5 in `refs/verify.rs`/`scan.rs`, 4 in
`wal.rs`/`rollback_verify.rs`, 1 in Active-WAL metadata, 1 in `commit_index.rs`/`lifecycle_cache`.**

**Ten of the eleven are in ref-publication and WAL machinery — exactly what RFC 101 would restructure.**

That converts this from a preference between two independent queues into a real dependency. DC-95's
tests assert that `verify` detects specific malformed on-disk states. If RFC 101 changes what the
on-disk states *are*, fixtures built against today's pointer and log format need rebuilding, and some
checks stop existing while new ones appear.

**Nothing is invalidated yet** — §5 is investigation, and no design exists. But work done on those ten
rows before we know whether ref publication changes is work placed at risk for no gain.

## 2. Why the narrowing, rather than "all six prerequisites first"

The six prerequisites are not equally decisive, and they do not all point the same way.

**§5.1–§5.3 are the survival gate.** §5.1 (is the WAL created at `init` or lazily) is one code path.
§5.2 enumerates every transition needing a name that did not previously exist. §5.3 asks, per
transition, whether fixed-name routing eliminates the requirement — and **a transition that cannot be
routed is a stop-and-report that ends the RFC.** These three answer the only question that matters to
sequencing: *does ref publication change at all?*

**§5.4–§5.6 point the other way.** §5.4 asks what `verify` and `doctor` must say about a repository
crashed mid-replay. **That question is easier with DC-95's inventory further along, not harder** — the
inventory is the map of which `verify` checks are currently load-bearing, and RFC 101's constraint 3
forbids regressing recoverability below today's ceiling. You cannot show a new design does not lose a
guarantee without knowing which checks carry it today.

So running §5.4 early would be running it with worse inputs.

## 3. The ruling

1. **RFC 101 §5.1 first.** Cheapest, gates the rest. If the WAL is created lazily, say so plainly and
   state whether creation can move to `init` without changing a guarantee.
2. **Then §5.2 and §5.3, to the stop-and-report gate.** This is the round's real work and the point at
   which the RFC either survives or ends.
3. **Then DC-95 round 9 resumes**, with §5.4–§5.6 available to interleave.

**If §5.3 returns a stop-and-report, RFC 101 ends and DC-95 continues unaffected** — and we will have
spent the minimum to find out. That asymmetry is the whole reason for the narrowing: the cheap branch
is also the decisive one.

## 4. On the priority question underneath

The dev team framed option 1 as *"the owner's Windows-parity direction outranks continued verify-
coverage work."* **It does** — the owner stated it on 2026-08-12 and accepted RFC 101 knowing 0.20.0
moves. That is settled and does not need re-asking.

**But it is not why this ruling orders things as it does.** Even at equal priority, answering the cheap
decisive question before investing ten rows of fixture work in machinery that may change is the right
order. The priority direction and the technical dependency happen to agree here; if they had
disagreed, I would have said so rather than let the direction carry a technical call.

## 5. What does not change

- **DC-95 Stage 1 is not abandoned or descoped.** Eleven rows remain and all eleven are still owed.
  This is a pause of one round, not a reduction.
- **RFC 101 §5 remains investigation only.** No design, no implementation, no production code —
  unchanged from the prerequisite handoff.
- **The three verified facts in the handoff's §2 are still the first thing to check**, and finding one
  of them wrong is still the most valuable outcome available. They are mine and I got the adjacent
  claim wrong a day earlier.

## 6. Standing

- **RFC 101:** §5.1–§5.3 cleared and next.
- **DC-95 Stage 1 round 9:** paused for that, then resumes. §2's five rows remain next-up within it.
- Green three-platform CI before any merge, unchanged.
