# G1 — a declared break excuses the wrong failures

**Base:** current `main` (`9c7472e`). **Under `003-landing-work-on-main.md`.**
**Origin:** found while choosing a control site for the `0.25.0` fixture refresh, reported and
correctly not fixed there.

---

## 1. The hole

`g1_last_release_fixture_is_compatible_or_the_break_is_declared` decides a break is authorized with:

```rust
DECLARED_BREAKS.iter().any(|break_| break_.object_type == object_type)
```

**`version_pair` is never consulted.** It is populated, and
`every_declared_break_names_a_persisted_object_type` asserts about it — but it plays no part in
deciding whether a failure is excused.

**`Tag` is therefore exempt from G1 permanently.** The single entry records a `0.22.1 -> 0.23.0`
break. The fixture is now `0.25.0`. **A `Tag` decode failure against a `0.25.0` fixture means current
code cannot read what `0.25.0` wrote — which has nothing to do with a 2026 `0.22.1` break — and this
gate would report success.**

## 2. The entry's own doc states the hole as if it were the design

> it becomes load-bearing the day a *future* release breaks `Tag` again

**That is the defect, described as a feature.** An entry authorized for one version pair "becoming
load-bearing" for an unrelated future break is precisely what must not happen. **Correct this
sentence** — leaving it would re-teach the misconception to the next reader.

## 3. What an entry can ever mean

The fixture is always **the last release**; the gate always compares it against **current code**. So
the only pair a declared break can describe is **`<fixture version> -> current`**.

**An entry whose older side is not the fixture's version cannot apply to anything the gate checks.**
The `0.22.1 -> 0.23.0` entry is in that state today, and every future fixture refresh will put more
entries into it.

**So a stale entry is not merely inert — it is actively dangerous**, because the predicate in §1 lets
it excuse a live failure on a matching object type.

## 4. What to build

**Make applicability version-scoped, and make staleness loud.**

- **Introduce a constant naming the fixture's release** (`0.25.0` today) and **derive
  `last_release_fixture_root()`'s path from it** — the version is currently a literal inside a path
  string, which is one more transcription that will go stale at the next refresh.
- **An entry applies only when its older side equals that constant.** Both conditions, not either.
- **Add a test that every `DECLARED_BREAKS` entry is applicable to the current fixture.** A stale
  entry must **fail loudly**, not sit inert — otherwise §1's hole reopens the moment someone adds a
  second entry for an object type that already has one.

**Adjudicate the consequence, and say which you chose**: with that test, the `0.22.1 -> 0.23.0` entry
**fails**, because it is stale. Either

- **retire it** — its record already lives in `CHANGELOG.md`'s `0.23.0` breaking-change section, and
  `format_stability_gate.rs`'s precedent is an **empty-but-ready** list; or
- **keep a historical record somewhere the gate does not match against** — a separate constant or a
  doc comment.

**My lean is retire it and let the list be empty.** A list that mixes gate inputs with documentation is
the defect this project keeps removing, and the CHANGELOG is already the record. **But argue it if you
disagree** — what is not acceptable is a list where an inapplicable entry can still excuse a failure.

## 5. Out of scope

- **The fixture itself**, refreshed last increment.
- **The reverse-direction ruling**, still outstanding on this same file — **if it lands first, rebase
  onto it and do not revert it.**
- **`format_stability_gate.rs`.**
- **Any product behaviour.** This gate is test-only.

## 6. Controls

1. **A stale entry now fails** — restore the `0.22.1 -> 0.23.0` entry (or add an equivalent) against
   the `0.25.0` fixture and quote the failure.
2. **A live failure is no longer excused by a stale entry** — this is the actual defect. With a stale
   `Tag` entry present, break `Tag` decoding and show the gate **fails**. **Before the fix it would
   have passed** — run it both ways and quote both, or say why you cannot.
3. **A genuinely applicable entry still excuses its own break** — construct one whose older side
   matches the fixture, break that type, and show the gate passes. **The gate must still be able to
   authorize a real break.**
4. **The path derivation is real** — changing the version constant moves the fixture path. Show it.
5. **Full suite green**, count moved and why.

**Quote every failure.** Control 2 is the one that proves the increment; a green run there without the
before-and-after is not evidence.

## 7. What to report

1. **The predicate, before and after.**
2. **Your §4 adjudication** on the historical entry, with reasoning.
3. **The corrected §2 doc sentence.**
4. All five controls (§6), quoted — **especially control 2's before-and-after.**
5. **Full gate set against the exact commit, after the last edit.**
6. **Whether the reverse-direction increment had landed** when you started.
7. **Every numbered requirement's disposition, including ones that went without incident.**
8. Anything here was wrong.

**Stop and escalate, do not guess**, if: making the match version-scoped turns out to break a case I
have not considered — **that would mean the current predicate is deliberate and I have misread it, and
I want to know before it changes.**
