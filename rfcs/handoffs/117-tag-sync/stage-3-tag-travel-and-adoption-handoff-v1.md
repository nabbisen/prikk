# RFC 117 stage 3 — tags travel, and the receiver adopts them: implementation handoff

**Design:** `rfcs/handoffs/117-tag-sync/design-v1.md` **T3 and T4. Read T4 in full — it is the ruling
most likely to be implemented backwards, because the convenient thing to build is the wrong one.**
**Base:** current `main` (`335d658`). **This is the last increment RFC 117 needs.**

---

## 1. What this delivers

A tag created on one repository can be adopted on another: it travels in the exchange artifact, the
receiver resolves its patch set to a local block, and **the receiver signs its own tag**.

Three parts: the artifact carries Tag objects (§2), accept stores and reports them (§3), and a separate
explicit command adopts one (§4).

## 2. The artifact gains a tag section — and that is a format revision

Add a **Tag section** to the exchange artifact, alongside patches, blobs, author key material and
claims. The artifact is **representational** (RFC 114 §3), so this is permitted — but it is still a
format change and must be versioned as one.

**Ruled: `PEXCH001` → `PEXCH002`.** Old artifacts are **refused**, not migrated: an artifact is
transient, in-flight data, not stored history, and under the standing "no production" ruling there is
nothing in flight to preserve. **State that in the module doc** — a reader should not have to infer why
one format is versioned and the other is broken.

**Which tags travel: every tag whose target block lies within the ancestry of the ref being synced.**
Enumerate `tags/*` via `RefStore::list_ref_pointers`, resolve each to its Tag object, and include it if
its `target_block_id` is in the closure the sender already walks. Sending a tag the receiver already
holds is harmless — objects are content-addressed and accept is idempotent — so **do not try to send
only what is missing**; the receiver cannot say what it lacks without resolving, and asking it to would
add a round trip for no gain.

**The stage 7 cross-platform fixture must be regenerated**, because it is a committed `PEXCH001`. Its
own handoff anticipated exactly this: *"when the artifact format legitimately changes, the fixture is
regenerated and the change reviewed."* **This is that case.** Use the `#[ignore]`d regenerator that
already exists; do not hand-edit bytes; and say in your report that you regenerated it and why.

## 3. Accept stores and reports; it does not adopt

A received Tag object is written like any other carried object, and its signature outcome is **reported,
never gating** — the same treatment a recognition claim gets. An unadopted signer's tag reads
`Unverifiable`, is visible, and **does not refuse the exchange**.

`sync accept` should report received tags the way it reports claims, so an operator can see what
arrived.

## 4. Adoption is the receiver's own signed act — this is the ruling to get right

**Sync does not mint tags.** T4 is explicit, and the convenient implementation is the wrong one: it
would be easy to have `accept` create a local tag automatically, and that is precisely what must not
happen.

A tag is **a signed assertion about your own repository**. Conjuring one on your behalf from someone
else's assertion would be the one place in this design where a signature does not mean what it says.
Sealing is already a separate explicit act (D5) for the same reason.

**So: a separate command.**

```
prikk sync tags                          # list received tags with their resolution state
prikk sync adopt-tag <name> [--signer]   # resolve, then create a LOCAL tag under the receiver's key
```

- **`sync tags`** — for each received Tag: its name, its signature outcome, and its resolution state
  from stage 2 (`Resolved(block)` / `NotHeld` / ambiguous). **`NotHeld` is ordinary here** — you have
  not synced that far — and must read as information, not failure.
- **`sync adopt-tag`** — resolve the received tag's `patch_set_digest` **and `patch_count`** (stage 2a's
  signature) to a local block, then create a local tag naming **that local block**, that same digest and
  count, signed with the receiver's own maintainer key via the ordinary tag-creation path.
- **Refuse to adopt** when resolution is `NotHeld` or ambiguous. Ambiguity refuses by naming candidates
  — stage 2 already does this; do not soften it here.
- **`verify_signer_trusted` before signing**, as `prikk tag` already requires.

**The sender's tag and the receiver's tag are different objects with the same global identity.** That is
the same relationship their blocks already have, and it should be said in the module doc, because "the
tag ids differ" will otherwise read as a bug.

**Naming is the owner's to override** — `sync tags` / `sync adopt-tag` are my choice, not a ruling. Say
in your report whether they read naturally once you have used them.

## 5. Security properties, as refusals

| # | Property | Control |
|---|---|---|
| 1 | A received tag adopts no key and creates no local tag by itself | Accept an artifact with a tag → assert no local `tags/*` ref appeared and the adopted-key set is unchanged |
| 2 | Adoption refuses when the patch set is not held | `NotHeld` → refusal, no tag written |
| 3 | Adoption refuses on ambiguity, naming candidates | Two blocks, one patch set → refusal |
| 4 | An `Unverifiable` received tag is reported, not refused | Tag from an unadopted signer → accept succeeds, outcome visible |
| 5 | The adopted tag is signed by the **receiver's** key, not the sender's | Assert the local tag's `author_key_id` is the receiver's |
| 6 | A refused exchange records no tag | Force a Phase-C failure with a tag present → no Tag object written |
| 7 | `PEXCH001` is refused by the new reader | Feed the old fixture bytes → refusal naming the format |

**Row 1 is the one that proves T4.** It must assert *absence* — no ref, no tag object adopted into local
refs — not merely that some report field says nothing happened.

**Row 5 is the one that proves the signature means what it says.**

## 6. Out of scope

- **Tag deletion or movement across repositories.** Design §7; create-once locally, and making removal
  travel is its own question.
- **Discovery of a counterpart's tags without an exchange.** Remote-tracking's territory.
- **Changing `TagPayload`.** Stage 2a settled it. **If you find it short a field, stop and escalate** —
  that has happened three times in this arc and the fourth should be a decision, not a fix.
- **The four ref-tip resolution copies.** Still recorded, still not this. **Do not add a fifth.**

## 7. What to report

1. Control output for every row of §5 — actual failure text, and the single line mutated.
2. **Confirmation that the stage 7 fixture was regenerated with the existing regenerator**, and that the
   cross-platform test still passes against the new bytes.
3. **For row 1:** exactly what absence you asserted.
4. Whether §4's command names read naturally in use.
5. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
6. Test counts before and after, per crate. **`snapshot.txt` must not change** — no payload type or
   schema changes here.
7. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: `TagPayload` turns out to need another field (§6); the tag
section forces a change to how claims or patches are framed; or an end-to-end adopt cannot be driven
through the CLI alone — **that last one would mean the surface is incomplete, as it did in RFC 116.**
