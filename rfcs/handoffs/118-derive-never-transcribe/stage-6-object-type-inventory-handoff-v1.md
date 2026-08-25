# RFC 118 stage 6 — derive the `ObjectType` inventory

**Base:** current `main` (`2edfb56`, CI green on all 12 jobs). **Under `003-landing-work-on-main.md`.**
**Origin:** the gap the DC-21 reviewer found and correctly did not fix — `RecognitionClaim` is missing
from a proptest sampling list.

---

## 1. The defect, and why it is not "add one line"

`crates/prikk-store/src/file_codec/tests.rs`:

```rust
const ALL_OBJECT_TYPES: [ObjectType; 9] = [ /* Patch, Block, RefState, RefUpdate, Tag,
                                               Attestation, Blob, BlockSummaryCache, RecoveryNote */ ];
```

**Nine members. Ten live variants. `RecognitionClaim` is absent** — so the one object type that
actually travels on the sync wire is excluded from these property tests' random sampling, while the
exhaustive match below them enumerates it correctly.

**Adding `RecognitionClaim` and bumping `9` to `10` is the wrong fix** and will be rejected. It re-arms
the identical trap for the eleventh type.

**The variant set is transcribed five times:**

| Site | Drift-proof? |
|---|---|
| the `enum ObjectType` declaration | — |
| `from_code`'s match | **Yes** — compiler-enforced |
| `name()`'s match | **Yes** — compiler-enforced |
| `signature_contract_tests/vectors.rs::ALL_OBJECT_TYPES` + its hand-written `assert_eq!(len(), 10)` | **No** |
| `file_codec/tests.rs::ALL_OBJECT_TYPES` | **No — and it has already drifted** |

The matches cannot silently disagree with the enum. **The two hand-written lists can, and one did.**

## 2. A gate over a hand-written list cannot fix this

**Do not propose a hand-maintained `ALL` plus a completeness test.** RFC 118 stage 4 settled this and
the reasoning is in that increment's review: adding a variant forces you to extend the *forward*
match, but nothing forces a *separate list* to grow, and a test can only inspect what the list already
contains. Stable Rust has no `mem::variant_count` and no variant reflection. **A test cannot see a
variant it was never handed.**

**So: one token list, generating the enum and its inventory together** — the shape used twice already,
by `verification_stages!` (stage 4) and `conflict_witness_kinds!` (DC-21).

## 3. What to build

**Generate `ObjectType` from a single token list**: the variants **with their explicit discriminants**,
plus `ALL`, plus `code()`, `from_code`, and `name()`. One place to add the eleventh type; no second
place to forget.

- **`RETIRED_CODES` stays separate.** It is not a variant list, and `0x0A` must keep being refused
  **before** the live match — do not let the macro reorder that.
- **Per-variant doc comments must survive.** Both prior macros preserved theirs; a conversion that
  quietly ate the documentation would be a real loss.
- **`ObjectType` is public API of a published crate.** `code()`, `from_code`, and `name()` must behave
  identically; `ALL` is an additive `pub const`. **Report any signature change** — I expect none.

**Then make both `ALL_OBJECT_TYPES` lists derive from `ObjectType::ALL`**, and **delete
`vectors.rs`'s hand-written `assert_eq!(ALL_OBJECT_TYPES.len(), 10)`** — a length pinned by hand is the
same defect one layer down.

**The `RecognitionClaim` omission then closes by construction, not by being noticed.**

## 4. Out of scope

- **Changing any type code, name string, or `from_code` behaviour.**
- **The retirement of `0x0A`**, beyond keeping it working.
- **`ConflictWitnessKind` and `VerificationStage`** — already done; do not "unify" the three macros.
- Adding or removing an object type.

## 5. Controls

1. **A new variant reaches every consumer with one edit** — add a throwaway variant to the token list
   and **nowhere else**; show it appears in `ObjectType::ALL` and in **both** derived test arrays.
   Quote it, then revert. This is the control that proves the whole increment.
2. **The proptest now samples `RecognitionClaim`** — show it is exercised, not merely present in a
   list. If the strategy makes that hard to observe directly, say how you established it.
3. **Public behaviour is unchanged** — `code()`, `from_code`, and `name()` agree with `2edfb56` for
   every live variant, and `from_code(0x0A)` still returns the retirement error.
4. **Doc comments survived** — say how many per-variant doc lines exist before and after.
5. **Full suite green**, and say whether the count moved and why.

**Quote every failure.** A control that passes without your assertion firing is worse than none.

## 6. What to report

1. **The macro**, and **why the eleventh type cannot be added to one place and forgotten in another.**
2. **Whether both `ALL_OBJECT_TYPES` lists could genuinely derive**, or whether either needs a subset —
   `file_codec`'s excludes nothing today, but **say so rather than assuming.**
3. All five controls (§5), quoted.
4. **Full gate set against the exact commit, after the last edit.**
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here that was wrong.

**Stop and escalate, do not guess**, if: explicit discriminants cannot be carried through a
`macro_rules!` token list cleanly; the two test lists turn out to need *different* member sets (that is
a real finding, not a blocker to work around); or generating a **public** enum this way changes
anything a downstream consumer could observe.
