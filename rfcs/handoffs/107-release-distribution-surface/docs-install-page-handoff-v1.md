# Docs — add the missing install page

**Base:** current `main` (`01fd32f`). **Under `003-landing-work-on-main.md`.**
**Origin:** the owner asked whether setup friction makes new users feel passive. Checking found the
docs book has **no install page at all**.

---

## 1. The gap

`docs/src/SUMMARY.md` has **no** install, getting-started, or quickstart entry. `docs/src/index.md`
mentions **neither** `install`, `binstall`, nor `download`. The Guide's first entry is **Security and
Signing Setup**, which assumes prikk is already on the machine.

**So a reader who lands on the documentation site — where "how do I start" traffic goes — finds
nothing about getting the binary.** Installation exists only in the root `README.md`.

## 2. Do not restate the README

**The README's install section is already good**: `cargo binstall prikk`, direct download with
`.sha256` and `.build-info.txt`, `cargo install prikk`, and an honest release-authority caveat.

**A page that repeats it is the defect just ruled against for the crate READMEs** — one text in two
files with nothing binding them. **If a sentence would be byte-identical to the README's, do not write
it.** Link instead.

**The split:** README is the thirty-second version and stays canonical for *what to run*. **The guide
page owns the operational walkthrough** — the parts a reader gets stuck on, which the README has no
room for.

## 3. What the page must actually contain

The README says *"verify the attached `.sha256` checksum"* and stops. **That sentence is where a new
user stalls.** Cover, concretely:

1. **Verifying the checksum** — the real command, per platform (`sha256sum -c`, and the PowerShell
   equivalent). A reader must be able to copy it.
2. **Where to put the binary, and how to get it on `PATH`** — undocumented anywhere today.
3. **Confirming it worked** — `prikk --version`, and what correct output looks like.
4. **What to do next** — link straight to **Security and Signing Setup**, which is the real first
   step and currently has nothing pointing at it.
5. **Uninstalling** — **documented nowhere today.** prikk places nothing outside its own binary and
   the `.prikk` directory inside a repository; **say that plainly**, because a user who cannot find
   out how to remove a tool is exactly the passivity the owner asked about.

**Keep it short and actionable.** This page exists to remove friction; a wall of prose adds it.

**The release-authority caveat — link, do not restate.** A reader following these steps must know a
checksum proves transport integrity and not authority of origin, but that sentence already exists in
the README and the release-compatibility reference. **Point at it.**

## 4. Wire it into the book

**First Guide entry, before Security and Signing Setup**, in `docs/src/SUMMARY.md`.

**Also link it from `docs/src/index.md`**, whose "start with" list currently sends a newcomer to the
Data Model reference — correct for an architecture reader, wrong for someone trying to run the tool.

## 5. Out of scope

- **Rewriting the root `README.md`'s install section.** It is fine. **One link into the new page is
  acceptable; a rewrite is not.**
- **Any installer script, package-manager channel, or `prikk install` subcommand.** Separately ruled
  on; not this increment.
- **`docs/src/index.md`'s own framing** — it still calls the documentation *"intentionally short in
  the early implementation phase."* **That is arguably stale at `0.25.0` with a full guide. Report it;
  do not fix it here.**
- Platform-support content, which has its own reference page — **link, do not duplicate.**

## 6. Controls

1. **Every command on the page runs as written**, on this machine, for the paths that can be exercised
   here. **Quote the real output for `prikk --version` and the checksum verification.** Do not
   transcribe expected output from memory.
2. **No sentence duplicates the README** — show it mechanically, the same way the crate-README
   increment did.
3. **`mdbook build` clean**, and the page is reachable from both `SUMMARY.md` and `index.md`.
4. **Full gate set green**, count moved and why — **a docs-only change should not move it.**

**Quote every failure.** If a command in the page does not work as written, **that is the finding** —
report it rather than adjusting the page to match a broken step.

## 7. What to report

1. **The page, in full.**
2. **Control 1's real output**, quoted.
3. **Whether `index.md`'s "intentionally short" framing is stale** (§5), reported not fixed.
4. All four controls (§6), quoted.
5. **Full gate set against the exact commit, after the last edit**, including `mdbook build`.
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: a documented install path does not actually work — **that is a
release defect, not a docs defect, and it outranks this increment**; or the Windows verification
command cannot be checked from here, in which case **say it is unverified rather than presenting it as
tested.**
