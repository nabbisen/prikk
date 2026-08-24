# Gate A — the `0..=8` sweep bound

**Base:** current `main`. **Under `003-landing-work-on-main.md`.**
**Origin:** `.git-exclude/reviewed/gate-a-pair-granularity-review-v1.md` §4 — surfaced by the dev team's
own failed first control during `f1528b8`, and recorded then rather than absorbed.

**Small. One line and a comment, or one line and a derivation.** But it is a **verification gate**, so
§3's control is not optional.

---

## 1. What is wrong

`signature_contract_tests/vectors.rs:415`:

```rust
for schema_version in 0u32..=8 {
```

**A bare magic number with no justifying comment.** Gate A's coverage stops at schema 8: **a pair
admitted at schema 9 or above is invisible to the guard**, while the test's own name and doc still say
*"every admitted pair."*

**This is the same class of defect `f1528b8` just fixed** — a guard whose real coverage is narrower than
its stated coverage — one dimension over. `f1528b8` fixed the *type-vs-pair* dimension; this is the
*range* dimension.

**How it surfaced is the point.** Implementing `f1528b8`, the dev team's first control used throwaway
schema `99`. **Gate A passed** — because `99` fell outside the sweep, so the pair was never enumerated
and the control proved nothing. **They caught it, switched to `5`, and reported the failed attempt.**
Had they not, a control that never reached its target would have been recorded as evidence the gate
works.

## 2. Two acceptable fixes — pick one and say why

**(a) Derive the bound** from the admitted set — e.g. sweep to `max(admitted schema) + N` — so it widens
automatically when a new schema is admitted.

**(b) Keep a literal bound and justify it in a comment** — stating why the bound is safe, and what must
be changed if a schema ever approaches it.

**(a) is better if it is clean.** If deriving it means reaching for something awkward, **(b) with an
honest comment is entirely acceptable** — a documented bound is not a defect; an undocumented one is.

**What is not acceptable is raising `8` to a larger number with no comment.** That moves the cliff
without removing it.

## 3. The control

**Whichever fix you choose, prove the guard now sees a pair it previously could not.**

Temporarily admit an unvectored pair **above the old bound** — the `(Patch, 99)` case that silently
passed during `f1528b8` — run Gate A, and **quote the failure.** Then revert; `git status` clean before
the final gate run.

**Under fix (b), that control must still work** for any schema inside the new stated bound — so choose a
throwaway value that exercises the range you are claiming, and say which you used and why it is inside
it.

**Also confirm Gate A still passes unmodified.**

## 4. Out of scope

- **`validate_format2_schema`'s admitted set** — read it, do not change it.
- **Any identity vector.** No vector should be added or changed.
- **Gate A's pair predicate** — fixed at `f1528b8`, untouched here.
- **`MILESTONES.md`** — criterion 2 already records this residual; **I will update it when this lands.**

## 5. What to report

1. **Which fix you chose and why** (§2).
2. **The control, with the quoted failure** (§3), the throwaway pair used, and confirmation the tree was
   clean afterwards.
3. **Confirmation Gate A passes unmodified.**
4. **Confirmation no vector and no admitted-set entry changed.**
5. **Full gate set against the exact commit, after the last edit.** Test counts — **expected unchanged.**
6. Anything here that was wrong.

**Stop and escalate, do not guess**, if: deriving the bound (a) turns out to need the admitted set
duplicated rather than read — **that is the trap `f1528b8` already caught once**, where deriving a guard
from the thing it guards destroys the guard; or the sweep bound turns out to be load-bearing for
something other than coverage, such as test runtime.
