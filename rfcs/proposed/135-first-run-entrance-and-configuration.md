# RFC 135 — The entrance: what a new user meets before anything works

**Status.** **Proposed.** Raised by the project owner 2026-09-04, on reading the `README.md` Quick
Start: *"They are generally unfamiliar with visitors. I doubt it makes visitor feel uneasy and brings
their withdrawal."*

**Deliberately unhurried.** The owner asked for careful design *"now and later with time we need"*, and
directed that security be considered seriously. **This RFC opens the problem and the option space; it
settles nothing.** The architect's first framing — a small key command — was rejected by the owner as
*"insufficient"*, and rightly: it sized the fix to a symptom the architect had personally hit rather
than to what a visitor meets.

**Tracks.** The first-run surface, and the configuration mechanism prikk does not have.

---

## 1. What a visitor meets today

`README.md`'s Quick Start, before anything happens: **four `export` lines carrying two opaque 32-byte
hex seeds**, then a `trust maintainer add` carrying **a third opaque hex value**, then `commit` and
`seal`.

**The values are correct** — the architect derived the public key from the sample maintainer seed and
it matches the README byte for byte. **Nothing is broken.** The question is entirely what it costs a
reader to get past.

## 2. It is not reproducible with your own key — and that is a capability gap, not a documentation one

The three hex strings are **not independent**: `--public-key` must be the Ed25519 public key *of*
`PRIKK_MAINTAINER_SEED`. Substitute your own seed and the trust entry silently stops corresponding.

**And prikk offers no way to recompute it.** There are **23 CLI commands and not one generates or
derives a key.** `prikk_crypto::Ed25519KeyPair::generate()` exists; **no CLI path reaches it.**

**First-hand evidence rather than supposition:** preparing 0.31.0's cross-version test, the architect
needed a maintainer public key for a seed and had to write a throwaway Rust crate depending on
`prikk-crypto` by path to print one. **If that is the cheapest route available to the project's own
architect, a visitor has none.**

**So the Quick Start is take-it-or-leave-it**: copy all of it exactly, or supply your own values and
be unable to complete step three.

## 3. Twelve variables, three separable concerns

Every `PRIKK_*` variable the CLI reads:

| Kind | Variables |
|---|---|
| **Policy, non-secret** (8) | `ACTIVE_PATCH_WARN`, `ACTIVE_PATCH_LIMIT`, `BUNDLE_MAX_BYTES`, `BUNDLE_MAX_OBJECTS`, `EXCHANGE_MAX_BYTES`, `EXCHANGE_MAX_OBJECTS`, `SYNC_SUMMARY_MAX_BYTES`, `SYNC_SUMMARY_MAX_REFS` |
| **Identity, non-secret** (2) | `AUTHOR_KEY_ID`, `MAINTAINER_KEY_ID` |
| **Secret key material** (2) | `AUTHOR_SEED`, `MAINTAINER_SEED` |

**These are three problems, not one, and conflating them is what made the first framing too small.**

- **The eight policy values are an already-admitted gap.** `main.rs` says of the threshold pair:
  *"never persisted in the repository — a durable policy belongs to a future general configuration
  increment."* Today every non-default limit must be re-exported **in every shell and every CI job**,
  permanently. **This carries no security question at all.**
- **The two key ids are not secret** and are pure friction.
- **The two seeds are private key material**, and everything about persisting them is a security
  decision (§6).

## 4. The constraint that rules out the obvious answer

**`placement.rs`'s `ALLOWED_THIRD_PARTY` lists `("prikk", &[])` — the CLI crate is gated at *zero*
third-party dependencies**, and the gate runs in the standing set.

**So every off-the-shelf settings crate is out by construction**, as the owner noted of
`app-json-settings`. A configuration file must be parsed **with `std` alone**, which is a design input
rather than an inconvenience: it argues for a format that is trivial to parse and impossible to
half-parse.

**The precedent exists and is recent.** `.prikkignore` (RFC 124, `text_span`'s sibling `ignore.rs`) is
line-oriented, one directive per line, whitespace-trimmed, blank lines skipped, **fail-closed on
anything malformed**, and **absent means defaults**. Those four properties were reviewed and are the
right starting point for any config file here.

## 5. The option space

**None of these is recommended yet.** They are separable and can combine.

**(a) `prikk config` — durable, non-secret settings.** The eight policy values plus key *ids*. No
secret at rest, no lifecycle question, and it closes a gap `main.rs` already names. **The cheapest
real improvement, and the one with no security cost.**

**(b) `prikk key` — generate and derive.** Closes §2's capability gap: produce a fresh seed with its
public key, and derive a public key from a seed already held. Uses an API that exists. **Note it can
only print secret material to a terminal** while there is nowhere to put it — which is a reason to
consider (a) and (c) together rather than shipping (b) alone.

**(c) `prikk setup` — a flow, not storage.** Composes generation, trust registration, and telling the
user what to export. **It need not require prikk to own any secret**: it could emit a file the user
sources, keeping convenience separate from persistence.

**(d) A helper boundary**, as git separates config from credential storage and ssh keeps keys in files
the tool never invents. **Worth naming even if refused** — refusing it deliberately is a different act
from never having considered it.

## 6. Security — the part that must not be rushed

**Persisting a seed means private key material at rest**, owned by prikk, on a path prikk chooses,
under permissions prikk sets. Every one of those is a decision this project has not made.

**And it presses on a gap that is already open.** prikk has **no key rotation and no revocation beyond
the trust store**; `SECURITY.md` states plainly that release-signer verification of a `prikk` binary is
not yet available, and key lifecycle is deferred to RFC-025. **Making a long-lived key easy to keep is
not obviously an improvement while the machinery to retire one does not exist.**

**The honest framing**: convenience that increases the population of long-lived keys raises the cost of
not having lifecycle. **That trade is the owner's**, and this RFC must not smuggle it in as a
by-product of a nicer first run.

**A shape that avoids the trade entirely** is (c)'s emit-a-file form: prikk never stores a secret,
never chooses a path, never sets a permission — it hands the user something and says what it is.
**Less convenient, and materially smaller in security surface.**

## 7. Out of scope

**Key lifecycle** — rotation, revocation, hardware or threshold signing. RFC-025's, and named here only
because §6's decision changes its urgency.

**The two-key model itself.** Author and maintainer are separate deliberately: authoring is not
publishing. **This RFC does not propose collapsing them** — but it may propose that a visitor's first
success not require both, since `init` → `commit` → `verify` works today with the author key alone
(verified), and the maintainer ceremony is needed only to `seal`.

## 8. What this RFC does not yet decide

The shape (§5), whether prikk ever stores a secret (§6), and the format of any config file (§4).

**What it does establish**: the gap in §2 is real and measured; the three concerns in §3 are
separable; and the zero-dependency constraint in §4 rules out the usual answers before the design
starts.
