# DC-44 increment 1 — verify an export offline, before restoring it

**Authority:** `rfcs/done/DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md`, design goal 2 (*"verify an
export offline before restore"*), selected by the owner 2026-08-31. **Base:** `026307c` or later
`main`. **Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This does not close DC-44.** §6 names what remains. Filed here because `DC-78-history-exchange` is
where the bundle format's own handoffs live.

---

## 1. The gap, measured

**Everything about a bundle is checked *during import*, and nothing before it.** `bundle.rs`'s own
doc states the design: *"Import records material; `verify` decides (D7)"* — the receiver's confidence
comes from running `verify_repository` **after** the objects are already written.

**So there is no way to answer "is this backup any good?" without restoring it.** For an exchange
artifact that is a reasonable design. **For a backup it is the wrong shape**: the moment you need a
backup is the moment you least want to find out by importing it somewhere.

**What already exists, and is good** — do not rebuild it: malformed bytes fail closed
(`import_of_malformed_bytes_fails_closed`), object-count and total-byte limits fire boundary-exactly
and **write nothing** when they refuse, re-import is idempotent, and key-material conflicts are
refused. **The import path is well covered. The gap is that none of it is reachable without a
repository and a write.**

## 2. What to build

**`prikk bundle verify --input <file>`** — reads a bundle file, reports whether it is
structurally sound and internally consistent, **writes nothing and needs no repository.**

**The decisive constraint:** it must **reuse `bundle.rs`'s existing decode path**, not re-implement
it. **Two decoders drift, and the drift is silent** — this project has found that failure repeatedly,
most recently where one shared classifier made parity a property rather than a decision. If reuse
needs a refactor to expose the decode without the write, **do the refactor**; do not copy the logic.

## 3. What you must adjudicate

**3.1 — what is actually checkable offline.** Work this out from the format rather than from this
list, which is my guess and may be wrong: magic and version (`PBNDL002`, with `PBNDL001` accepted as
import does), framing and declared counts against actual content, each object decoding, and **each
object's id recomputed from its own bytes and compared to the id it is stored under.** That last one
is the real integrity check and the reason this is worth building.

**3.2 — what is *not* checkable offline, stated as a limit.** AUTHOR signature verification against
trust material the file does not carry is the obvious one. **Say what the command does not prove, in
its own output or its docs** — a verifier that stays silent about its limits is how "the backup
verified" becomes a false belief. DC-44's own last design goal asks exactly this.

**3.3 — output shape.** Prose certainly. **Whether `--format json` belongs here is yours to argue** —
`verify` has one and CI gates use it, but scope creep is real. If you defer it, say so.

## 4. What must not change

- **No bundle format change.** No `PBNDL003`, no manifest, no new section. This increment reads the
  format as it stands — **and building the checker is how you learn what a manifest would need to
  add**, which is why it comes first.
- **No mutation.** `bundle verify` opens a file. It does not need, create, or touch a repository.
- **No change to `export_bundle`/`import_bundle` behaviour.** If verification needs something the
  decode path does not expose, expose it — do not alter what import does.

## 5. Controls

1. **A good bundle verifies**, produced by a real `bundle export`.
2. **The failure modes DC-44 names, each with an observed failure** — for those checkable offline:
   truncated/incomplete file, a corrupt object inside an otherwise well-formed bundle, wrong magic,
   and a count that disagrees with the content. **A corrupted object whose id no longer matches its
   bytes is the one that proves 3.1's integrity check works**; the others prove framing.
   **For any DC-44 failure mode you conclude is not offline-checkable, say which and why** — that is
   a finding, not an omission.
3. **`verify` and `import` agree — the control that matters most.** A bundle `bundle verify` accepts
   must import successfully; one it rejects must be refused by import too. **Demonstrate both
   directions.** If they can disagree, you have built the second decoder §2 forbids.
4. **Nothing is written.** Run it against a bundle with no repository present and show the working
   directory unchanged.
5. **Full gate set against the exact final commit**, plus `mdbook build` if you add documentation.
6. **Per-job CI** — a new CLI subcommand with tests. Say whether cross-target clippy applies.

## 6. What this leaves for later, so nobody reads it as closing DC-44

- **The self-describing manifest** (format, refs, objects, digests, tool version, exclusions) — a
  bundle format change, and therefore migration-covered under RFC 114's contract, which explicitly
  places `bundle` in the changeable-with-migration category.
- **Interrupted export and destination collision** — export-side failure modes this increment does
  not touch.
- **A migration rehearsal** and **the documentation page** stating what backup/restore proves and does
  not prove.

## 7. The report

To `.git-exclude/review-request/`. Include §3's three adjudications, all six controls quoted, the full
gate set, an explicit statement of which DC-44 failure modes you covered and which you judged not
offline-checkable, and **anything in this handoff that was wrong** — including §3.1's checkable list,
which I wrote from reading `bundle.rs`'s doc comments rather than its decoder.
