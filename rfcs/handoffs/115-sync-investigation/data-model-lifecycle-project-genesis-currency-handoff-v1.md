# `data-model-lifecycle.md` still documents `ProjectGenesis` as live

**Base:** current `main` (`628bb63`). **Under `003-landing-work-on-main.md`.**
**Origin:** found while fixing the conflict-arbitration theme, reported and correctly not bundled.
**This is my omission**: the repository-identity settlement told you to delete the variant from code
and record the ruling in `trust-threat-model.md`. **It never said to sweep the docs.**

---

## 1. What is wrong

**`docs/src/reference/data-model-lifecycle.md:25`**, in the live object taxonomy:

```
| `0x0A` | **ProjectGenesis** | Project identity anchor; its id *is* the `project_id` | `objects/` |
```

**That asserts a repository-identity concept the settlement ruled does not exist.** Repositories are
anonymous; identity lives in signer keys and patch ids. There is no `project_id`.

**Lines 298-300** describe it as *"a reserved type code with no payload module"* that
`validate_format2_schema` refuses. **That was true before `c1046b7`. It is not now.** The variant is
**deleted**, and `id.rs`'s `RETIRED_CODES` makes `from_code(0x0A)` return a hard retirement error —
*"object type code 10 is retired (formerly project-genesis) and must never be reused."* **Retired is
not reserved.**

## 2. The rest of the table is correct — verified, so do not re-audit it

I compared every row against `ObjectType::ALL`:

```
table:  0x01..0x09, 0x0A, 0x0B          (11 rows)
ALL:    0x01..0x09,       0x0B          (10 variants)
```

**Exactly one row is wrong.** Every other code, name, and ordering matches. **Delete the `0x0A` row;
change nothing else in the table.**

## 3. Do not leave the gap unexplained

With `0x0A` gone the table runs `0x09` then `0x0B`. **A reader will notice and wonder.** Lines 298-300
are the right place to answer it — **rewrite them rather than deleting them**: `0x0A` was
`ProjectGenesis`, removed with the repository-identity settlement, and the code is **permanently
retired and must never be reassigned**.

**Keep it short.** One or two sentences. The reasoning behind the settlement belongs in
`trust-threat-model.md`, which already carries it — **link, do not restate.**

**`trust-threat-model.md:147` is correct and current** (*"no repository identity for a future
increment to eventually add trust to"*). **Do not touch it.**

## 4. Adjudicate: should the table be bound to `ObjectType::ALL`?

**This table is a hand-maintained transcription of an enum that now has a derived inventory.** It went
stale **this session**, one increment after the enum changed, and nothing noticed.

**The precedent exists**: `crates/prikk-store/src/trust_gated_operations_binding_gate.rs` binds a code
enum to a markdown section in `trust-threat-model.md`, scoped by HTML comments so the gate reads only
the intended block.

**My lean is that this is worth binding** — the table is exactly the kind of inventory RFC 118 exists
to stop transcribing, and the mechanism is already built and proven twice.

**But adjudicate it, and a reasoned "no" is acceptable.** The table carries a **Role** column that is
authored prose, not derivable — so a gate could only bind the *code/name pairs*, not the whole row.
**If that partial binding is not worth its own machinery, say so.**

## 5. Out of scope

- **Any code change**, unless §4 concludes a gate is worth building.
- **`trust-threat-model.md`** (§3).
- **The `### Sync` heading** in `ROADMAP.md`, still reported and unfixed. **Leave it.**
- **The rest of `data-model-lifecycle.md`.** If you find other staleness, **report it, do not fix it** —
  this page is long and I have not swept it.

## 6. Controls

1. **No live document describes `ProjectGenesis` as reserved, live, or as anchoring a `project_id`** —
   show it mechanically across `docs/src`, `README.md`, `ROADMAP.md`, and `MILESTONES.md`.
2. **The table matches `ObjectType::ALL`** after the edit — ten rows, same codes and names. Show the
   comparison, do not assert it.
3. **`mdbook build` clean**, and the anchor a reader would follow from the table still resolves.
4. **Full gate set green.** If §4 concludes no gate, **the test count must not move**; if it concludes
   a gate, say exactly how many tests it added.

## 7. What to report

1. **The before/after of the table row and lines 298-300.**
2. **Your §4 adjudication**, with reasoning either way.
3. **Anything else stale you found on this page** — reported, not fixed (§5).
4. All four controls (§6), quoted.
5. **Full gate set against the exact commit, after the last edit**, including `mdbook build`.
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: any live code path still constructs or admits `0x0A` —
**that would contradict the settlement and outranks this increment.**
