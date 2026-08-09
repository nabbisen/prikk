# DC-82 Handoff v1 — Addendum 1: accepted, merged, and both questions answered

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-82-implementation-review-v1.md`. **Merged to `main` at `c2bb296`** after a
green macOS CI run — including the `macOS mutation test suite` job.

## 1. Your first question: the scope was right, the ambiguity was mine

**"Cleared to start on §3 only" was ambiguous and I wrote it.** §3 of *which* document? The RFC's §3 is
the blocking prerequisites; this handoff's §3 is "the one thing this must not break." Every prior
handoff put the questions in its own §1, so the bare number carried a convention this one broke.

**Your reading was reasonable, and your reason for it was right:** you cannot demonstrate a constraint
without something to demonstrate it *of*. You also stopped short of pushing and flagged the ambiguity
rather than absorbing it. **No overshoot.**

**And you recounted rather than matching my number** — eleven mutation functions, not ten. Reporting the
discrepancy instead of quietly conforming is the right instinct and I would rather have it every time.

## 2. Criterion 5's demonstration is better than what I asked for

Gating `none.rs` `#[cfg(any(test, not(any(linux, macos))))]` so it is visible in test builds **on every
platform** lets `no_durability_every_method_fails_at_runtime_not_compile_time` exercise all eleven
methods on the dev host. That is stronger and cheaper than cross-compiling and running a binary, and it
is not what I had in mind when I wrote the criterion.

The note that your first run caught the `"i/o error: "` prefix — **fixed after seeing the failure, not
asserted blind** — is the same discipline that has run through this whole sequence.

## 3. Your second question: criterion 3 is not met, and the target was mine to miscalibrate

**`fsutil/` production gates went 121 → 94. Not single digits, and your diagnosis is correct** — I
verified it. `directory.rs`'s remainder is `use rustix::fd::OwnedFd`, `use rustix::fs::{Mode, OFlags}`,
a gated struct field. Those are **per-platform type and primitive differences underneath the contract**,
not dispatch branching, and the pattern cannot reach them by construction.

**My error:** DC-81 §6 set the single-digit target from a call-site-layer argument, and I then wrote
criterion 3 as a whole-tree number without checking what was actually reducible.

**The property that matters is met, and you named it exactly:** `anchored.rs`'s remaining 14 gates scale
with **platform count**, not **call-site count**. Windows now costs about one line per gate instead of
eleven new arms. That was the point.

## 4. And the answer on the remaining layer: defer it to Windows. Do not do it standalone.

Those Unix-only helpers exist because `LinuxDurability` and `MacosDurability` share them. **Windows will
not use them at all** — different primitives entirely. So the natural seam, gating the Unix helpers once
at module level rather than per item, **falls out of the Windows increment**. Cutting it now, before
Windows' shape is known, risks putting the seam in the wrong place.

**Recorded as a Windows-increment target**, not dropped.

## 5. Nothing owed

DC-82 is complete. **DC-79** (`sha2` + `getrandom`) is next in your queue, then **DC-80**
(`ed25519-dalek`). DC-78's design is mine and blocked on an owner ruling.
