# Crate READMEs — stop duplicating the description, and gate them too

**Base:** current `main` (`e804ea5`, CI green on all 12 jobs). **Under `003-landing-work-on-main.md`.**
**Origin:** the follow-up found and correctly not fixed while rewriting the manifest descriptions.

---

## 1. The defect is duplication, not just staleness

Seven library crates each carry a `README.md` that crates.io renders **directly beneath the
description**. Six of the seven are **exactly three lines** — a heading, a blank line, and **their own
pre-fix description, verbatim**:

```
# prikk-store

Prikk storage crate scaffold.
```

**So a `prikk-store` visitor currently reads "storage crate scaffold" twice on one page**, once from
each field.

**Restating the description in the README is the actual bug.** It puts one sentence in two files with
nothing binding them — the transcription defect this project keeps removing, and precisely how these
went stale while the manifests were fixed.

**Do not fix this by copying the new descriptions into the READMEs.** That reproduces the defect with
fresher text and guarantees the same drift next time.

## 2. `prikk-replay` already got it right — use its shape

It is the only one of the seven that says something the description cannot:

```
This crate is workspace-internal during replay-boundary stabilization. Public Rust items exist for
workspace integration, primarily `prikk-store`, and do not imply a stable external API.
```

**That is the posture ruling, written before the ruling existed.** Make the other six match its
*shape*: say what the crate is for **within prikk**, and point a visitor at the `prikk` CLI — content
the one-line description has no room for.

**"During replay-boundary stabilization" is now stale in its own way** — the posture is settled, not
transitional. **Update that phrase too**, and keep the rest.

## 3. What each README must and must not do

**Must**: name what the crate does inside prikk, state that it is an internal component whose API may
change before 1.0, and point at `prikk` — the crate a visitor probably wants.

**Must not**: repeat the manifest description verbatim. **If a sentence would be byte-identical to the
description, it does not belong in the README.**

**Do not oversell**, and **do not add usage examples** for an API §2's ruling declares unsupported —
an example is a promise.

## 4. Extend the gate

`boundary-check`'s `package` module now refuses provisional language in **descriptions**. **It does not
read these READMEs**, so `scaffold` can sit in seven files indefinitely — which is exactly what
happened.

**Extend the same check to each published crate's `readme` target.** The wordlist and the
substring/whole-word split are already built and already correct; reuse them rather than writing a
second mechanism.

**Adjudicate one thing**: `prikk`'s `readme` points at the **workspace root `README.md`**, which is a
much larger document with different rules. **Decide whether the gate covers it**, and say why. My lean
is that it should — a provisional word in the root README would be at least as visible — **but if
scanning a long prose document produces false positives the manifest check never faced, say so and
scope it to the seven.**

**Consider gating the duplication itself**: a check that a crate's README does not contain its own
description verbatim would make §1's rule mechanical rather than editorial. **Adjudicate whether that
is worth it** — I have not decided, and a reasoned "no" is acceptable.

## 5. This cannot repair `0.25.0`

crates.io serves the README from each published version. **`0.25.0`'s pages keep the current text
permanently**; this takes effect at the next publish. **Say so rather than implying otherwise.**

## 6. Out of scope

- **The workspace root `README.md`'s content.** Only §4's gating question touches it.
- **Manifest descriptions**, settled last increment.
- **Any code, API, or behaviour change** beyond the gate.
- **Republishing.** Mine, at the next cut.

## 7. Controls

1. **The extended gate fires on a provisional word in a crate README** — plant one, quote, revert.
2. **The gate fires on a missing README** that a manifest's `readme` field names — or, if you conclude
   that case cannot occur, show why.
3. **No README restates its description** — show it mechanically, not by reading.
4. **Full gate set green**, count moved and why.

**Quote every failure.** If a control passes without your assertion firing, say so.

**Revert control mutations with an explicit restore, not `git checkout --`, for any file whose fix is
not yet committed** — that discarded real work in the last increment.

## 8. What to report

1. **The seven before/after READMEs**, in full.
2. **Your §4 adjudications**: the root README, and whether duplication itself is gated.
3. All four controls (§7), quoted.
4. **Full gate set against the exact commit, after the last edit.**
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here was wrong.

**Stop and escalate, do not guess**, if: a crate cannot be described without repeating its description
(that would mean the description is doing the README's job or vice versa, and I want to see it); or the
root README trips the wordlist on legitimate prose — **that is a finding about the gate, not a reason
to reword the README.**
