# Amendment to `exit-code-contract-handoff-v1.md` — round 3 is §3 only, and its scale was understated

**v1 and v2 stand. This corrects one number and states what is left, so a reader picking up round 3
does not have to subtract two finished increments from a multi-part document.**
**Architect, 2026-09-02.**

---

## 1. What is done, and must not be redone

| | Landed |
|---|---|
| v1 §2 — the `CliError` type and the `0`/`1`/`2` mapping | `215b497` |
| v1 §5 — the JSON-printer `panic!` | `215b497` |
| v1 §4 — `unlock`'s abort path exits `1` | `44c9d2a` |
| v2 §2 — the stdout write-failure exit code | `44c9d2a` |

**Round 3 is v1 §3 and nothing else.** v1 §9's seam is spent; do not look for another.

## 2. The correction: §3.3 understated the work by roughly threefold

v1 §3.3 says *"Nine files define a `parse_*_args`"*. That is true and it is the wrong unit — **the
work is per-function, and there are 26**:

```
args.rs 9 · sync.rs 6 · args/merge_evidence.rs 3 · bundle.rs 3
args/checkout.rs 1 · args/merge_execute.rs 1 · branch.rs 1 · seal.rs 1 · tag.rs 1
```

Plus **twelve files carrying inline argument-matching loops** outside any `parse_*_args`, plus
commands with no parser at all — `status`, which swallows `--nonsense` and exits `0`, is exactly that
shape and is the reason §3.1's reproduction works.

**This is my error, and it is the same class the dev team has now caught in my handoffs three
times**: a count taken at the wrong granularity, stated as if it bounded the work. v1 §3.3 already
told you the list was a floor; it should also have told you the unit was wrong.

## 3. One thing the contract now makes checkable

`prikk commit` with no `-m` exits **`1`**. Under §1's ruled contract a missing required flag is a
**usage error — `2`**. That is round 3's to fix, and it is a good first probe: **it exercises the
`CliError::Usage` path from a real parser rather than from `main.rs`'s dispatch arm**, which is the
only place `Usage` is constructed today.

## 4. Everything else in v1 §3 is unchanged

The shared in-repo helper (no new dependency — `prikk-cli` stays at zero, `placement.rs` enforces
it), unknown-argument rejection, duplicate-flag refusal with `bundle export --ref` as the named
victim, `doctor --repair-main-ref`'s recognized-and-refused shape preserved, and the enumeration by
mechanism reported as its own result.

**No CI control** — that is the architect's at push time.
