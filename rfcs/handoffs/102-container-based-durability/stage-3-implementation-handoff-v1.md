# RFC 102, Stage 3 — Implementation Handoff v1

**Authorized by the project owner 2026-08-14.** Design: `design-v1.md` §2, §4, §5.
**Stages 4–6 are not authorized.** Refs, trust and compaction stay where they are.

**This is the first stage that changes the on-disk format, and the first that cannot be abandoned
cheaply once merged.** Stages 1 and 2 were format-neutral and independently valuable; this one is not.

## 1. Step 0 — four things, before any production code

The design has three unresolved items of its own (§10) and Stage 3 raises a fourth that outranks them.

**1. What happens to an existing format-2 repository?** Its objects are loose files; Stage 3's are in
containers. Two possibilities and **they are not equivalent**:

- **Reading both layouts** is a dual-path storage model — precisely the shape RFC 103 just spent an
  increment deleting for formats. It would reintroduce it one level down.
- **Bumping to format 3 and rejecting format-2 at open**, exactly as `layout.rs:368-373` now rejects
  format-1, is consistent with the owner's standing ruling that migration is not required in early
  development, and keeps one storage mechanism.

**My position is the second**, and the mechanics are already built — the rejection site, the message
shape, and the precedent all exist. **But it makes every existing repository unopenable, which is an
owner-level consequence, and the owner should confirm it rather than discover it.** Report the
mechanics; do not decide the policy.

**2. Container-record ordering within a type.** Must records be ordered by anything, or does the index
carry all ordering? Design §10.1 — I did not derive this.

**3. What does `verify` report for a present-but-unindexed object?** Rebuildable, so not an error — but
it is a state that does not exist today and needs a name before it has one by accident.

**4. Lookup cost with the index cold.** Is rebuild-on-open acceptable for a CLI that runs once per
command? If not, say so now — it changes the design, not the implementation.

## 2. What Stage 3 builds

- **One container per persisted object type** (`persisted_object_types()`, six), plus the index. **Every
  name allocated at `init`, including A/B slots** — the enumeration is the acceptance test.
- **Framing and the isolate-and-continue reader from Stage 2**, reused. Do not write a second one.
- **The index**: append-only, rebuildable by scanning, off the durability path.

## 3. The write protocol — the part no framing enforces

1. Append the object record to its container. **Make it durable.**
2. **Only then** append the index entry.

A crash between them leaves an object present and unindexed — recovered by rebuild, the safe direction.
**Reversed, a reader sees a valid, checksummed index entry pointing at bytes that are not there.**

**State this ordering at the call site, not only in a design document.** It is the one invariant here
that no type, checksum or framing can enforce for you.

## 4. Never `atomic_replace`

Every durability-bearing write uses `durable_append`/`durable_truncate`. `atomic_replace` creates a temp
name and renames even over an existing destination, whose Windows durability is DC-87 §3.4's open
question — the RFC's §3 correction. **Seven production sites still use it**; do not add an eighth on a
container path.

## 5. Acceptance criteria

1. **No container or index name is created after `init`** — proven by enumeration.
2. **No durability-bearing write uses `atomic_replace`** — proven at the call, and ideally by a
   behavioural test as Stage 1 did rather than by grep.
3. **The §3 ordering proven**: simulate a crash between the two appends; the object is unindexed and
   recoverable, never an entry pointing at absent bytes.
4. **Corruption isolation**: a damaged record is named and the scan continues, every other record still
   readable — inherited from Stage 2, re-proven at container scale.
5. **A repository that failed verification before still fails it.**
6. **DC-95's classification survives** — several of its 41 rows sit on object-read paths.
7. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.

## 6. Standing

- **Step 0 is reported and ruled before any production code.** Its item 1 needs an owner decision, not
  just my ruling.
- A stop-and-report remains a complete outcome, and is worth more here than in any prior stage.
- Stage 3 merges before Stage 4 is scoped.
