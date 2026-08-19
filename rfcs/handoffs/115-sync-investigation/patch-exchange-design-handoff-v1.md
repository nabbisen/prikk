# RFC 115 — patch-level exchange interface: design handoff v1

**RFC:** `rfcs/accepted/115-sync-investigation.md` (ACCEPTED 2026-08-19). **Read §2 and §5.1 in full
before starting.** This handoff does not restate them.

**CORRECTED 2026-08-19 — Checkpoint 2 is withdrawn from the dev team and returns to the architect.**

**As originally written this handoff was out of role, and the error was the architect's.** The
architect's own operating instructions state that *"the Handoff must be specific enough that the
mid-capability model can implement the work without inventing important product or architecture
decisions"* (line 176), and name the architect **design authority** (line 5). The dev team's
instructions say to **escalate anything architectural** (line 284) and list *"continue by guessing when a
material design decision is unresolved"* among the things not to do (line 332).

**Checkpoint 2 as drafted asked them to invent exactly those decisions** — where an accepted-but-unsealed
patch lives, the object shape for block recognition, and the resolution of the DC-78 question. **That is
the architect's to decide, not theirs to propose.**

**Checkpoint 1 was correctly scoped and stands**: investigation, code-level findings, options with
consequences, and challenges to the questions themselves. It produced a finding that changed the design
space (`parent_patch_ids` has never been populated), which is exactly what an investigation is for — and
the dev team's report stayed in role despite this handoff's framing, giving options and asking for
rulings rather than presenting a settled design.

**What remains for the dev team: nothing, until the architect issues the design.** The implementation
handoff will follow from it, per §176's standard.

---

**Original framing, retained so the correction is legible:** *"This asks for a design, not code, and its
first stage is answering §2's open questions rather than proposing a shape. Report before designing, then
report before implementing."* **The first half was right; the second was not.**

## 0. What is settled, and must not be re-opened

- **The exchange unit is the Patch.** Blocks do not travel as the carrier (owner ruling, §2.2).
- **Block recognition travels as a claim** about patches, not as the thing carrying them (§2.2, §4).
- **Divergent blocks are accepted, not arbitrated.** Patch id is the global identifier; block id is
  local publication detail; a patch-set digest restores one-hash comparison (§2.7).
- **Transport is out of scope here** (§3, four options open). **A design that presumes a transport has
  exceeded this handoff** — the interface must be carriable by a file, a file plus a basis, SSH with
  prikk on both ends, or a protocol, without change.
- **§5.1's test and security discipline is binding**, and §7 below makes it concrete.

## 1. What already exists — build on it, do not rebuild it

Verified during the investigation; re-derive anything you intend to rely on, but do not start from
scratch:

- `PatchPayload` carries **`parent_patch_ids`** (`payload/patch.rs:57`) — patches already form a DAG
  independent of blocks — and **`preconditions`** (`:61`), so a patch already states what it requires to
  apply.
- Patches are content-addressed and AUTHOR-signed; **their key material already travels** (DC-53 Stage 2).
- `bundle export`/`import`, the `remotes/` received namespace, and merge-from-received-ref (DC-85) all
  exist and are tested end to end.

## 2. Q1 — the unit and its closure

**What travels when one patch is exchanged?** At minimum the patch, the transitive `parent_patch_ids`
the receiver lacks, and the author key material needed to verify it.

**Answer concretely:** does the closure include Blobs the operations reference (it must, or the patch is
unapplyable), and what bounds it? **DC-86's reasoning applies**: a declared count or size over the limit
must cost no more than reading one integer to reject.

## 3. Q2 — where an accepted-but-unsealed patch lives

**This is the one item with no existing home, and it is the question most likely to change the design.**

The WAL is the receiver's **own** active work. A received patch is not that: it is verified, not yet
sealed, and not authored here. **Report the options with their consequences** — a third container, a
received-patch area beside the received-ref index, or immediate sealing on acceptance — and say which
you recommend and why.

**Do not decide this in passing.** It determines what `verify` must say about such a patch, what
`doctor` sees, and whether a crash mid-acceptance is recoverable.

## 4. Q3 — the DC-78 collision, which is the one I most want challenged

DC-78 rules that a receiver's claim is *"sealed by a Maintainer key you adopted."* **A received patch
has an AUTHOR signature and no maintainer seal**, so that rule does not cover it.

**My reasoning, offered to be argued with rather than implemented on trust:** the receiver's own
maintainer seals what it accepts, which *preserves* DC-78 rather than weakening it — every block in your
repository is still sealed by someone you trust, because you sealed it. **This is also how Darcs and
Pijul work.**

**What I want from you:** confirm or refute it against DC-78's actual text and the code, not against my
summary. **If accepting a patch requires a maintainer act, say what triggers it** — is acceptance itself
the seal, or are the two separable, and what is the state in between? If you conclude my reasoning is
wrong, that is the more valuable answer.

## 5. Q4 — block recognition as a claim: the object shape

**"Travels as a claim" is a principle, not a design.** The receiver must learn *which patches the sender
sealed into which block, under which maintainer key* — an assertion **about** patches, separable from
them.

`payload/attestation.rs` already carries a claim about a block as its own signable object, and refs
already carry `required_attestation_ids`. **RFC 110 §4.1 and RFC 113 §4.1 both reached for this shape
for the same structural reason: sealing an unverifiable claim into content makes hearsay look verified.**

**Decide whether this reuses attestation, extends it, or needs its own object** — and if a new object
type, remember it becomes a frozen `(object_type, schema_version)` pair under RFC 114 the moment it is
first written.

## 6. Q5 — the patch-set digest

A canonical hash over the sorted set of patch ids reachable from a ref, so *"are these two repositories
the same?"* has a one-hash answer at the level where identity actually holds (§2.7).

**Follow prikk's own precedent for identity:** an explicit domain-separated preimage, not a hash of
whatever serialization happens to be in use (`prikk-object/src/id.rs`, and RFC 114's ruling on exactly
this). **Say whether the digest is identity-bearing** — if it is ever persisted or compared across
versions, RFC 114's frozen surface applies to it.

## 7. The test and security deliverables — named, not inferred

§5.1 is binding; these are its concrete artifacts for this increment:

1. **A written threat model, before the design is finalised.** §5.1.2's refusal list is its starting
   point, not its conclusion. **A peer that is syntactically perfect and semantically dishonest is the
   adversary to model** — fuzzing does not reach it.
2. **Adversarial fixtures committed as bytes**, per RFC 114's approach. A hostile artifact reconstructed
   at test time from today's encoder proves the parser accepts a shape, not that the shape occurs.
3. **A two-repository harness** as a first-class thing, extending `dc78_bundle_exchange.rs` and
   `dc85_merge_from_received_ref.rs` rather than sitting beside them.
4. **A negative control per refusal in §5.1.2** — disable the check, watch the specific test fail,
   restore. **A refusal nobody has seen fire is not evidence.**
5. **Partial-failure coverage** using the existing `failpoints.rs` `Point`/`TestBarrier` machinery: an
   interrupted acceptance must leave the receiver sound, and **must not leave material behind in a
   container that has no prune, no compaction and no repair** — DC-53 Stage 2 already proved that state
   is reachable.

## 8. Out of scope

- **Transport** (§0).
- **The `merge_execute` fast-forward gap.** Real, separable, and worth fixing regardless of sync — it is
  not this increment's, and folding it in would blur what this design is judged on.
- **Canonical sealing** (§2.6). Recorded as declined with a stated reason; reopening it is an owner
  decision, not a design one.
- **Any new compatibility promise.** RFC 114's contract governs anything this adds.

## 9. What to report, and when

**Checkpoint 1 — before designing:** Q1-Q5 answered, the threat model drafted, and **any question you
believe is the wrong question**, with why. §4 in particular invites disagreement.

**Checkpoint 2 — WITHDRAWN.** The design is the architect's, written from Checkpoint 1's answers. An
implementation handoff follows it.

**Take the time this needs.** The interface decided here is hard to reverse: it determines what two
prikk repositories can ever say to each other, and every transport built later inherits it.
