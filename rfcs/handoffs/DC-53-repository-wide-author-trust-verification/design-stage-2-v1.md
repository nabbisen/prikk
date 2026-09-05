# DC-53 Stage 2 — transport and pinning, design v1

**RFC:** `rfcs/done/DC-53-REPOSITORY-WIDE-AUTHOR-TRUST-VERIFICATION.md`
**Stage 1 design:** `design-v1.md` (amended v1.1, v1.2). **Stage 1 merged 2026-08-18 at `970bc27`.**
**Started on the owner's direction, 2026-08-18** ("Start DC-53 Stage 2"), after criterion 3 closed.
**Independence:** author-reviewed, the standing ceiling.

## 1. What Stage 1 left, established from code

- `verify` cryptographically checks every reachable Patch's AUTHOR signature against recorded key
  material. A signature that fails against recorded material **fails `verify`'s exit status**; a Patch
  whose `key_id` has no recorded material is reported **Unverifiable**, and `verify` still passes.
- **Key material is recorded at authoring time only** (`author_signing.rs`'s path), and **does not
  travel**: `bundle.rs` references neither `author_key_container_path` nor `author_key_index`.
- `import_bundle` writes objects and records a received pointer. **It verifies no signature at all.**
- Consequence, verified: **a Patch received from another party is permanently `Unverifiable`.** Import a
  bundle today and `verify` will report every imported Patch unverifiable, forever, on every future run.

**That is the whole of what Stage 2 must close, and it is what keeps badge criterion 5 open.**

## 2. The honest limit, stated before the mechanism

Transported key material is **supplied by the sender**. A signature verified against a key that arrived
in the same bundle proves only that the two are consistent with each other — an attacker who re-signs a
Patch with their own key and ships that key produces a bundle that verifies perfectly.

**So transport alone proves nothing. Pinning is what makes it worth anything**, and what it buys is
precisely this: *the same `key_id` always carries the same public key*. First contact is trust-on-first-
use and is **not** verified; every subsequent appearance is.

**This must be written into `docs/src/reference/trust-threat-model.md`, not left in this document.** A
reader must be able to tell "prikk verified this author" from "prikk verified this author is the same
one as last time," because only the second is true.

**Nothing here weakens DC-78's existing exchange claim.** A receiver still relies on the maintainer
signature for the decision to include those patches; author verification adds continuity of authorship
on top, and does not replace it.

## 3. D6 — Transport: a new bundle section, and the format version must bump

**The bundle format has no room for this and cannot be extended silently.** `decode_bundle` ends with:

```rust
if !cursor.is_finished() {
    return Err(PrikkError::MalformedData("trailing bytes in bundle".to_string()));
}
```

An appended section is therefore **impossible** without a version change — an older prikk would reject
the whole bundle as malformed rather than ignore the extra bytes. **Ruled: bump `BUNDLE_MAGIC`
`PBNDL001` → `PBNDL002`** and carry an explicit author-key section.

**This is fail-closed and that is why it is acceptable**: an old client meets a clear "unsupported bundle
version" refusal instead of importing history whose authorship it silently cannot check. State the
refusal message and the pre-1.0 compatibility position in the same change
(`docs/src/reference/release-compatibility.md` already owns that boundary).

**Scope of the section: exactly the authors of the Patches in the bundle.** Not the whole local
container — that would leak every author this repository has ever seen to every recipient, which is a
disclosure the sender did not choose. **Derive the set from the exported objects, and say so in the
code**, because "export everything we have" is the easy wrong implementation.

## 4. D7 — What import does with the material

**Import records material; `verify` decides.** Keeping the judgement in `verify` preserves Stage 1's
shape, where `verify` is the only thing that renders a verdict, and keeps `import` a transport step.

**But import must reject a conflicting key rather than record it** — see D8. An import that silently
appends a second key for an existing `key_id` would hand `verify` an unresolvable state and, under
Stage 1's current rule, would make a forged signature Sound.

## 5. D8 — Pinning, and the Stage 1 rule this must change

**Stage 1 deliberately made the lookup permissive.** `verify_author_signature` today:

```rust
let verifies = entries.iter().any(|entry| verify_ed25519(&entry.public_key, &preimage, &sig).is_ok());
```

Every entry ever recorded for a `key_id` is accepted, and `record_author_key_material` appends a
conflicting key rather than refusing it. That was correct for Stage 1 — it had no conflict semantics —
and it is **exactly the exposure the Stage 1 review recorded for Stage 2 to close**: anyone able to
author can append material under any `key_id` and make a forged signature Sound.

**Ruled: one `key_id` binds to one public key. A second, different key for a recorded `key_id` is an
authorship-integrity failure** — D3's fourth row — and fails `verify`'s exit status, the same as a
signature that does not verify.

**Migration, and it is not hypothetical.** Stage 1 shipped permitting multiple keys per `key_id`. Any
repository that recorded two will begin failing when Stage 2 lands. **Report what exists before changing
the rule**: scan for `key_id`s with more than one recorded key, in this project's own fixtures and in any
real repository available. If none exists, say so and the migration is empty. **If any exists, stop and
report** — silently failing repositories that Stage 1 told were fine is not acceptable, and the answer
would be a decision, not an implementation detail.

**Rotation remains unimplemented and indistinguishable from impersonation** (D5, unchanged). Stage 2
makes that consequence real rather than theoretical, so D5's documentation obligation lands here.

## 6. D9 — Why authors get transport and maintainers do not

MAINTAINER key material does not travel either — `bundle.rs` has zero references to `trust_index` — and
a receiver adopts a maintainer key deliberately. **That asymmetry is intended and must be justified in
the design, not just inherited**: adoption is an admission decision a human makes about a small, stable
set; authorship is an observation about a large, growing one. Requiring per-author adoption would turn
`bundle import` into an administrative task, which is what D1 rejected.

**The consequence to state:** a maintainer key means *"someone decided to trust this."* An author key
means *"this is the key that has always signed under this name here."* Different claims, different
strength, both useful — and the CLI's own wording should not blur them.

## 7. Vectors

`design-v1.md` §4's **vector 6** (a pin-conflict pair: one `key_id`, two public keys) is Stage 2's, and
it should be a committed literal like vectors 1-5. Add:

- **Vector 7** — a bundle whose author-key section omits a key for a Patch it contains. Must import (the
  material is optional per-author) and the Patch must read **Unverifiable**, not Sound and not a failure.
- **Vector 8** — a bundle whose author-key section carries a key that does **not** verify the Patch's
  signature. This is the transport-layer forgery case and must fail.

## 8. Staging within Stage 2

Two reviewable steps, both required before criterion 5 can be reassessed:

- **Step 1 — pinning. COMPLETE, merged 2026-08-18 at `27088c9`.** D8's one-key-per-`key_id` rule
  enforced at record time and again at verify time, the migration scan (empty), D5's documentation,
  vector 6. The Stage 1 exposure is closed: nobody able to author can append material under another
  `key_id` to make a forged signature Sound. **A lock-ordering hole found in review was fixed with it** —
  `rollback_draft` recorded key material outside `ActiveLock`, so the check-then-append could race into
  the unrecoverable conflict state.
- **Step 2 — transport. THIS IS THE LIVE STEP.** D6's bundle version bump and author-key section,
  D7's import recording, vectors 7-8, and §2's threat-model text.

  **Three conditions carried from Step 1's reviews, binding on Step 2:**

  1. **Import must be structurally unable to write a conflicting key** into a receiver's container. A
     hostile or merely stale bundle must fail the **whole** import — never partially succeed into a state
     `verify` cannot recover from, because a conflicted `key_id` has no remedy (no prune, no compaction,
     no `doctor` repair) and fails `verify` permanently.
  2. **The deferred lock-ordering test lands here.** Step 1 fixed the ordering and shipped without a test
     for it, on the argument that a concurrency test would be indistinguishable from testing
     `ActiveLock`. **Import makes the container's third writer**, so the concurrency surface grows and the
     ordering now deserves a barrier-based race test — RFC 106's failpoint barriers are the precedent —
     **or a concrete statement of why that machinery cannot reach these call sites.**
  3. **State the honest limit in `docs/src/reference/trust-threat-model.md`**, per §2: transported
     material is sender-supplied, so what this buys is **continuity of authorship, not authenticity of
     first contact.** A reader must be able to tell "prikk verified this author" from "prikk verified
     this is the same author as last time." Only the second is true.

  **Note on that page:** it is stale beyond this obligation — it opens *"describes the released
  implementation through 0.16.0"* and still claims prikk *"does not currently implement a repository-wide
  AUTHOR trust store."* **The broad refresh is the architect's and is not yours**; add only the sentence
  §2 requires.

**Pinning first, deliberately.** Transport without pinning verifies nothing (§2), so shipping transport
first would add a mechanism whose only guarantee is not yet enforced. The reverse order leaves a useful
increment at every point.

## 9. Report before implementing each step

Per this project's standing shape. **Step 1's report must lead with the migration scan** of D8 — that
number decides whether Step 1 is a rule change or a rule change plus a decision for the owner.

## 10. What Stage 2 does not grant

- **No first-contact authenticity.** §2. A design that reads as though it provides this has failed.
- **No key rotation, revocation, expiration, thresholds, or hardware signing.** D5 unchanged.
- **No machine-to-machine sync.** Criterion 1 is untouched; this rides on the existing bundle.
- **No identity-bearing byte change.** Object ids, signature preimages and canonical encodings are
  frozen. **The bundle format is transport, not identity** — bumping it changes no persisted object.
