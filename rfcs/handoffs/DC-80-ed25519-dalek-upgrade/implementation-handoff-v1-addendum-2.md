# DC-80 Handoff v1 — Addendum 2: §1 accepted, implementation cleared

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-80-prerequisite-questions-review-v1.md`.

## 1. Accepted — and I reproduced your probe rather than take it

I built my own cross-version harness. Same results, including the one that decides this increment:

```
3 S+L overflow=false high-3-bits-set=false
3 S+L rejected by 2.x / 3.x : true / true
```

**That is the silent direction — 3.x accepting what 2.x rejected — and it does not happen.**

**And `high-3-bits-set=false` is what makes the case worth having.** The malleated scalar does not have
its top bits set, so the naive high-bit heuristic would miss it entirely. You exercised the subtle shape,
not a trivially-caught one. **Your `L` constant is also correct** — I used the same value independently;
a wrong one would have produced a rejection for the wrong reason and a test proving nothing.

Also confirmed: both MSRVs land **at** 1.85, and `verify_strict` differs by **exactly one line**
(`message` → `&[message]`), as you said.

## 2. Two judgment calls I want to affirm

**Naming the constant-time-equality CHANGELOG entry and explaining why it is *not* relevant** — timing
profile, not which values compare equal — is better than omitting it. An entry that looks alarming and
is not should be addressed, not skipped.

**Declining to restate addendum-1's six-package collapse figure** was right. Measuring means touching the
manifest, which your §1 clearance excluded, and "not measured, and here is why" beats repeating my
number as though you had checked it. **That is the third time this cycle you have declined to inherit a
figure of mine.** Measure it now and report before/after, per criterion 3.

## 3. Cleared — with the bar unchanged

**Criterion 4's negative control remains the acceptance bar, and your §1 probe does not discharge it.**
That probe was a throwaway harness; criterion 4 asks for both directions demonstrated **against the real
workspace**, with a repository sealed under 2.x verifying identically under 3.x.

Hard limit not triggered — nothing here requires changing what is signed, the preimage, or any envelope
shape. If implementation suggests otherwise, **stop and report**.

**Green macOS run before merge.** Gates per rule 9 as amended.
