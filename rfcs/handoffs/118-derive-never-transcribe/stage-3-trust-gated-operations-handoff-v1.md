# RFC 118 stage 3 — trust-gated operations

**Base:** current `main` (`81d85e0`). **Under `003-landing-work-on-main.md`.**
**RFC:** `rfcs/accepted/118-derive-never-transcribe.md` §10.3, **owner-selected 2026-08-24.**

**Read §2 first. It states what this stage does *not* achieve, and that limit must survive into the
documentation.**

---

## 1. The transcription being removed

`docs/src/reference/trust-threat-model.md:80` lists the gated operations in prose: *"`merge`,
`sync build`, `sync seal`, `sync adopt-tag`, and, since `053e442`, `prikk tag create`, `prikk branch
create`, and `prikk branch close`"* — alongside `seal`, eight in total.

**That list is a fact with one true source: which functions call `verify_signer_trusted`.** I derived it
by hand twice this session — **eight surfaces once, and two-instead-of-three the other time.** A derived
list cannot be wrong that way.

**Nine call sites, two crates**: `prikk-cli` (`seal.rs` ×2, `tag.rs`, `branch.rs` ×2) and `prikk-store`
(`seal_from_accepted.rs`, `merge_execute.rs`, `tag_travel.rs`, `sync_negotiation/sender.rs`).

## 2. What this stage does NOT do — and the documentation must say so

**Two different claims live near each other. This stage binds one.**

- **(i) "These are the operations that gate."** A declared, enumerable set — **bindable, and this
  stage's target.**
- **(ii) "Every operation that ought to gate does."** **Not bindable by this stage, and it is the claim
  that actually bit us**: `prikk tag create` published maintainer-signed objects for months *without
  calling the gate at all*. **No enumeration of gated operations can catch an operation that is
  absent from it.**

**Do not let the documentation imply (ii).** After this stage the page may say the list is derived and
cannot drift; **it must not say prikk guarantees every publishing operation is gated.** That would be
exactly the overstatement `MILESTONES` criterion 2 had to withdraw.

**If you see a way to bind (ii), report it — do not build it here.** It would be a larger and more
valuable increment.

## 3. The design: make the operation a declared value

**Give `verify_signer_trusted` an operation parameter** — an enum in `prikk-store`, used by both crates:

```rust
pub enum GatedOperation { Seal, Merge, SyncBuild, SyncSeal, SyncAdoptTag, TagCreate, BranchCreate, BranchClose }
```

**Why an enum rather than scanning source:** the set becomes **a declared type the compiler knows**, not
a pattern a regex infers. **Source-scanning would be the "second parser is a second copy" shape** this
RFC rejects — and it is brittle against formatting besides.

**Each call site names its operation.** `seal.rs`'s two sites are both `Seal` (ordinary seal and
signer-backed recovery) unless you find they are genuinely different acts — **say which you chose and
why.**

## 4. The binding

**A `#[test]`, colocated, reading the enum** — the stage-2 pattern and Gate A's:

- **Every `GatedOperation` variant is named in `trust-threat-model.md`.**
- **Every operation the page names as gated is a real variant.**

**Bidirectional, as RFC 118 §8 requires.** The page's prose stays authored; only the *list* is bound.

**Where the test lives is your call** — `prikk-store` has the enum; the page is a repo file reachable
by `CARGO_MANIFEST_DIR`, as `format_stability_gate.rs:49` already does. **Say where you put it.**

## 5. Out of scope

- **Changing any refusal message.** An operation identifier makes a better message possible — *"tag
  create refused: signer not trusted"* — **but that changes user-facing output**, and this stage must
  not. **Report it as a follow-up if you agree it is worth doing.**
- **Adding a gate to any operation that lacks one** (§2). If you find one, **report it** — that is a
  security finding, not a refactor.
- **`boundary-check`, `release-policy`.** This is a test, like stage 2.
- **The command registry.** Untouched.

## 6. Controls

1. **The list-binding fires**: add a variant without documenting it; observe the failure; revert.
2. **The reverse fires**: name a non-existent operation in the page; observe; revert.
3. **Behaviour is unchanged**: **1321 tests pass**, and the three trust-gate refusal tests from
   `053e442` plus the five from `aa1b25d` still pass **unmodified** — they are the behavioural control,
   since every call site is being edited.

**Quote every failure.**

## 7. What to report

1. **The enum**, and the `seal.rs` two-site decision (§3).
2. **Where the test lives** (§4).
3. **The documentation wording**, and confirmation it does **not** imply (ii) (§2).
4. **All three controls, quoted** (§6).
5. **Any operation you found that publishes maintainer-signed objects without gating** (§5) — report only.
6. **Full gate set against the exact commit, after the last edit.** Test counts rise by the new tests.
7. Anything here that was wrong, **including my nine-call-site count and the eight-operation list**.

**Stop and escalate, do not guess**, if: the two `seal.rs` sites turn out to be genuinely different
operations and naming them both `Seal` would hide something; threading the parameter through
`prikk-store`'s internal callers forces a public API change beyond the enum; or **you find an operation
that should gate and does not** — that stops this stage.
