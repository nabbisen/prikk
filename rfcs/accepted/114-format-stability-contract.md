# RFC 114 — The format-stability contract

**Status.** **ACCEPTED by the project owner 2026-08-19.** Answers **badge criterion 2**, scheduled by the
owner the same day as the next work. **Acceptance clears the direction, not the design** — §5's five
decisions precede any implementation, and **§5.2 and §5.3 are the owner's**, not the architect's: they
are support commitments, not technical rulings. **Independence:** author-reviewed, the standing ceiling; every claim below cites the code or
the record it came from.

**The criterion, in its own answerable form:** *what minimum must never change for a verification claim
made today to hold in ten years.*

## 1. prikk already has an answer. It is informal, unstated, and it failed yesterday.

The de-facto policy is **migration by bundle**: `layout.rs` refuses every retired repository format with
*"to migrate: use a prikk version that supports format N to `prikk bundle export`, then `prikk bundle
import` here"* — **five occurrences, one per retired format**.

**That path was severed on 2026-08-18** by DC-53 Stage 2's `PBNDL001` → `PBNDL002` bump, which made
current prikk refuse the bundles those older versions produce. Every repository below format 6 became
unmigratable, and the refusal advised *"re-export with a current prikk build"* — impossible, because a
current build cannot open the repository at all
(`handoffs/DC-53-repository-wide-author-trust-verification/bundle-v1-import-regression-v1.md`).

**The defect is being fixed. The reason it happened is this RFC's subject.** There was no stated promise
to violate, so nothing detected the violation: not review, not nine gates, not CI. **An unstated
compatibility policy cannot be broken, only discovered to have been absent.**

Two further facts from the code, both consequences of the same gap:

- **A `Block` with `schema_version != 2` is an `Integrity` error**, not an unsupported version
  (`block_state.rs:455`). It is unreachable only because the repository-format gate refuses first.
- **`dc55_identity_evidence.rs` already documents the shape of the problem**: its format-2 fixture is
  *"permanently unopenable through `RepositoryLayout::open`"*, so the module asserts identity by reading
  bytes directly. **The project already keeps historical evidence and already works around its own
  unreadability** — without a rule saying what that evidence must guarantee.

## 2. The distinction the answer turns on

**Not everything in a repository participates in a verification claim.** Two categories, and they need
opposite rules:

**Verification-bearing — what a claim made today depends on:**

- The object-id preimage: `OBJECT_ID_DOMAIN` (`b"PRIKK-OBJECT-ID-v1"`) ‖ type code (u16 BE) ‖
  `schema_version` (u32 BE) ‖ payload length (u64 BE) ‖ canonical payload, hashed **SHA-256**
  (`id.rs:110-122`).
- The canonical encoding of **each `(object_type, schema_version)` pair that has ever been written**.
- The signature preimage, per `SignerRole` (`signature.rs`'s `signed_bytes`).
- The algorithms themselves: Ed25519 verification, SHA-256.

**Representational — how those bytes are stored and moved:**

- Repository format version and directory layout; container framing; the object index; the WAL; the
  bundle format.

**RFC 111 already used this distinction correctly in a smaller place** — *"the bundle format is
transport, not identity"* — and DC-53 Stage 2 relied on it to bump `PBNDL002` without touching an object
id. **The distinction is sound and already load-bearing. What is missing is that it is written down
anywhere, and that anything checks it.**

## 3. The proposed contract

**Verification-bearing bytes are frozen forever. Representation may change, and must carry a tested
migration path.**

Stated as a promise a user can rely on:

> **Any prikk release can read every object any prior release wrote, and verifies it to the same
> conclusion. Storage may require a migration step, which is documented and tested. Object identity and
> signatures never require one.**

**Freezing is not "never add a field".** `schema_version` is *inside* the id preimage, so a new field
means a new schema version, new ids for new objects, and **no change whatsoever to objects already
written**. DC-75 did exactly this (`Block` 1 → 2) and `RefState` already carries two versions at once.
**The obligation is not to stop evolving — it is to keep every version ever written decodable, forever,
and to keep its bytes hashing the way they did on the day they were written.**

**The corollary that would have prevented yesterday:** *read what the past wrote; write only the
present.* Import accepts old formats; export emits current. **prikk already applies this to repository
formats and did not apply it to bundles.**

## 4. What "and its answer honoured" requires — the gate

A promise with no check is the state we are already in. Two gates, and the second is the one with teeth:

1. **Frozen identity vectors, per `(object_type, schema_version)` ever written.** DC-40's literal-vector
   precedent, generalized: committed bytes plus their expected object id and signature preimage. Any
   change to §2's verification-bearing list breaks a vector. **Cheap, deterministic, and it belongs in
   the ordinary suite.**
2. **A migration conformance test per historical repository format.** A fixture at each retired format,
   carried through **the documented migration path**, ending in a repository that opens and verifies.
   **This is the gate that yesterday's defect would have failed**, and it is the one that costs
   something: it requires the path to be executable in CI without old binaries — which means keeping
   **old bundles as byte fixtures**, since a `PBNDL001` bundle is just bytes.

`dc55_identity_evidence.rs` is the existing precedent for both, and the design should extend it rather
than start beside it.

## 5. What a design must decide

1. **Where the contract is published.** `docs/src/reference/release-compatibility.md` is the obvious
   home, and it currently states the *release* boundary rather than the format promise. A promise users
   cannot find is not a promise.
2. **What the migration path actually is, now that bundles are the only one.** Bundle export/import is
   load-bearing for every format transition and was, until this week, unowned. **Does it stay the
   mechanism, and if so does it become a first-class supported operation with its own tests, rather than
   a sentence in an error message?**
3. **How far back "forever" reaches.** Formats 1 and 2 are already unopenable and their migration path
   requires binaries nobody keeps. **Decide honestly whether they are supported or abandoned** — an
   abandoned format that the error message pretends is migratable is worse than a stated end of support.
4. **Whether `schema_version` gaps are permitted.** If a version is written by a pre-release build and
   never shipped, is it owed forever-decodability? The rule should name shipped releases, not every
   commit that existed.
5. **What happens on a hash or signature algorithm break.** SHA-256 and Ed25519 are frozen by §3 — but
   "frozen" cannot mean "we have no plan if one is broken." The answer is likely a new algorithm
   identifier alongside, never a redefinition of an existing one, **and it should be stated before it is
   needed rather than under pressure.**

## 6. Non-goals

- **Not a promise that every release opens every repository in place.** Migration steps are permitted;
  undocumented or untested ones are not.
- **Not a freeze on representation.** Containers, the index, the WAL and the bundle may all evolve.
- **Not a 1.0 stability commitment.** This is the *minimum* that must hold for verification to mean
  anything across time; product-surface stability is a separate question.
- **Not retroactive rescue.** Whether formats 1 and 2 can still be migrated is §5.3's decision, and the
  honest answer may be no.
