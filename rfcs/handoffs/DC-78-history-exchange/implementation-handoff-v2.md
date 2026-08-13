# DC-78 History Exchange — Implementation Handoff v2

**Supersedes v1 and its two addenda for implementation purposes** — they remain the record of how the
rulings were reached. **Design accepted by the project owner 2026-08-09** and is **§D of
`rfcs/done/DC-78-HISTORY-EXCHANGE.md`**. **Authored by** the architect.
**Cleared to answer §D7's four questions only.** Implementation follows their acceptance.

## 1. Read §D first, and know why it is short on mechanism

The design deliberately adds **no new provenance mechanism, no new verification path, and no "pull"
concept** — because each already exists. If your implementation finds itself inventing one of those,
that is a signal to stop and report, not to proceed.

## 2. The finding that drives the whole increment

`verify/objects.rs:223` applies the publication-trust check to **`Block` and `RefState`**, so
`verify_repository` checks **every block** against the trusted policy. With DC-11's single key:

> **Adopting a peer's key today makes the receiver's own blocks untrusted, and their own repository
> fails `verify`.**

**That is the bug this increment exists to make impossible.** It is also the sharpest test of whether the
implementation is right — see §4.

## 3. This touches the one component every other guarantee rests on

The maintainer trust store is what makes "sealed by an authorised identity" mean anything. Everything
else in prikk — merge adoption, publication CAS, release signing — assumes it is correct.

**Consequences for how you work here:**

- **The parser stays strict and fixed-shape.** DC-11 says it "is not a general TOML implementation."
  Accepting a list must not turn it into one. Reject anything you do not explicitly expect.
- **Fail closed everywhere.** An unparseable policy, an unknown key, a mismatched public key — all
  refuse. Never degrade to "trust it anyway."
- **No key is adopted implicitly.** Adoption is an explicit act, recorded.

## 4. What I will check hardest — four negative controls

Each is a *specific* failure I will construct myself if you do not:

1. **Adopting a second key must not invalidate existing history.** Build a repo with its own sealed
   history, adopt a second key, run `verify`. **It must still pass.** This is §D1's bug; if it survives,
   the increment has failed regardless of what else works.
2. **Import must not advance any local ref.** After importing a bundle, every `heads/*` is byte-identical
   to before. Incorporating received work is a **merge**, explicitly performed.
3. **TOFU must enforce, not re-prompt.** A changed public key for an already-adopted key id is
   **refused**. Construct it; show the refusal.
4. **The bundle must be a verifiable subset, not a summary.** A receiver verifies with
   `verify_repository` **unchanged** — no new verification path, no digest-of-digests shortcut.
   NFR-PERF-04's spirit forbids a bundle that becomes a new root of trust.

## 5. Two documentation sentences become false the moment this lands

`docs/src/guide/security-setup.md:67` and `docs/src/reference/trust-threat-model.md:61` both state
plainly that **there is no trust-on-first-use rule.** This increment builds one. **Both must change in
the same commit**, and the threat-model page must say what TOFU does and does not protect against —
first-contact substitution is the exposure it accepts, and saying so is the point.

## 6. Answer §D7 before writing code

1. Does the multi-key parser stay strict and fixed-shape?
2. **What does `required` mean once several keys exist?** §D2 says one trusted signature suffices —
   **confirm nothing in `trust.rs` or DC-11 assumed otherwise, and report if it did.**
3. Which ref namespace do received refs land in, and **does anything today assume every ref under
   `refs/` is locally sealed?** Check `branch list`, `history`, and `verify`'s counts.
4. **Does the per-block trust check cost change** when the policy is a set? It runs per block, and
   `FINDINGS.md` already records `verify` as O(N³).

## 7. Scope limits

**No transport.** No network, no new dependency, `ALLOWED_THIRD_PARTY` untouched. Exchange is a file.

**Genesis-complete only** (ruling 2). If you find yourself wanting a shallow horizon, that is a finding —
it would mean redesigning `verify`'s integrity walk.

**The trust claim is "sealed by a Maintainer key you adopted."** Never imply authorship is verified;
nothing in the product checks author signatures, and DC-53 is unscheduled.

Gates per rule 9 as amended. **Sequencing is yours** — DC-80 was next in your queue and is smaller.
