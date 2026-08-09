# DC-79 Handoff v1 — Addendum 1: both questions ruled, proceed to the upgrade

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-79-prerequisite-questions-review-v1.md`.

## 1. Accepted — and the method is why the rulings are easy

You answered every question by **doing the upgrade, measuring, then reverting**, and found the
`getrandom` API break from the actual compile error rather than a changelog. Verified independently:
both target crates declare `rust-version: 1.85`, and `ed25519-dalek 2.2.0` does transitively require
`sha2 0.10`.

## 2. Ruling 1 — land DC-79 now. The duplication is transient whichever order we pick.

I checked the other side of your finding, which resolves it rather than deferring it: **`ed25519-dalek 3`
requires `sha2 0.11`** — confirmed by resolving a throwaway project against `ed25519-dalek = "3"`.

| State | `sha2` versions |
|---|---|
| Today | 1 |
| **DC-79 only** | **2** (0.10 via dalek 2, 0.11 direct) |
| **DC-80 only** | **2** (0.11 via dalek 3, 0.10 direct) |
| **Both** | **1** |

**No ordering avoids the window — only landing both closes it.** So the real choice was a temporary
second `sha2`, or re-bundling DC-79 into DC-80.

**Land DC-79 now.** Two `sha2` versions compiled in is cosmetic — SHA-256 is SHA-256, and your own
evidence shows no digest moves. Re-bundling would undo a split made deliberately, because DC-80 carries
behavioural risk to already-sealed signatures and this increment does not.

**DC-80 gains an acceptance criterion from this:** *"`sha2` collapses back to a single version"* — cheap
to check, and it confirms the pair landed correctly.

**Advance finding for DC-80, from the same probe:** `ed25519-dalek 3` also pulls **`curve25519-dalek`
4.1.3 → 5.0.0**. It declares `rust-version: 1.85` so MSRV holds, but that second major bump is now in
DC-80's scope to investigate, not just ed25519-dalek's own changes.

## 3. Ruling 2 — the rename is in scope, and my wording was the problem

**`getrandom` → `fill` is in scope. Do it.**

§3 said *"only versions move"*, which reads more narrowly than I meant; the condition that actually
matters is the other sentence — *nothing computes differently*. **A rename forced by an upstream API
change is part of a version bump, not scope creep.** Your evidence that it is behaviour-preserving
(identical counts on both toolchains) is what makes this easy.

Asking rather than assuming was right given how I wrote it.

## 4. Your scope note is taken, and it is the second instance

You flagged that "cleared to start on §1 only" did not map, since the questions live in §2 — **the same
ambiguity DC-82 hit.** Saying so rather than letting it pass twice is what stops it becoming a habit.

**Standing correction:** future handoffs will name the section by content — "cleared to answer the
prerequisite questions" — since numbering is not stable across documents.

## 5. Proceed

MSRV holds, no digest moves, rename cleared, duplication accepted as transient. **Criterion 2's proof
stands as you ran it:** DC-41's vectors and the frozen pre-DC-55 differential passing **unchanged**, with
no edited hash literal anywhere. Gates per rule 9 as amended.
