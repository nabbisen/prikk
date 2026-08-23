# Consolidating the ref-tip resolvers: implementation handoff

**Investigation:** `.git-exclude/reviewed/ref-tip-resolution-consolidation-investigation-v1.md`.
**Base:** current `main` (`faa4d39`).

**A behaviour-preserving refactor, and that is the whole specification.** No new capability, no error
that did not exist before, no test that needs editing. It exists because "do not add a fifth" now
appears in six handoffs, and an instruction repeated that often should be replaced by the thing it is
policing.

---

## 1. What is being consolidated — three, not four

| # | Site | Today |
|---|---|---|
| 1 | `bundle.rs:656` `resolve_ref_target_block` | the two-hop, **plus** pushing the Tag envelope into a caller-supplied accumulator |
| 2 | `patch_set_digest.rs:123` `resolve_ref_to_tip_block` | ref-name lookup **and** a `remotes/*` refusal **wrapped around** the same two-hop |
| 3 | `patch_exchange.rs:49` `resolve_ref_tip_block` | the two-hop, plain |

**#1 and #3 are the same function.** #2 is that function under a layer of its own.

**`refs/verify/scan.rs:405` `ensure_ref_target_valid` is NOT in scope — see §3.** It looks like a fourth
copy and is not one.

## 2. The shared core

```
pub(crate) fn resolve_ref_tip_block(
    object_store: &impl ObjectReader,
    ref_state_payload: &RefStatePayload,
) -> Result<(ObjectId, Option<ObjectEnvelope>)>
```

`Branch` → `(target_object_id, None)`. `Tag` → read the Tag, decode, `(target_block_id, Some(envelope))`.

**Site it in the `refs` module**, re-exported from `refs.rs` beside `ensure_ref_target_valid` — they are
the two halves of the same subject and should be found together. Do not put it in any of the three
callers; that is how a fourth copy happened.

**Rewiring:**

- **#3** becomes a call to it.
- **#1** calls it and pushes the returned envelope. **The accumulator stays in `bundle.rs`** — do not
  move a `&mut Vec` parameter into the shared function to accommodate one caller.
- **#2** keeps its ref-name lookup and its `remotes/*` refusal, and calls the core for the two-hop only.

**Nobody pays for the envelope.** #2 and #3 already decode it to reach `target_block_id`; returning it
costs nothing and they discard it.

## 3. What must NOT be folded in, and why

**`ensure_ref_target_valid` stays exactly as it is.**

- It **validates**, and returns `()`. For a `Branch` it checks the **block exists** — which none of the
  three resolvers do.
- Its errors carry **`owner`**, the id of the ref object being verified, which a resolver never sees.
- **That message is load-bearing.** *"ref object {owner} targets missing tag {target_object_id}"* is the
  message the DC-78 tag-gap ruling turned on: the reason a bundle must ship the Tag object is that
  omitting it produces exactly this from `verify`. Degrading it to a generic resolver error makes the
  next person's diagnosis strictly harder.

**Add a short comment on both sides** — resolver and validator — naming the other and saying why they
are separate. That analysis has now been re-derived four times; write it down once.

## 4. Message unification, and the one thing to check first

`bundle.rs` uses its private `read_required`, whose message renders as `missing tag object: {id}`
(lowercase, via `ObjectType`'s `Display`). The other two write `missing Tag object: {id}`. **After
consolidation there is one message, so one of these changes.**

**Verified before writing this: no test asserts either string.** The only matches are production sites
emitting it, plus one test *helper* constructing its own unrelated error.

**Check it yourself anyway before relying on it. If any test does assert one of these strings, stop and
report it — do not edit the test to match.** A refactor that requires editing an assertion is not
behaviour-preserving, which is exactly what §5 is about.

## 5. The control — this refactor is unusual in that its control is its own test suite

**Every existing test must pass unchanged.** No test added, none edited, none deleted.

**That is the whole control, and it is a strong one here:** a behaviour-preserving refactor whose tests
need editing is not behaviour-preserving. If something has to change, the change is the finding —
report it rather than accommodating it.

The three call sites are covered today by `bundle/tests.rs`, `patch_set_digest/tests.rs` and
`patch_exchange`'s own suites, including the tag two-hop path in each. **You are not asked to add
coverage; you are asked not to disturb it.**

**One mutation control is still worth running**, to prove the shared core is actually reached rather
than dead code sitting beside three untouched copies: break the `Tag` arm in the shared function and
confirm failures appear in **all three** callers' test suites, not one.

## 6. Out of scope — including one adjacent finding, recorded not folded in

- **`ensure_ref_target_valid`** (§3).
- **A separate, smaller duplication found while investigating:** five sites read a required Tag object
  by id with the same `read_typed(..).ok_or_else(..)` shape and the same message —
  `tag_travel.rs:207,375`, `sync_negotiation/sender.rs:181`, and the two inside the resolvers. `bundle.rs`
  already has a private generic `read_required` that would serve all of them. **This is a different
  pattern from ref-tip resolution — a one-hop read, not a two-hop resolve — and folding it in would
  broaden the diff and weaken §5's control.** Recorded here so it is not lost; its own increment if ever.
- **Any behaviour change at all.** If you find one that seems desirable, it is a separate increment.

## 7. What to report

1. **Confirmation that no test was added, edited or deleted**, and the workspace total before and after
   (they must match).
2. §4's check: whether any test asserts either message form, and what you found.
3. §5's mutation control: breaking the shared `Tag` arm fails **all three** callers' suites — name them.
4. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
5. **`snapshot.txt` must not change.**
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: any existing test needs editing (§5); siting the core in `refs`
drags something across a module boundary; or a fourth genuine ref-tip resolver turns up that this
investigation missed.
