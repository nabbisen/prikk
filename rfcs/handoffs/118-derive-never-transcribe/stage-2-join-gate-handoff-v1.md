# RFC 118 stage 2 — the join gate

**Base:** current `main` (`a27b531`). **Under `003-landing-work-on-main.md`.**
**RFC:** `rfcs/accepted/118-derive-never-transcribe.md` §8. **Stage 1 landed** the registry
(`crates/prikk-cli/src/commands.rs`).

**A new gate. No behaviour change, no documentation rewrite.**

---

## 1. Prerequisite 2's open sub-question — ruled

The prerequisite ruling left open how a checker reads the registry, naming three candidates and
warning that **`release-policy` parsing Rust source would be a second copy.**

**Ruling: none of the three. The join gate is a `#[test]` in `prikk-cli`, reading `COMMANDS` directly.**

**The precedent is this project's own.** Gate A — RFC 114's completeness guard — is a `#[test]`
colocated with the thing it guards, not a `release-policy` check. And tests here already read repo
files by walking from `CARGO_MANIFEST_DIR`: `format_stability_gate.rs:49`,
`dc55_identity_evidence.rs:149`, `prikk-object/src/vectors/snapshot.rs:25`.

**So there is no serialization boundary at all**: the registry is Rust, the gate is Rust, the documents
are text on disk. **No emitted inventory, no build artifact, no parser, no new command, no dependency.**

## 2. The two rules — RFC 118 §8

**(A) Every documented `prikk <command>` names a real registry entry.**
**(B) Every registry entry is explained somewhere.**

**(A) is the one that catches the defects this arc actually had** — `README.md`'s stale command list,
`main.rs`'s stale module inventory, prose naming commands that no longer exist. **Implement it first and
make it strict.**

**(B) is a coverage claim and needs an escape hatch** (§4).

## 3. Scope of documents — declare it, do not glob

**22 files under `docs/src/` mention `prikk <command>`, plus `README.md`.**

**Do not scan everything by wildcard.** A declared list is itself a claim, and it belongs under the same
discipline: **a file added to `docs/` should not silently escape the gate, and a declared file that is
deleted should fail loudly.** Assert the declared paths exist.

**Start with `README.md` and `docs/src/`.** **Exclude `rfcs/`** — RFCs are historical records and
legitimately name commands that were never built or have since gone.

## 4. Rule (B) needs a declared-undocumented list, and it has a precedent

Some commands are **deliberately** undocumented. **`doctor --repair-main-ref` is the live example**: it
is a permanently-refusing input, and I ruled it should *not* appear in `--help` precisely because
documenting it would imply a viable option.

**So (B) fails unless a command is either explained or declared undocumented with a reason** — exactly
`RFC114_ADMITTED_BUT_UNWRITTEN`'s shape next to Gate A's `frozen`. **Use that as the model, including
requiring a reason string.**

**This is the judgment/facts boundary from RFC 118 §9**: which commands deserve prose is judgment; that
the list is complete is a fact. **Gate the fact, declare the judgment.**

## 5. What "explained" means — decide and state it

**A mention is not an explanation**, but a full prose analysis is not testable. **Pick the weakest rule
that still has teeth, and say what you picked.** My suggestion, not binding: the command name appears
in a declared documentation file **outside** a bare command-listing block — so `README.md`'s
`Useful Commands` list alone does not count as explaining anything.

**If that proves unworkable, report what you chose instead and why.**

## 6. The controls

1. **Rule (A) fires**: add `prikk frobnicate` to a declared document, observe the failure, revert.
2. **Rule (B) fires**: remove a command's explanation (or add a new registry entry with none), observe
   the failure, revert.
3. **The declared-undocumented list is load-bearing**: remove `--repair-main-ref`'s entry (or whichever
   you declare) and confirm (B) then fails.
4. **The gate passes unmodified** on current `main`.

**Quote every failure.** A gate never observed failing is not a gate — **this project has now been
bitten by that twice on Gate A alone.**

**If (B) cannot be made to pass on today's documentation without declaring most commands undocumented,
stop and report.** That would mean the documentation gap is larger than a gate should paper over, and
**the finding is more valuable than the gate.**

## 7. Out of scope

- **Writing any missing documentation.** If (B) reveals gaps, **report them as a list** — that list is
  this stage's most valuable output.
- **Changing `commands.rs`, `help.rs`, or any command behaviour.**
- **`rfcs/`** (§3).
- **Enumerations beyond commands** — RFC 118 §10.3, still the owner's.

## 8. What to report

1. **Where the gate lives**, and confirmation it reads `COMMANDS` directly (§1).
2. **The declared document list** (§3), and how a new `docs/` file is prevented from escaping.
3. **Your definition of "explained"** (§5).
4. **The declared-undocumented list**, with each reason.
5. **All four controls, with quoted failures** (§6).
6. **Any documentation gap (B) revealed** — the list (§7).
7. **Full gate set against the exact commit, after the last edit.** Test counts rise by the gate's own
   tests; say by how much.
8. Anything here that was wrong, **including my 22-file count**.

**Stop and escalate, do not guess**, if: (B) requires declaring most commands undocumented (§6); the
declared-document list cannot be made to fail on a deleted file; or **reading `COMMANDS` from a test
turns out to need it made `pub`** in a way that widens the crate's API beyond what the gate needs —
**say so rather than widening it.**
