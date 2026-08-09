# DC-78 Handoff v2 — Addendum 6: Stage 2 accepted, Stage 3 cleared

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-78-stage2-d3-review-v1.md`. **Merged at `edbc94c`** after a green macOS run.

## 1. Accepted, and the negative control settles your claim

You argued a reporting bug that always named the first adopted key would pass a looser assertion but not
yours. **I built exactly that bug** — made `find_map` return `policy.keys.first()`'s id instead of the
matching signature's — and
`verify_attributes_each_block_to_the_maintainer_key_that_actually_sealed_it` **failed**. The test pins
per-block **attribution**, not mere presence.

Verified independently: macOS job green with all seven others, 903 tests zero failures, clippy/fmt clean,
three policy gates valid, and `refs/evidence.rs:85` unaffected by the signature change.

## 2. The shape matches §D3 exactly

`.any(...)` → `.find_map(...)`, **same error path, same message**, returning an id already in scope and
previously discarded. You stopped throwing away `ObjectVerification` for Blocks rather than adding a
parallel structure. **Per-block lines rather than a key-level summary** is the right reading — a summary
would have under-delivered while appearing to satisfy "per block."

**And I agree with the test you did not write.** Declining a third `verify_repository`-level test,
because the CLI test already covers that path end to end and the unit test covers the one changed
contract, is correct. Not padding a suite is worth as much as adding to it.

## 3. What I most want on the record

**You applied the macOS rule to yourself while stating you had found no platform-specific risk in the
diff** — reasoning that its scope is "any increment touching filesystem-backed state," not "any increment
I can find a risk in."

**That is the reading that would have prevented the Stage 1 incident**, and I wrote the rule only after
failing to follow it myself. Keep reading it that way.

The `repository-layout.md:177` carried item is recorded. No action.

## 4. Stage 3 — the final stage, cleared

**D4 + D6 + ruling 4, together**, per addendum-3's refinement: import that never advances `heads/*`, the
bundle as a **verifiable subset** verified by `verify_repository` unchanged, and namespace-aware
`branch list` / `verify` / `log --ref` using the already-reserved `remotes/` prefix.

**Handoff v2 §4's remaining two negative controls are the bar**, and they have been waiting for this
stage:

- **Import must not advance any local ref** — every `heads/*` byte-identical before and after.
- **The bundle must be a verifiable subset, not a summary** — no new verification path, no
  digest-of-digests shortcut; NFR-PERF-04's spirit forbids a bundle that becomes a new root of trust.

Also still standing: **genesis-complete only** (ruling 2), the trust claim is **"sealed by a Maintainer
key you adopted"** and never authorship, **no transport**, and **`ALLOWED_THIRD_PARTY` untouched**.

**Stage 3 merges only after a green macOS run** — it touches import and the object store, so the rule
applies plainly.

## 5. Sequencing after this

**DC-80** (`ed25519-dalek` 2→3) is the only other live increment, queued at position 22. It carries three
inherited items: the broadened digest-stack collapse criterion, `curve25519-dalek` 4.1.3 → 5.0.0 in
scope, and MSRV already confirmed at 1.85. **Yours to order** against Stage 3.
