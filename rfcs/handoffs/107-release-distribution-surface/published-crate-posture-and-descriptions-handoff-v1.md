# Published crates — rule the API posture, fix the false descriptions

**Base:** current `main` (`c8366cf`, CI green on all 12 jobs). **Under `003-landing-work-on-main.md`.**
**Origin:** the `#[non_exhaustive]` question raised by `0.25.0`'s three API breaks. **Researching it
found something worse**, and answered it at the same time.

---

## 1. What the research found

**`#[non_exhaustive]` is used zero times in this workspace.** My `0.25.0` readiness assessment said
"exactly once, in an unrelated `fsutil` file" — **that was wrong**: the hit was
`.finish_non_exhaustive()`, a `Debug` formatter helper, unrelated to the attribute.

**More seriously — two published crates describe themselves falsely, and it is live on crates.io now:**

| Crate | Description on crates.io today |
|---|---|
| `prikk` | **"Prikk CLI initial scaffold."** |
| `prikk-store` | **"Prikk storage crate scaffold."** |

**`prikk` is the flagship crate.** At `0.25.0` it is a released distributed VCS that syncs history,
merges with lineage, travels tags, gates trust, and emits machine-readable verification. **"Initial
scaffold" was true at `0.1.0` and is false now** — and it is the first sentence anyone reads.

**This is the same defect class this project spent weeks removing** — false README claims, false
ROADMAP themes, stale CLI output — **sitting on the most public surface there is.** It shipped in a
release I cut, and **nothing gates it**: no check in `boundary-check` or anywhere else looks at
`description` at all.

**Two more are accurate but written for the wrong reader:** `prikk-replay` ("Workspace-internal replay
and lifecycle semantics") and `prikk-ffi` ("Prikk's sole FFI surface -- DC-96, the exemption named in
`UNSAFE_EXEMPT_CRATES`"). **A crates.io visitor does not know what DC-96 is.**

## 2. The ruling on posture

**Only `prikk` — the CLI — is a supported product surface. The seven library crates are published
implementation detail.** `prikk-replay`'s own description already says "workspace-internal"; the
others are published for the same reason, which is that the CLI cannot be published without them.

**Their APIs may change without notice before 1.0.** `0.25.0` demonstrated it three times.

**Say this in the descriptions**, so a reader learns it before depending on one — not after.

## 3. The ruling on `#[non_exhaustive]`: do not adopt it

**Rejected, and record why so it is not re-litigated.**

The argument for adopting it now is real: it is free before 1.0 and a breaking change to add later.
**But it buys stability for an API this project does not offer.** Adding the attribute across ~170
public types would forbid downstream struct-literal construction and exhaustive matching, cost a large
mechanical diff, and protect consumers of a surface §2 declares unsupported.

**`prikk` itself has no `lib.rs`** — the product surface is a binary, and a binary has no
`non_exhaustive` question.

**Revisit only if a library crate is ever declared a supported API.** That is the trigger; nothing else
is.

**If you think this ruling is wrong, argue it before implementing anything** — this is a design
position, and it is cheaper to overturn now than after 170 attributes land.

## 4. What to change

**Rewrite all eight descriptions** so each is true at `0.25.0` and legible to a crates.io visitor:

- **`prikk`** — what the tool *is*, in one sentence, for someone who has never heard of it. **No
  "scaffold", no version-relative language.**
- **`prikk-store`** — what it holds, not "scaffold".
- **The six library crates** — each should say plainly that it is an internal component of `prikk`
  whose API is unstable before 1.0. **`prikk-ffi` must lose the DC-96 jargon**; keep the meaning, drop
  the internal reference.
- **Do not oversell.** The project's own README limits still apply; a description that implies more
  than the tool does is the same defect in the other direction.

**Report the eight before/after strings in full** — this is the one artifact a reviewer must read
literally.

## 5. Gate it, or say why it cannot be gated

**Nothing checks `description` today.** A description written once and never revisited is exactly how
"initial scaffold" survived to `0.25.0`.

**Adjudicate what is gateable.** A gate cannot judge whether a sentence is *true*. It can plausibly
check that **every published crate has a non-empty description**, and that none contains provisional
language — `scaffold`, `initial`, `placeholder`, `TODO`, `WIP`.

**My lean is that this narrow gate is worth having**; a wordlist ages badly, but these five words are
never correct in a published release. **If you conclude a wordlist is the wrong mechanism, say so and
propose what replaces it** — including "nothing; this is a review responsibility," which is an
acceptable answer if argued.

**`boundary-check`'s `package` module is the natural home** — it already reads manifests.

## 6. This cannot repair `0.25.0`

**crates.io serves the description from each published version.** `0.25.0`'s listing keeps
"initial scaffold" permanently; the fix takes effect at the **next publish**. **Say so in the report**
rather than implying the live listing changes.

## 7. Out of scope

- **Adding `#[non_exhaustive]` anywhere** (§3).
- **Any code, API, or behaviour change.** Manifest metadata and, if adjudicated, one gate.
- **Other manifest fields** — `keywords`, `categories`, `readme`. **If one is also wrong, report it,
  do not fix it here.**
- **Republishing.** Mine, at the next cut.

## 8. Controls

1. **The gate fires on a provisional description** — set one back to "scaffold", quote the failure,
   revert. (If §5 concludes no gate, say so and skip.)
2. **The gate fires on a missing description** — remove one entirely, quote, revert.
3. **Every published crate has a description** after the change, and none contains provisional
   language — show it mechanically, not by reading.
4. **Full gate set green**, count moved and why.

**Quote every failure.**

## 9. What to report

1. **The eight before/after descriptions**, in full.
2. **Your §5 adjudication** on gating.
3. **Whether you agree with §3**, and why — a one-line "agreed" is fine, but an objection is more
   valuable if you have one.
4. All controls (§8), quoted.
5. **Full gate set against the exact commit, after the last edit.**
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here that was wrong.

**Stop and escalate, do not guess**, if: a description cannot be made true without claiming something
the README's own limits contradict; or you find a published crate whose *name* is misleading —
**renaming a published crate is not a metadata fix and is not yours or mine to decide alone.**
