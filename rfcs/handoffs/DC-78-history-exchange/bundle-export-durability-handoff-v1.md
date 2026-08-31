# DC-44 increment 2 — interrupted export and destination collision

**Authority:** `rfcs/proposed/DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md`, design goal 5's
*"interrupted export … destination collision"*. **Base:** `d7c180c` or later `main`.
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**These are not missing tests. They are two live defects that compound**, and §1 is the whole reason
this increment exists.

---

## 1. What is wrong today, measured

`crates/prikk-cli/src/bundle.rs:47`:

```rust
std::fs::write(&parsed.output, &bytes)
```

**`std::fs::write` creates-or-truncates.** Two consequences, and they are worse together than apart:

**Destination collision — an existing backup is destroyed silently.** `bundle export --output
backup.bundle` over a path already holding yesterday's backup truncates it with no prompt, no
`--force`, and no message. **Nothing in the command tells you it just overwrote something.**

**Interrupted export — the destination is left invalid.** The truncate happens first and the write is
not atomic, so a crash, a full disk, or a killed process leaves a **partial file at the destination
path**. The offline verifier landed in increment 1 will correctly refuse it — but that is detection
after the fact.

**Together:** an export that fails partway **destroys the previous backup and replaces it with a file
that is not a backup.** That is the failure this is worth fixing for, and it is reachable with no
unusual conditions — a full disk is enough.

**The irony is worth naming, because it explains how this survived.** The repository's own writes go
through an elaborate anchored durability contract — `openat`-scoped, fsync'd, atomically replaced,
reviewed per platform. **The file that backs the repository up is written with a bare `fs::write`.**
The careful machinery guards the original; nothing guards the copy.

## 2. The complication you must design around, not ignore

**`write_file_atomically` exists (`fsutil/anchored.rs:86`) and does not apply here.** It is
*anchored*: it takes a `MutationRoot` and a repository-relative path, because the whole anchored
design exists to confine writes inside the repository. **A bundle destination is deliberately outside
it** — you back up somewhere else, that is the point.

**So this needs an atomic write to an arbitrary user-supplied path, which is new territory for this
codebase.** Do not force the anchored primitive to do a job it was designed to refuse, and do not
weaken it to fit.

## 3. What you must adjudicate

**3.1 — the overwrite policy.** Refuse when the destination exists, unless an explicit flag says
otherwise? Overwrite but say so? **The criterion is that destroying a backup must never be silent**,
balanced against not breaking a script that already exports to a fixed path each night. **If you add
a flag, name it and say what the default is and why.**

**3.2 — the atomic mechanism.** Temp file beside the destination, then rename, is the conventional
shape — same filesystem, so the rename is atomic. **Adjudicate the details and say what you chose:**
where the temp file lives, what happens to it when the write fails, whether you fsync before rename,
and whether the destination's directory is fsync'd after. **Say plainly what your choice does and does
not guarantee** — this project states durability limits rather than implying them.

**3.3 — where the code lives.** The write is currently in `prikk-cli`. Whether an atomic
arbitrary-path write belongs there, in `prikk-store`'s `fsutil` beside the anchored one, or somewhere
else is yours — **argue it, and be explicit that it is not the anchored primitive and must not be
mistaken for it.**

## 4. What must not change

- **No bundle format change.** Not this increment.
- **`export_bundle`'s own output** — the bytes it returns are unchanged; this is about how they reach
  the disk.
- **The anchored durability contract.** Do not extend, relax, or reuse it for non-repository paths.
- **`bundle import` and `bundle verify`.** Untouched.

## 5. Controls

1. **A failed write leaves the previous file intact.** The decisive control: with an existing backup
   at the destination, make the write fail, and show **the original still present and still passing
   `bundle verify`**. How you induce the failure is yours — an unwritable directory, a failpoint, a
   temp path that cannot be created — but **the assertion is on the destination's contents, not on
   the error message.**
2. **No partial file is left anywhere** after a failed export — neither at the destination nor as an
   abandoned temp file. If a temp file can survive a failure, say so and name where.
3. **Destination collision behaves as §3.1 decided**, demonstrated both ways: the protected case
   refuses or warns, and the permitted case succeeds.
4. **A successful export is byte-identical to what `export_bundle` returned**, and passes
   `bundle verify` — the new write path must not alter content.
5. **Existing bundle tests pass unmodified.** If one must change, behaviour moved somewhere §4 says it
   must not — **stop and report.**
6. **Full gate set against the exact final commit.**
7. **Per-job CI.** This is filesystem behaviour — rename semantics and directory handling differ across
   platforms, and Windows in particular has its own rules about replacing an open or existing file.
   **Say whether cross-target clippy applies, re-derived for this diff.**

## 6. What remains of DC-44 after this

**The self-describing manifest** (a bundle format change, migration-covered under RFC 114) and **the
documentation page** stating what backup and restore prove and do not prove.

**The migration rehearsal is already answered and is not outstanding** — DC-44's own status update
records that design goal 4 offered *exercise migration or explicitly supersede it*, and RFC 114 took
the second. **I listed it as remaining in increment 1's handoff; that was my error, corrected here.**

## 7. The report

To `.git-exclude/review-request/`. Include §3's three adjudications with reasoning, all seven controls
quoted, the full gate set, and **anything in this handoff that was wrong** — including my reading of
`fs::write`'s truncate-first behaviour, which you should confirm against the destination's actual
state after an induced failure rather than from the documentation.
