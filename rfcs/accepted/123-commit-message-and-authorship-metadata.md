# RFC 123 — The commit message is validated and then discarded

**Status.** **RULED by the project owner 2026-09-01: Option A — the message is evidence.** A new
optional `message` field on `PatchPayload`, inside object identity, at ~~`Patch` schema 3~~ **the next
free `Patch` schema, now 4**, mirroring `TagPayload.message`.

**Folder corrected 2026-09-05: `proposed/` → `accepted/`.** RFC-000's 5-folder variant defines
`proposed/` as *"open for review… implementer should not yet start work"* and `accepted/` as *"review
complete; implementer may start"*. §8's design is settled and a handoff is issued, so this belonged in
`accepted/` from the moment that handoff was written. Caught by the project owner; the architect had
left every owner-accepted RFC in `proposed/` and marked it accepted in the status text instead — the
exact folder-versus-status inconsistency `rfcs/README.md` warns against.

**Schema-number correction, 2026-09-04.** This ruling named schema 3 on 2026-09-01. **RFC 134 §8 then
minted `PATCH_TEXT_SPAN_V2_SCHEMA = 3` for content-unique span identity, on 2026-09-04, without the
architect checking whether an accepted ruling had already claimed that number** — the architect's
error, caught while surveying open work. **Nothing is broken**: schema 3 is span identity, correctly
implemented, and this RFC is unimplemented. **The number here moves to 4; the ruling's substance is
untouched.**

**The lesson, recorded where the next schema mint will meet it:** a schema number is a shared, ordered
namespace across every open RFC. **Before minting one, check every ruled-but-unimplemented RFC for a
claim on it** — `admitted_schemas` shows what is *taken*, never what is *promised*. **Option C-revised is taken immediately and independently**: `-m` stays required
and `commit` prints a `note:` line saying the message is not yet stored, in the CLI's existing idiom
for unimplemented areas. **The author display name stays deferred** (§5) and is not scoped with this.

**Those four open items are now designed — §8, written 2026-09-05.** Length bound: **none, and
deliberately** (§8.3). Encoding position: **tag 6, optional, schema 4** (§8.1), *optional* rather than
required because RFC 113's import must be able to represent a Git commit that genuinely had no message
(§8.2). `verify` treatment: **nothing new, stated as a negative so a redundant check is not added**
(§8.5). Compatibility: schema 1/2/3 patches keep working forever, but **the decoder refuses unknown
tags, so this is a second one-way break on every commit** — the same shape as 0.31.0's, and the
expensive part of the change (§8.6).

**§8.8's release grouping was AUTHORIZED by the project owner 2026-09-05.** Read as: **schema 4 ships
alone** — nothing is batched with it, because no other `Patch`-shape change is known. If one is found
before this lands, that is a stop-and-report, not a silent addition to the bump.

**Handoff issued:** `rfcs/handoffs/123-commit-message-and-authorship-metadata/message-field-schema-4-handoff-v1.md`.

Raised as **High** by the external architecture audit of 2026-08-31
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

### Option A — identity-bearing `message` field on `Patch`, at the next free schema (4 — see the Status block's 2026-09-04 correction)

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
committed fixtures all exist. A new-schema `Patch` field is a well-trodden path here in a way it would
not be in most projects.

## 7. Non-goals

No `blame`. No `show`. No commit-message templates, trailers, or conventions. No history rewriting to
attach messages to patches already sealed — that is impossible here by design and must not be
implied to users as a future.

## 8. The design — the four items §1's ruling left open, answered 2026-09-05

**Written by the architect on the owner's instruction after the 0.31.1 cut.** The ruling (Option A,
message-as-evidence) is settled input; nothing here reopens it. **Author-review independence: the
architect wrote this and is its only reviewer**, the standing gap on every architect-authored design
here, compensated at implementation review.

### 8.1 Shape and encoding position

**`message: Option<String>` on `PatchPayload`, canonical tag 6, `WireType::String`, emitted only when
`Some` and only at `Patch` schema 4 and above.**

`PatchPayload`'s tags today: 1 `operations`, **2 retired** (`parent_patch_ids`, never reusable), 3
`intent`, 4 `preconditions`, 5 `purpose` (`payload/patch.rs:111-126`). **6 is the next free tag** and
the writer emits in ascending tag order, so the field lands after `purpose` with no existing field's
bytes moving.

This mirrors `TagPayload.message` (tag 3, `Option<String>`, emitted only when `Some`) exactly, which
§3.2 named as the precedent.

### 8.2 Optional, not required — and the reason is RFC 113

The obvious simplification is to make the field **required** at schema 4: `-m` is already mandatory at
every construction site, so every patch this project authors would carry one, and "absent" would never
occur.

**Refused, on one future the project has already committed to.** RFC 113 is history import from Git,
Subversion and CVS. **Git permits an empty commit message.** A required field forces an imported
commit that genuinely had none to carry a fabricated one — a lie inside a signed object, in a system
whose whole argument is that history is evidence. Optional costs one `Option` and keeps "this commit
had no message" expressible.

This is the same shape as RFC 134's watch: a decision that looks free today is priced by a direction
already accepted.

### 8.3 No length bound — deliberately, and this must not be "fixed" later

**The format imposes no maximum message length.** Not an oversight:

- **The same object already carries two unbounded user-controlled byte fields.** `EditText` has
  `replacement_text: Vec<u8>` and `old_span_text: Vec<u8>` (`payload/patch/operations.rs:179,185`),
  neither bounded. Bounding the one *new* text field while leaving the two existing ones open would
  protect nothing and be incoherent.
- **Transport is already bounded** where untrusted input arrives: `DEFAULT_BUNDLE_MAX_TOTAL_BYTES`
  (256 MiB) and `DEFAULT_BUNDLE_MAX_OBJECT_COUNT` (100,000), with the exchange and have-list
  equivalents beside them. A message cannot exceed the artifact carrying it.
- **A bound inside the identity surface is permanent.** Picking 8 KiB and later wanting more is a new
  schema, because the bound would be part of what schema 4 means. The cheap-looking safety measure is
  the expensive-to-reverse one.
- `TagPayload.message` has no bound either. Adding one here and not there would split the precedent
  §3.2 rests on.

### 8.4 Empty is refused at both ends, so "absent" and "empty" never both mean "no message"

`PatchPayload::validate()` **must reject `Some("")`** — enforced on decode as well as encode, since
`validate()` runs in `encode_canonical` and decode goes through the same type. A hostile or merely
wrong object carrying an empty message is refused rather than admitted as an odd-but-legal patch.

**The format rejects length-zero only; the CLI keeps rejecting whitespace-only.** `args.rs:463`
already refuses `-m` that is empty after `trim()`, and `tag.rs:222-225` does the same for tags. That
split is deliberate: a format rule is permanent and should be the simplest thing that removes the
ambiguity, while `trim()`'s Unicode White_Space semantics are an interface-level nicety that may be
tuned without touching object identity.

### 8.5 `verify` gains nothing — stated as a negative because someone will otherwise add a check

**No new verification path, no new check, no new report line.** The message is inside the id preimage,
so:

- **Tampering is already caught** by the existing object-id check — altering one byte of a message
  changes the patch id, exactly as altering an operation does.
- **Malformedness is already caught** at decode, because `validate()` (§8.4) runs there.

This is Option A's stated advantage made concrete. **Adding a message-specific check to `verify` would
be redundant machinery that reads as extra assurance and provides none** — the failure it would look
for cannot reach it.

### 8.6 Compatibility — and this is a second one-way break, on every commit

`admitted_schemas(ObjectType::Patch)` becomes `[1, 2, 3, 4]` (`prikk-store/src/format.rs:40-44`).
Schema 1/2/3 patches carry no message and never will; they are not *missing* one — the concept did
not exist when they were written, and RFC 114 guarantees they stay readable forever, unchanged.

**The reverse direction breaks, and it is not avoidable.** `PatchPayloadFieldCursor` **refuses unknown
tags** (`payload/patch.rs:178-183`: `"unknown PatchPayload field tag: {other}"`), so there is no
skip-unknown path that would let an older reader tolerate tag 6. Combined with `-m` being mandatory,
**every commit authored by the release that mints schema 4 is unreadable by every earlier release** —
the same shape as 0.31.0's `Patch` schema 3 break, and it must be announced the same way, in the same
words, leading the release notes.

**The consequence for scheduling, which is the expensive part of this RFC:** the field is cheap and
the schema bump is not. **If any other `Patch`-shape change is foreseeable, it should ride schema 4
rather than mint schema 5.** None is known today; whoever finds one before this lands should say so
rather than take a second break. Check every ruled-but-unimplemented RFC before minting — the lesson
this RFC's own Status block already records after the schema-3 collision.

### 8.7 What the increment touches

| Site | Change |
|---|---|
| `payload/patch.rs` | the field, tag 6, `validate()`'s empty rejection, a `PATCH_MESSAGE_SCHEMA = 4` constant with a doc comment in the house style of the two beside it |
| `prikk-store/src/format.rs` | `admitted_schemas(Patch)` gains 4 |
| `worktree_patch/node_authoring.rs:550` | `prikk commit`'s construction site — carry the message through |
| `patch_inverse.rs:141` | the inverse/rollback-draft construction site — the same |
| `prikk-object/src/vectors.rs`, `vectors/hard.rs` | **a new schema-4 conformance vector; every existing vector's bytes must not move** |
| `prikk-cli` | `prikk log` surfaces the message; **the `note:` interim line from Option C-revised is removed in the same commit** — it says the message is not stored, and it would become false the moment this lands |
| `CHANGELOG.md` | the one-way break, led with, per §8.6 |

### 8.8 What this design does not decide

The author display name stays deferred (§5). No `blame`, no `show`, no trailers (§7). **How `prikk
log` formats a message** — one line, wrapped, truncated — is presentation and belongs to the increment,
not here. And the **release grouping** — whether schema 4 ships alone or with other work — is the
owner's, since §8.6 makes it a compatibility announcement rather than a feature.
