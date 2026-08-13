# RFC 103 Retire Format-1 — Prerequisite Handoff v1

**Cleared to answer §8's three prerequisites only.** Accepted 2026-08-13,
`rfcs/accepted/103-retire-format-1.md`. **No production code.**

## 1. What this is

Format-1 is retired: a format-1 repository is **rejected at open**. Not read-only, not auto-upgraded —
either would keep every dual-path branch alive, which is the cost being removed.

**The owner has accepted the risk explicitly** — format-1 repositories in the wild become unopenable by
every future version — on the grounds that prikk is in early development. That decision is made; do not
re-litigate it or design around it.

## 2. The prerequisites

1. **Enumerate every format-1 site independently.** My count is **22 `LegacyV1` mentions across 13
   files** plus five pieces of legacy-only machinery — that is one grep, not a derived set. Four
   consecutive investigations this month found my counts narrower than the code. Derive it yourself.
2. **Confirm each of the RFC's §2 three checks is genuinely format-1-only**, from DC-95 Stage 1's classification
   and the code — not from my table.
3. **Establish what a format-1 repository does at open today** — which code path first notices, and what
   it reports. The RFC's §4 rejection contract cannot be written against a guess.

## 3. The one thing most likely to be got wrong

**The `created_at == 0` check survives.** With format-1 gone it stops meaning "contaminated by an old
format" and becomes plain malformed-data detection — **unconditional, not weaker.** DC-95 Stage 1
classified it load-bearing.

Anyone reading the removal list and assuming "legacy checks go" would delete it. Don't.

## 4. Constraints

- **No check is rewritten, moved, or deleted beyond the three named in the RFC's §2.** Anything else that looks
  removable is a finding to report.
- **DC-95's classified inventory is updated in the same increment** — three of its 41 rows change status,
  and it is the map a future reader consults.
- **The rejection must be actionable**: name the detected format, the required one, the last version that
  supported format-1, and the bundle-export remedy. A bare `malformed persisted data` fails the RFC's §4.
- A stop-and-report is a complete outcome, as always.

## 5. Sequencing

RFC 102 §6.3 is also available. **Neither blocks the other**; take them in whichever order suits, but
raise it rather than assuming — that question has produced a better answer than the options offered,
twice.
