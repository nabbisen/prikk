# Data model and lifecycle reference catch-up: implementation handoff

**Base:** current `main` (`cbfdc6d`). **Under `003-landing-work-on-main.md`.**
**Origin:** owner questions, 2026-08-23 — whether the data model and its relations are documented in
enough detail, and whether the lifecycles are.

**Two files:** `docs/src/reference/data-model.md` (158 lines) and
`docs/src/reference/data-model-lifecycle.md` (168 lines).

**This is not a currency pass.** The four increments before it corrected sentences that had gone stale.
**Here the model itself was never written down** — RFC 115/116/117 shipped a new object type, two new
artifact formats, a new identity concept and a new namespace, and **none of them reached the reference.**
Expect to write sections, not edit them. **It is the largest documentation increment in this arc; if it
wants to be two, say so rather than thinning it.**

---

## 1. What is measurably absent

Counted across **both** files:

| Concept | Mentions | Status in code |
|---|---|---|
| `RecognitionClaim` | **0** | admitted object type, `format.rs:33` |
| `PEXCH` (exchange artifact) | **0** | `PEXCH002`, `artifact.rs:52` |
| `PSYNCSU1` / `PSYNCHV1` | **0** | shipped negotiation artifacts |
| `patch_set_digest` / `patch_count` | **0** | `TagPayload` fields 6 and 7 |
| received / `remotes/` namespace | **0** | where imported history lands |
| compaction | **0** | `prikk compact`; three functions in `compact.rs` |

## 2. Two errors, not omissions — fix these first

### 2.1 A false claim about Tag objects — `data-model.md:44-46`

> *"Tag and Attestation object types and directories are defined, but **current public command surfaces
> do not produce Tag or Attestation objects**."*

**False for Tag.** `prikk tag create` produces them, and so does `sync adopt-tag`.
**Still true for Attestation** — genuinely never constructed. **Correct the Tag half, keep the
Attestation half**, the same split-clause discipline as the README pass.

### 2.2 The file contradicts itself

`data-model.md:19-22` (`Core Caveats`) was updated to say `prikk sync`, tag travel and `prikk merge`
have shipped. **The object model section below it was not.** Staleness by partial update — a later
reader gets both answers from one file.

## 3. The concept the reference is missing most

**A tag names a patch set, not a block.** That is RFC 117's central identity ruling, and the reason
`TagPayload` carries `patch_set_digest` at all: **blocks diverge across repositories by design, so a
block id cannot be the stable cross-repository identity; the patch set can.**

It is load-bearing for how sync works, and it is **documented nowhere.** If only one thing from §1 gets
written properly, make it this.

## 4. Relations to document, since the question was specifically about relations

The older relations (patch→block, block→ref) are covered adequately. **The exchange-era relations are
absent entirely:**

- **claim → block**, and the claim's `patch_ids` (verbatim block order, D6) and `parent_block_ids` (N3)
- **tag → patch set digest → the patches that resolve it**, and why resolution is a *search*, not a lookup
- **received objects → `remotes/` → local refs**, and the rule that **import never advances a local ref**

## 5. Lifecycles — what exists, and the three gaps

`data-model-lifecycle.md` already covers, and covers reasonably: **content** (worktree → sealed),
**a node**, **a ref**, **block lineage**, **patch and operations**. **Do not rewrite these.**

### 5.1 Compaction is undocumented — the only operation that removes anything

`prikk compact --pointer-index|--received-index|--trust-policy|--all` is shipped, with
`compact_ref_pointer_index`, `compact_received_index` and `compact_trust_policy` in `compact.rs`.
**Zero mentions in either file.**

**This is the sharpest lifecycle gap.** Everything else in prikk is append-only and immutable; compaction
is where append-only structures get rewritten. A reader of the lifecycle reference currently cannot
learn that anything is ever reclaimed. **Document what it rewrites, what it preserves, and what
guarantees hold across it.**

### 5.2 No repository lifecycle

There is no *"Lifecycle: a repository"* section. `prikk init` creates one; nothing documents its stages,
the format-6 boundary, or what `doctor`/`unlock` mean in lifecycle terms.

**Write what is true today. Do not invent stages.**

### 5.3 `ProjectGenesis` — a reserved type code with nothing behind it

`ObjectType::ProjectGenesis = 0x0A` exists (`id.rs:38`), names itself `"project-genesis"`, and has a
test vector — but **has no payload module**, and `validate_format2_schema` **refuses it outright**
(*"is not authorized in a format-2 identity position"*).

**Add it to §6's list of things the model deliberately does not record.** **Do not describe a project
lifecycle** — there is not one, and inventing it is exactly the failure §2.1 and the `0.23.0` tag advice
came from.

## 6. Extend the honest-gaps section, do not shorten it

`data-model-lifecycle.md:160` — *"What the model does not currently record... Stated here so it is not
inferred from silence"* — is **the best practice in either file. Keep it and extend it.** Its four
entries (inert `parent_patch_ids`, attestation clearing, unauthored `RenamePath`/`CreateSymlink`, manual
merge-base) are **all still true — verify each, do not assume.**

Candidates to add: `ProjectGenesis` (§5.3); `Attestation` never constructed; tag **deletion and
movement do not travel**; there is **no ref deletion at all** (confirmed while reviewing `0.23.0` — no
production removal path exists, which is why a `0.22.1` tag cannot be cleared).

## 7. Out of scope

- **Every file except those two.** The guide pages (`guide/sync.md`, `guide/merge.md`) are their own
  audience; **report anything you find contradicting your work.**
- **No behaviour change, no code.** If documenting something reveals a code defect, **report it** — do
  not fix it here.
- **`MILESTONES.md`, the badge, `ROADMAP.md`.** Mine or the owner's.

## 8. What to report

1. **Each item in §1**, where you documented it, and the authority you derived it from — **the code, not
   an RFC's prose.** An RFC describes what was intended; the reference must describe what shipped.
2. **§2.1's correction**, and confirmation the Attestation half is still true.
3. **Your verdict on each of §6's four existing entries** — still true, or not.
4. **Whether this should have been two increments** (§ preamble). An honest "it was too big" is useful.
5. The **full gate set against the exact commit, after the last edit.**
6. Test counts — **expected unchanged**.
7. Anything here that was wrong. **Every handoff I have written in this arc has contained at least one
   error — a miscount, a mis-stated base, or advice that could not be executed. Assume this one does
   too.** In particular **verify §1's zero-counts yourself**; they are my greps, and a grep that misses a
   synonym reports a gap that is not there.

**Stop and escalate, do not guess**, if: documenting something requires stating a guarantee no code or
ruling establishes; **§5.2 or §5.3 tempts you to describe a project/repository lifecycle richer than what
exists**; or you find a *fifth* undocumented shipped concept large enough to need its own increment.

**One open question I am NOT asking you to answer**, recorded so it is not lost: the owner asked about a
**"workspace"** concept. **There is no such concept in prikk** — zero mentions in either file, and none
in the code; prikk has a **worktree**. Whether a workspace is wanted as a future concept is the owner's
question to settle, not something to document into existence.
