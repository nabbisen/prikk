# RFC (proposed) - DC-91 Publication Record Shape

**Status.** **PROPOSED** — needs the project owner's acceptance. **This is an evaluation, not a
commitment to change anything.** §5 states what a "no" outcome looks like and why it is a good result.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** DC-87 Stage 2's transition-durability investigation, 2026-08-11, which established that
DC-38's invariant cannot hold on Windows under the current publication design.
**Target.** Owner's call. DC-87 Stage 2 is blocked either way until this is answered or abandoned.

## 1. The question, and only this question

> **Does a fixed-name, slot-based durable publication record have independent value on POSIX — or is it
> purely a Windows tax?**

Everything else follows from the answer, and nothing else is in scope.

## 1a. The owner's deciding criterion, 2026-08-11

> "I prefer one which is more secure and more robust. It's true minimalism is very important but, in
> production use, the stable performance and the data integrity is more important."

**This changes how §1's question is weighed, and the change is on the record because the architect's
original framing carried the opposite bias.** §2 and §5 below were written leaning toward *do not
disturb proved-safe machinery without cause* — treating "the current design is already tested" as close
to decisive.

It is not decisive. It is a **cost**, to be stated honestly and weighed against robustness and data
integrity, which rank above minimality of change. Concretely, for §4:

- A design that is **more robust** does not lose because it would require re-earning evidence. Report
  the re-proving cost as a cost; do not let it settle the comparison.
- "Fewer crash states" is not the only robustness axis. **Detectability** (can a bad state be recognised
  from the artifact's own bytes?) and **recoverability** (can it be finished without operator judgment?)
  count equally, and the current design should be scored on them too, not just on state count.
- Performance stability is in scope where the shapes differ — a fixed-size slot write versus a
  write-then-rename-then-directory-sync sequence have different costs and different variance.

## 2. Why it is worth asking

DC-87 Stage 2 found that every durability-bearing transition splits in two: an **update of an existing
name**, which needs only content durability, and a **first appearance of a name**, which needs naming
durability. Windows can do the first and not the second. Today prikk's ref publication depends on the
second — a candidate file is written and then *renamed* into place, and the rename's durability is what
POSIX's directory `fsync` provides and Windows has no equivalent for.

A record whose transitions are content updates to an already-existing name — two slots, each with a
sequence number and a checksum, always overwriting the stale one — needs no new name after its
one-time creation. That converts the recurring case into the achievable one.

**That is a Windows argument. This RFC exists because there may be a POSIX argument too, and it should
be found or ruled out before anyone builds anything.** Candidate reasons it might stand on its own:

- **Fewer reachable crash states.** Today's candidate-write-then-promote sequence has interruption
  points DC-38 enumerates and `doctor` has to classify. A slot record's recovery rule is "pick the
  valid slot with the higher sequence number," which is decidable from the file's own bytes.
- **Less dependence on directory-sync ordering**, which is the part of DC-38's reasoning hardest to test
  and the reason DC-41's failpoint matrix is as large as it is.
- **Self-describing recovery.** A checksummed slot pair distinguishes "torn write" from "valid older
  state" without consulting anything else — the property prikk already relies on everywhere else it
  content-addresses.

These are hypotheses. **They may not survive contact with DC-38's actual state machine**, which is
exactly what §4 is for.

## 3. What this is not

- **Not a Windows increment.** If the answer is "independent value: yes," Windows benefits as a side
  effect. If "no," Windows is a separate decision (DC-87 Stage 2's §5 options 2 and 3).
- **Not a commitment to change ref publication.** §5's "no" is a legitimate, cheap outcome.
- **Not a reopening of DC-38's guarantees.** It asks whether the same guarantees can be delivered by a
  different mechanism with fewer states, never whether to weaken them.

## 4. Blocking prerequisites

1. **Enumerate today's reachable interruption states** for one ref publication, from DC-38's step list
   and DC-41's failpoint matrix — not from first principles. How many are there, which does `doctor`
   classify, and which require signer-backed retry to finish?
2. **Enumerate the same for a slot record.** If the count is not meaningfully lower, the independent-value
   case fails on its own terms and this RFC ends. **Say so plainly if that is what you find.**
3. **What does the slot record NOT cover?** Its one-time creation still needs a name to appear. What else
   — ref logs, WAL files, trust-store entries — would still require first-appearance durability, and does
   the answer change if only the *pointer* moves to a slot record? A partial fix that leaves DC-38's
   invariant still broken on Windows is worth knowing about early.
4. **Cost, honestly.** This touches the most safety-critical machinery in the product, with the largest
   existing test matrix. Estimate the blast radius against DC-41's matrix and DC-38's failpoint
   requirements — not in hours, in *what would have to be re-proved*.

## 5. Acceptance criteria

1. §4 answered and reported before any design.
2. **A "no" is a complete, successful outcome**, reported as such and not softened. Recording that the
   current design's state count is already near-minimal would settle a question that is currently open
   and is worth the investigation on its own.
3. If "yes": a design proposal comes back as its own increment. **This RFC does not authorize
   implementation**, and no publication code changes under it.
4. **No change to DC-38's guarantees** in either direction. If the shape appears to require one, stop and
   report.

## 6. Non-goals

- Any implementation.
- Any Windows work.
- Any change to `DurabilityContract` — DC-88 established that `atomic_replace`/`promote`/`durable_append`
  are already requirement-shaped and permit any satisfying implementation.
- Deciding DC-87 Stage 2's fate. That is the owner's, informed by this.
