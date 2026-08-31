# DC-44 increment 4 — the page: what backup and restore prove, and what they do not

**Authority:** `rfcs/proposed/DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md`, design goal 6.
**Base:** `c135dd0` or later `main`. **Under `003-landing-work-on-main.md`** — commit locally on
`main`, do not push, do not tag.

**This closes DC-44.** It is a documentation increment with one currency obligation attached (§5).

---

## 1. Why this page has to exist, in the project's own words

**The product already tells users to restore from a backup it has never taught them to make.**

```
patch_replay.rs:298   "preserve the repository and restore from backup"
doctor.rs             "…or restore the repository from backup"
```

**And three reference pages still list "backup/restore tooling" as `Still deferred`** —
`durability-recovery.md:176`, `concurrency-locking.md:255`, `integrity-recovery.md:175`.

**So today a user meets remediation advice pointing at a capability the documentation says does not
exist.** That contradiction is what this increment resolves: the page states what the capability now
is, and §5 corrects the three claims that say it is absent.

## 2. What is actually true now — establish it yourself, do not take this list on trust

Three increments have shipped since DC-44 was written. **Read the code and the increments' own
reviews before writing a word of the page**, because a page that overstates this would be worse than
none:

- `bundle export --ref REF --output FILE [--force]` — **one ref's sealed closure**, written
  atomically, refusing to overwrite without `--force`.
- `bundle verify --input FILE` — **offline**, needs no repository, checks structure, framing, closure
  resolution, and manifest agreement against the exported RefState's own **signed** ref name.
- `bundle import --input FILE` — records material; **verification is a separate act**
  (`verify_repository` afterwards). The module's own words: *"Import records material; `verify`
  decides."*
- **`PBNDL003` manifest** — repository format, tool version, and an explicit statement that the scope
  is a single ref. `PBNDL001`/`PBNDL002` still import.

## 3. What the page must say plainly, and the two hardest items

**These are the ones a reader most needs and is least likely to work out.**

**3.1 — a bundle carries *sealed* history only.** `export_bundle` resolves the ref's tip block and
walks its ancestry. **Work that is committed but not yet sealed lives in the active WAL and is in no
block, so it is not in the bundle.** A user who commits, does not seal, and exports has backed up
none of that day's work. **Say so where they will meet it, not in a footnote.**

**3.2 — a bundle is one ref.** Backing up a repository with several branches means several bundles,
and nothing takes them for you. The manifest now states the limitation inside the artifact; the page
must state it too.

**Then the rest of the "does not prove" list**, each with its reason:

- **No signature is cryptographically verified offline.** A standalone bundle carries no trust
  material to check one against; `verify_repository` after import is what checks authorship.
- **Authorship is trust-on-first-use.** Transported author key material is supplied by the sender, so
  a verifying bundle proves the signature and key agree *with each other* — **"the same author as
  last time", not "who this author is."** `trust-threat-model.md` already states this; **link it,
  do not restate it in different words**, or the two will drift.
- **Maintainer trust policy is not in the bundle.** Establish what a restored repository does and
  does not have, and say it.
- **A checksum proves transport integrity, not authority of origin.** `release-signers.toml` is still
  empty; the installer already carries this sentence, and the page should not contradict it.

**And what it *does* prove**, stated as precisely: content addressing means corruption is detectable;
`bundle verify` answers "is this file intact and internally consistent" without a repository;
`verify_repository` after import is what turns a restored copy into a checked one.

## 4. Placement and shape

**A guide page, not a reference page** — this is a task a person performs. Propose where it sits in
`SUMMARY.md` and argue it; **near Sync is the obvious neighbourhood**, since both concern moving
history out of one repository.

**Show the actual commands**, in order, for both directions: make a backup, check it later without
restoring, restore it, confirm the restore. **Every command must be one the current binary accepts.**

**Consider anchoring it as the tutorial is anchored.** `beginners_tutorial.rs` binds a page's command
sequence to a test so a CLI change breaks the test rather than the reader. **Adjudicate whether this
page earns the same treatment** — it is a recovery procedure, which is a strong argument, but say
what you decide and why either way.

## 5. The currency obligation — fix the three stale claims

**`backup/restore tooling` is listed as `Still deferred` on three pages** (§1). That is no longer
accurate, and it is also **not fully delivered** — multi-ref export does not exist, and no restore
rehearsal has been performed.

**Correct all three to what is true**, neither "deferred" nor "done". **Do not simply delete the
phrase** — a reader of those lists needs to know where the boundary now falls, which is the same
discipline every status correction in this project has followed.

## 6. What must not change

- **No code.** If the page cannot be written truthfully without a behaviour change, **stop and
  report** — that is a finding, not something to paper over in prose.
- **No new claims.** Everything the page asserts must be traceable to shipped behaviour.
- **Do not restate `trust-threat-model.md`'s authorship limit in your own words.** Link it.

## 7. Controls

1. **Every command on the page runs**, in order, against the compiled binary. Quote the transcript.
2. **§3.1 demonstrated, not asserted** — commit without sealing, export, and show the bundle does not
   carry that work. **This is the claim most likely to be wrong if I have misread the walk, and the
   one a user is most likely to be hurt by.**
3. **§5's three corrections**, quoted before and after.
4. **`mdbook build` clean, `SUMMARY.md` updated, every internal link resolving in the built HTML** —
   verified against `docs/book/`, since mdbook does not check links.
5. **Full gate set against the exact final commit.**
6. **Per-job CI** if §4's anchoring adjudication adds a test; say which applies.

## 8. The report

To `.git-exclude/review-request/`. Include §4's placement and anchoring adjudications, §5's three
corrections, all six controls quoted, the full gate set, and **anything in this handoff that was
wrong** — especially §2's summary of current behaviour and §3.1's claim about unsealed work, both of
which I derived by reading `export_bundle`'s ancestry walk rather than by exporting a repository with
unsealed work in it. **Control 2 exists precisely to catch me being wrong about that.**
