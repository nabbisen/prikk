# RFC 138 — Asking what a repository trusts

**Status.** **PROPOSED, 2026-09-06.** Opened at the project owner's instruction after the stikk
project asked for it (`.git-exclude/external-communication/stikk/receive/002-trust-listing-and-the-no-audit-flag.md`),
and the owner approved the architect's recommendation that it is worth doing.

**Author-review independence.** The architect wrote this and is its only reviewer — the standing gap
on every architect-authored design here.

**Tracks.** A read surface over state prikk already holds. **No change to what is trusted, how trust
is adopted, or what trust means.**

---

## 1. The question nobody can ask

`prikk trust maintainer` offers `add` and `remove` (`commands.rs:115-116`). **There is no way to ask
which keys a repository currently trusts.**

This bites a caller that must answer *"can I actually seal here?"* before starting a ceremony that
ends in immutable history. `seal` requires a MAINTAINER key adopted **in the repository's trust
policy**, not merely present in the environment — so seeing the environment is not seeing the answer.

**`prikk verify` looks like it answers this and does not.** It prints `sealed-block <id>: <key_id>`
per sealed block (DC-78 Stage 2 §D3). Two reasons that is not the same question:

- **It is historical signer attribution, not current policy.** A key that signed a block may have been
  revoked since; the line still prints.
- **It does not exist before the first seal.** An unsealed repository prints nothing — and that is
  exactly when the question is asked.

**So the gap is worst precisely where a caller meets it.**

## 2. The capability exists; only the surface is missing

`load_maintainer_trust_policy(layout) -> MaintainerTrustPolicy` (`trust.rs:214`) returns
`keys: Vec<AdoptedMaintainerKey>` — *"every currently adopted key, in the order it was adopted"* —
where each carries `key_id: String` and `public_key: [u8; 32]` (`trust.rs:54-59`).

**This RFC adds no read, no state and no policy. It exposes one that is already loaded on every
seal.**

## 3. What "trusted" means here — and one thing the CLI already prints that is not read

Two facts a read surface must not blur, both from `trust.rs:61-64`:

1. **It is object trust, not ref authority.** *"A `Block`/`RefState` is trusted if **any** adopted key
   signed it… adopting a key never lets it move a ref — `RefStore::publish` still requires a signature
   from this operator's own signer."* A listing answers "whose signatures does this repository accept
   on objects", **not** "who may publish here".
2. **There is no threshold.** `MaintainerTrustPolicy` holds a `Vec` and nothing else. Trust is
   any-of-N by construction.

**Found while writing this: `policy: required=1` is a hard-coded literal**, printed by
`main.rs:295` and `setup.rs:106`. It is *true* — any one adopted key suffices — but it is **printed as
if read**, and no such field exists. A read surface that reported it the same way would be inventing a
policy value. **Either derive it or stop printing it as policy**; this RFC must not add a third site
that repeats a constant in the voice of a query.

## 4. The option space

**(a) `prikk trust maintainer list`** — enumerate adopted keys. What the request asked for. Cheapest
to consume; commits us to enumerating a trust policy as a public surface.

**(b) `prikk trust maintainer check --key-id <ID>`** — answer one question, exit-coded. **The
requester offered this as a fallback and it may be the better shape**: it matches
`verify_signer_trusted`'s own question, it is what a ceremony actually asks, and it commits us to
nothing about enumeration. Its cost is that a caller wanting the whole picture must already know the
ids.

**(c) both.** They answer different questions and (b) is not a restricted (a) — one is *"what is
here"*, the other *"is this here"*.

**(d) machine-readable output, as a first-class question rather than a follow-up.** `verify --format
json` is the existing precedent. **The requester's highest-value standing ask across our whole CLI is
a machine-readable surface**, and adding a human-only listing now would create a second thing to
retrofit. Deciding it here costs a paragraph; deciding it later costs a format.

## 5. Threat model — checked, not assumed

**Nothing here is secret.** Adopted public keys were typed on the operator's own command line by
`trust maintainer add --public-key HEX`. A listing returns what that operator put there.

**No tension with repository anonymity** (settled: no repository identity, no peers, only artifacts).
That property is about repositories not *carrying* identity; it says nothing about an operator reading
their own local policy. The requester checked `trust-threat-model.md` before asking and read it
correctly.

**One thing to keep straight in any output**: §3.1's distinction. A caller who reads "trusted" as "may
publish here" has been misled by us, not by themselves.

## 6. What this RFC does not decide

Which of §4 (a)/(b)/(c), and whether §4(d) rides with it. **§3's `required=1` finding is a defect to
resolve either way** — it predates this RFC and is not caused by it.

**Out of scope:** any change to adoption, revocation, thresholds, multi-maintainer policy, remote
trust, or what `seal` requires. **Key management and rotation remain unimplemented and unscheduled**
(`IMPLEMENTATION-STATUS.md`), and nothing here moves them.
