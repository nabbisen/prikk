# `commit` must say that it is discarding your message — implementation handoff

**Authority:** `rfcs/proposed/123-commit-message-and-authorship-metadata.md` §4 Option C-revised,
**ruled by the project owner 2026-09-01** and ruled to be taken *"immediately and independently"* of
the schema-3 work.
**Base:** current `main` (`a660586`). **Under `003-landing-work-on-main.md`.**

**This is one output line. It is in this handoff on its own because it has been outstanding since
the ruling, and every commit made in the meantime silently discards a message the user was required
to type.**

---

## 1. The defect it mitigates — not the one it fixes

`prikk commit -m <message>` **requires** a message, validates it non-empty, and drops it. It arrives
at `node_authoring.rs:182` bound as `_message` and goes no further. `prikk log` shows block and ref
metadata and no message at all.

**This increment does not fix that.** Fixing it is Option A — an identity-bearing `message` field on
`PatchPayload` at `Patch` schema 3 — which the owner also ruled and which is weeks of format work.
**This increment stops the silence while that is built**, because silently discarding required user
input is the worst of the available behaviours, and it is what ships today.

## 2. What to build

**One `note:` line in `commit`'s output.** `main.rs:139-149` already prints
`recorded worktree patch in active WAL`, the report fields, the per-change lines, and then a trailing
`note:`.

**The register is established — match it, do not invent one.** `commit` already ends with:

> `note: multi-operation text diff minimization, patch algebra, rename detection, and audit plugins remain later increments`

and the CLI speaks this way in five other places (`output.rs:48,51,60,78,120`). **No RFC numbers** —
none of the existing notes carry one.

**Candidate wording, to refine rather than adopt verbatim:**

> `note: the message is validated but not yet stored -- it does not appear in `prikk log`; persisting it is a later increment`

**The requirement the wording must meet:** it must say **what the user loses**, not merely that
something is unimplemented. *"Messages are not yet stored"* alone leaves a reader assuming it shows
up somewhere; naming `prikk log` is what makes the consequence concrete. Get that property and the
exact phrasing is yours.

**Print it on every commit.** There is no state to remember having said it once, and the existing
notes print every time too.

## 3. What this must not do

- **Do not make `-m` optional.** The owner ruled against it explicitly: it is a user-facing interface
  change that Option A would want to reverse, and it removes the prompt that makes a message field
  feel natural later.
- **Do not start storing the message** anywhere — not in the patch, not in a sidecar, not in the WAL.
  That is Option A and it needs the schema-3 design.
- **Do not touch `node_authoring.rs`.** `_message`'s underscore is correct until Option A lands;
  renaming it would only create the appearance of a change.
- **Do not add an author display name.** RFC 123 §5 defers it deliberately as a separate decision.

## 4. The one thing that makes this more than a one-liner

**Two documentation pages quote `commit`'s output**, and they trim it differently — check both and
say what you found:

- **`docs/src/guide/backup-restore.md`** marks its trims with a standalone `...`, including one
  already covering `commit`'s existing trailing note. A new note line should fall inside that
  existing marker — **verify it does rather than assuming.**
- **`docs/src/guide/tutorial.md`** ends its commit block at `  create-file readme.txt` with **no
  trailing `...`**, so neither the existing note nor `text edits: 0` is shown or marked.

**Both pages are anchor-tested** (`beginners_tutorial.rs`, `dc44_backup_restore_page.rs`) against the
real binary, so run those.

**If `tutorial.md`'s unmarked trim bothers you, report it — do not fix it here.** Marking it is the
same correction I required on `backup-restore.md` during DC-44, and it is a documentation-consistency
item, not this increment's. Naming it is right; folding it in is scope creep on a one-line change.

## 5. Controls

1. **The note shown in real `commit` output**, before and after, from the compiled binary.
2. **`prikk log` still shows no message** — quoted, so the note's claim is demonstrated true rather
   than asserted.
3. **`-m` is still required**: `prikk commit` with no `-m` still refuses, exit code shown.
4. **Both anchor tests green**, and your finding on §4's two pages stated as a result.

## 6. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit — **note that the set now
includes `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`,
added 2026-09-02** — with clippy as a single invocation per target and the exit code captured
explicitly, plus `mdbook build` if any doc page changes. Cross-target clippy judged from your own
diff.

**No CI control** — that is the architect's at push time.

One commit on `main`, local, **no push, no tag**.

## 7. Scope

RFC 123 does **not** close with this. Option A (the schema-3 `message` field) and §5 (the author
display name) both remain, and this note is what stands in the meantime — **its removal is part of
Option A's own increment**, not a separate cleanup to forget.
