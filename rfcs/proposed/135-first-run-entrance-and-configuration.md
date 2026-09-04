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

## 4. The dependency question — a decision, not a prohibition

**Corrected 2026-09-04.** The architect first wrote that `placement.rs`'s `ALLOWED_THIRD_PARTY` entry
`("prikk", &[])` puts every settings crate *"out by construction"*. **The owner refused that reasoning
and was right.**

**The constant states no policy.** It is a per-crate allowlist in which every non-empty entry carries a
recorded reason — `windows-sys` cites DC-96 — and `("prikk", &[])` records **what the CLI currently
depends on, not what it may ever depend on.** It is the same allowlist-with-reasons idiom as
`UNSAFE_EXEMPT_CRATES`, whose own module doc states the principle: *a visible edit to a reviewed
constant is the control; invisibility was the problem.*

**So adding a dependency to the CLI is a reviewable act, not an impossibility** — the same act that
admitted `sha2`, `ed25519-dalek`, `getrandom`, `rustix`, and `windows-sys` to their crates.

**The owner's position, recorded because it governs this RFC's design:** *"monolith without
carefulness is not preferred. If some external crate is well designed and properly maintained, there
is (or is not) possibility to rely on it. Of course, we should be really careful and take sufficient
verification."*

**This project already has a method for exactly that question, and it should be used rather than
re-invented.** **DC-50** was an explicit return-on-investment decision on whether to depend on `sha2`
or write SHA-256 first-party; it produced a decision record, and **DC-55** implemented the outcome.
**DC-79** and **DC-80** are dependency *upgrades* carried out as their own reviewed increments. The
project has both adopted and displaced dependencies deliberately, with evidence each time.

**What survives of the original point, as a preference rather than a constraint:** a configuration file
that `std` alone can parse has no supply-chain surface, no version-skew risk, and cannot half-parse.
**That is an argument to weigh, not a rule that decides.** If a settings crate is well designed and
well maintained, DC-50's method is how this project finds out whether depending on it is the better
trade — and the shipped binary's dependency surface is one input to that, not the whole answer.

**The precedent for the std-only shape**, if it wins:

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
separable; and §4's dependency question is **open and decidable by DC-50's own method**, not closed by
the current allowlist.

**And one correction worth carrying**: the architect twice today read a *mechanism* as a *policy* —
first `candidate_sequence`'s structure as proof of reachability (RFC 134 §3), then this allowlist as a
prohibition. **A control that makes a change visible is not a rule forbidding it.**
