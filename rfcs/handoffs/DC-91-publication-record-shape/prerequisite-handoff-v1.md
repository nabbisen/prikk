# DC-91 Publication Record Shape — Prerequisite Handoff v1

**Cleared to answer §4's four questions only.** Accepted 2026-08-11,
`rfcs/done/DC-91-PUBLICATION-RECORD-SHAPE.md`. **No design, no implementation, no production code.**

## 1. What this is, and what it is not

One question: **does a fixed-name, slot-based durable publication record have independent value on
POSIX — or is it purely a Windows tax?**

It exists because DC-87 Stage 2 established that Windows cannot satisfy DC-38's *"format-2 publication
never permits an ahead log"*: step 6's log append is an existing-file content append and **is**
achievable there, while step 5's pointer promotion needs transition durability and **is not**. A record
whose transitions are content updates to an already-existing name would convert the recurring case into
the achievable one — but that means changing how prikk publishes refs **on every platform**, in the most
safety-critical machinery it has.

**A "no" is a complete, successful outcome.** Establishing that today's design is already near-minimal
in reachable states would settle a question that is currently open, and would be worth the work on its
own. Do not soften it if that is what you find.

## 2. The owner's criterion, and precisely what it does and does not settle

> "I prefer one which is more secure and more robust. It's true minimalism is very important but, in
> production use, the stable performance and the data integrity is more important."

**This is the deciding rule, not the answer.** It settles *how* to weigh the comparison — robustness and
data integrity outrank minimality of change, so "DC-41's matrix already proves the current design" is a
**cost to state honestly**, never a reason the comparison ends there. It does not settle *which* design
wins, because nobody has made the comparison yet. That is §4.

My original framing carried the opposite bias and is corrected in the RFC's §1a. If you find yourself
reaching for "but the current one is already tested" as decisive, that is the bias the owner overruled.

## 3. Where to start

**§4.1 first — enumerate today's reachable interruption states**, from DC-38's own seven-step list and
DC-41's failpoint matrix, **not from first principles**. The existing machinery is the baseline and it
must be characterised accurately before anything is compared to it. How many states are there, which
does `doctor` classify, which need signer-backed retry to finish?

**§4.2 is the comparison, and it is where a "no" would come from.** If the slot record's state count is
not meaningfully lower, say so plainly and this RFC ends there.

**Score more than state count.** Per the owner's criterion, robustness has at least three axes and the
current design should be scored on all of them, not just the first:

- **Count** — how many reachable interruption states.
- **Detectability** — can a bad state be recognised from the artifact's own bytes, without external
  context?
- **Recoverability** — can it be finished without operator judgment?

**§4.3 is the one I expect to be underestimated.** The slot record's one-time creation still needs a
name to appear. Ref logs, WAL files and trust-store entries would still require first-appearance
durability. **A partial fix that leaves DC-38's invariant still broken on Windows is worth discovering
now, not after a design.** Answer specifically whether moving only the *pointer* to a slot record
changes the answer.

**§4.4 — cost in what would have to be re-proved**, not in hours. DC-41's crash matrix and DC-38's
failpoint requirements are the units.

## 4. Limits

- **No design and no implementation.** §5 criterion 3 is explicit: even a "yes" comes back as its own
  increment, and no publication code changes under this RFC.
- **No change to DC-38's guarantees in either direction.** This asks whether the same guarantees can be
  delivered by a different mechanism with fewer states — never whether to weaken them. If the shape
  appears to require a change, stop and report.
- **No Windows work**, and no deciding DC-87 Stage 2's fate — that is the owner's, informed by this.
- **"I could not determine this" remains a first-class answer.**

## 5. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer §4 in order. Findings outside scope go in the
report; I register them in `FINDINGS.md`.

## 6. Sequencing

- **DC-92 is blocked on a CI fix** (`ci-failure-report-v1.md`) — the cross-target gate. That fix comes
  first; it is small and mechanical.
- This touches no production code, so it will not collide with DC-92's fix or its eventual merge.
- DC-87 Stage 2 and its Stage 1 seam remain deferred under the owner's accepted, tracked deferral, with
  this increment's answer as the named unblocking condition.
