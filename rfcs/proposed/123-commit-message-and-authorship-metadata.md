# RFC 123 — The commit message is validated and then discarded

**Status.** **Proposed. This RFC records a decision the owner must make; it deliberately does not
make it.** Raised as **High** by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-1a.md` §3); reproduced independently at `3a8d730`
(`.git-exclude/reviewed/external-audit-20260831-review-v1.md` §1.2).

**Tracks.** What a `Patch` records about *why* it exists. This is a format decision, not a CLI one.

---

## 1. The defect

`prikk commit -m <message>` **requires** a message, validates it non-empty, and then drops it. It
arrives at `node_authoring.rs:183` bound as `_message` and goes no further. Reproduced:

```
$ prikk commit -m "a message that should be findable later"
$ prikk seal --allow-no-audit
$ prikk log
block 7dbf2d90…
  ref-state: 7960ea42…   update-seq: 1   kind: Root
  parents: 0   patches: 1   rollback-patches: 0
```

No message. No author name. Nothing that answers *what was this change?*

**Silently discarding required user input is the worst of the available behaviours** — worse than
not asking for it, and worse than storing it. A user who types a careful message has every reason to
believe it was kept.

## 2. Why this is disproportionately serious for this project

This product's claim is that **the repository is the evidence**. Its verification story is
exceptional: 14 stages, identity recomputed on every read, publication trust three-valued, merges
proven confluent by replay rather than asserted. And the history it so carefully protects cannot
answer the first question anyone asks of a history.

The audit puts it exactly right: a VCS whose history cannot answer "what was this change?" fails its
own explainability goal.

## 3. What the format already tells us

Three facts constrain the design, and all three are load-bearing:

**3.1 `Patch` already carries an identity-bearing advisory field, and nothing sets it.**
`PatchPayload.intent: Option<Intent>` (`payload/patch.rs:67`) is encoded into the canonical payload
at field 3 (`:104-105`), so it is inside object identity. `Intent` is a five-variant enum —
`Feature`/`Fix`/`Refactor`/`Docs`/`Test` (`payload/common.rs:24-34`) — **not** a message slot, and
`node_authoring.rs:568` writes `None` at every construction site. The design anticipated per-patch
advisory metadata; it did not anticipate free text.

**3.2 `Tag` already persists user text inside an identity-bearing object.**
`TagPayload.message: Option<String>` (`payload/tag.rs:38-39`), encoded at field 3. So there is direct
in-format precedent for optional user-authored text under a signature — the question is not *whether*
this project stores such text, but why `Patch` does not.

**3.3 A date cannot be stored, and this is not an oversight.**
`created_at` is pinned to `0` at both signing sites (`author_signing.rs:52`,
`maintainer_signing.rs:38`) and non-zero values are refused at publication
(`refs/publication.rs:212-214,276`). This is what makes object ids reproducible, and CI *proves* it
by mutating a repository independently on Windows and Linux and diffing the resulting object-id
lists. **Any design that puts a timestamp inside the identity surface destroys that property.** The
audit lists "message, author display name, or date" together as missing metadata; the date is the one
item on that list that must stay missing, and this RFC states so before anyone treats the trio as one
job.

## 4. The options

### Option A — identity-bearing `message` field on `Patch`, at schema 3

A new optional field, mirroring `TagPayload.message` exactly.

- **For:** the message becomes signed evidence, immutable, transported by bundle and sync for free,
  and verified by every existing check with no new path. It is what a reader of this codebase would
  expect given §3.2. Schema evolution is a solved problem here — the admitted-schema table, the
  retirement precedent (`PATCH_PARENT_IDS_RETIRED_SCHEMA`), and format-refusal tests all exist.
- **Against:** every existing patch's id was computed without the field, so old and new patches
  differ in shape — handled by schema versioning, but it is a real bump with a real compatibility
  statement to write.
- **The permanence objection does not survive examination**, and it was the strongest thing on this
  side. A message under Option A is permanent and unrewritable — but so is every path, every blob,
  and every `old_span_text` preimage already in the patch. **A secret typed into a file is exactly as
  permanent as a secret typed into a message**, and this product's answer to that is that the history
  is append-only and that is the point. Treating the message as needing an escape hatch the file
  content does not get would be incoherent, and would quietly imply a rewrite capability that does not
  and will not exist.

### Option B — non-identity sidecar object keyed by patch id

A separate object type carrying `patch_id → message`, outside the `Patch` preimage.

- **For:** messages become correctable and omittable; a repository can drop them without touching
  history identity; no `Patch` schema bump.
- **Against:** the message is then **not evidence** — unsigned, or signed separately, and detachable
  from the thing it describes. It introduces the first "true but unverified" surface into a product
  whose distinguishing property is that it has none. It also needs its own transport story in bundle
  and sync, its own verify stage, and its own answer for what a missing sidecar means — so it is
  **more** work than Option A, not less.
- **Its own headline advantage is its worst property.** A correctable message means a message read out
  of history is not evidence of what the author said; it is evidence of what someone last said.
  **Mutable annotations bolted onto immutable history is the one combination this project should never
  ship**, because it makes a message that is *wrong* possible where today only *absent* is possible.

### Option C — make `-m` optional and say plainly that messages are not stored

- **For:** honest immediately, costs nothing, and stops the active harm (users typing messages that
  vanish) today rather than at the next schema bump.
- **Against:** it is not a solution, and it removes the prompt that would make the eventual message
  field feel natural.

### Option C, revised — say it in the output, do not weaken the interface

**The first draft of this RFC recommended making `-m` optional. That was wrong, and a better interim
already exists in this CLI's own idiom.** Making a required flag optional is a user-facing interface
change that Option A would then want to reverse, and it removes the prompt that makes a message field
feel natural later.

**Instead: keep `-m` required and print a `note:` line saying the message is not yet stored.** The CLI
already speaks this way about every unimplemented area — `output.rs:48,51,60,78,120`, and `commit`'s own
output already carries *"note: multi-operation text diff minimization, patch algebra, rename detection,
and audit plugins remain later increments"*. One more clause in that register is honest immediately,
costs one line, churns no interface, and is exactly the house style.

**C-revised is compatible with either A or B and should be taken immediately regardless of the ruling**,
because A and B are both weeks of format work while users are losing messages today.

## 5. The separate question of an author display name

The audit groups it with the message. It is a different decision. The AUTHOR key id already
identifies the author cryptographically; a display name is **unverified text asserting an identity**,
which is a new category for this project and interacts with the trust/threat model (`trust-threat-model.md`
already draws a hard line between continuity and identity). **Recommendation: rule on it separately
and later.** Nothing in the message decision depends on it.

## 6. The ruling this RFC needs

1. **A or B:** is the message evidence (inside identity) or annotation (outside it)?
2. **Take C-revised now, or wait?**
3. **Author display name:** defer, or scope it with the message?

**The architect's recommendation: C-revised immediately, then A.** Three arguments converge and none
of them is about effort:

- **§3.2.** This project already stores optional user text inside a signed, identity-bearing object —
  `TagPayload.message`. No principle distinguishes a tag's message from a patch's, and shipping one
  as evidence and the other as annotation would be a distinction the codebase cannot justify.
- **B's advantage is a liability here** (§4, Option B). Correctable annotations on immutable history
  is the one combination that lets a message be wrong rather than merely absent.
- **B is also the more expensive option**, once its transport, verification, and missing-sidecar
  semantics are counted — so the usual reason to prefer a sidecar does not apply.

**And the format machinery is why this is cheap for prikk specifically**: the admitted-schema table,
the `PATCH_PARENT_IDS_RETIRED_SCHEMA` retirement precedent, and format-refusal tests against real
committed fixtures all exist. A schema-3 `Patch` field is a well-trodden path here in a way it would
not be in most projects.

## 7. Non-goals

No `blame`. No `show`. No commit-message templates, trailers, or conventions. No history rewriting to
attach messages to patches already sealed — that is impossible here by design and must not be
implied to users as a future.
