# `trust-threat-model.md` — verify's Tag check and the widened gate set: implementation handoff

**Base:** current `main` (`da29c0f`, CI green). **Under `003-landing-work-on-main.md`.**
**Origin:** reported by the `verify` local-tag increment (`da29c0f` §5), plus a second gap I found while
scoping it.

**One file.** **Two changes, and the second is larger than the report that prompted this.**

---

## 1. `verify` now checks Tag publication trust — two enumerations are incomplete

`da29c0f` added the `LocalTagTrust` stage. Two places on this page enumerate what `verify` trust-checks
and both stop at three types:

- **`:139-140`**, `## What Verify Checks`: *"...and publication trust for **Block, RefState, and
  RefUpdate** envelopes against the repository-local maintainer trust policy."*
- **`:186`**, the anchor table: *"Verify checks publication trust for **Block, RefState, and RefUpdate**
  envelopes."*

**The qualifier is the whole point, and getting it wrong would be worse than leaving the row stale.**
`verify` checks **locally-published** tags only — those reachable from a local `tags/*` ref. **A
received, unadopted tag is deliberately not checked**, because its signature is the sender's under a key
this repository has not adopted. The page already says so in `## Trust Roots and Roles`
(*"an `Unverifiable` tag is stored and reported exactly like a `Sound` one"*).

**"`verify` checks Tag publication trust", unqualified, is a false claim on a threat model.** Say
locally-published, and say why received ones are exempt — or point at the section that already does.

## 2. The larger gap — the page never recorded `053e442`

**Zero mentions of `tag create`, `branch create` or `branch close`** anywhere on the page. `053e442`
added the maintainer trust gate to all three, and the page predates it.

**`:77` now misleads by omission:**

> *"**Seal** uses real role-bound Ed25519 MAINTAINER signatures for Block, RefState, and RefUpdate
> envelopes and **verifies the signer against the local maintainer trust policy before publishing**."*

**Accurate about `seal`. Incomplete as a description of the trust posture** — a reader assessing *which
operations check the trust policy* concludes only `seal` does. **Eight do**: `seal`, `merge`,
`sync build`, `sync seal`, `sync adopt-tag`, and — since `053e442` — `tag create`, `branch create`,
`branch close`.

**Derive that list yourself** (`grep -rn "verify_signer_trusted(" crates/ --include='*.rs'`, excluding
tests and the definition). **It is my list, and my lists have been wrong four times in this arc.**

**Where it goes is yours to judge** — extending `:77`, or its own short paragraph. **Do not restructure
the page.**

## 3. Do not overclaim — the standing constraint on this file

- **Do not imply a gate is cryptographic proof.** It checks the *local operator's own* signer against the
  *local* policy. It says nothing about who anyone is.
- **Do not reuse criterion 5's AUTHOR trust-on-first-use framing.** A Tag and a RefState are
  **MAINTAINER**-signed. **The previous increment corrected me on exactly this** — the page's own
  MAINTAINER-key TOFU paragraph is the right connection, not DC-53's AUTHOR one.
- **Report what you deliberately did not claim.** On a threat model that list is part of the deliverable.

## 4. Out of scope

- **Every other file.** The reference pages were adjudicated at `93c0b53`; **report contradictions, do
  not edit.**
- **No code.** If the page should say something the code does not do, **report it**.
- **`MILESTONES.md`, `ROADMAP.md`, the badge.**

## 5. What to report

1. **Both enumerations** (§1), with the exact qualifier you used.
2. **Your derived list of gating surfaces** (§2) — and whether mine was right.
3. **Where you put §2's correction**, and why there.
4. **What you deliberately did not claim** (§3).
5. **Full gate set against the exact commit, after the last edit**, plus `mdbook build`.
6. Test counts — **expected unchanged**.
7. Anything here that was wrong, **including my three line numbers**.

**Stop and escalate, do not guess**, if: stating the locally-published qualifier requires asserting
something about received tags the code does not enforce; or you find a **third** place on this page whose
claim `053e442` or `da29c0f` invalidated — **that would make this a pattern rather than two edits, and
it is the finding I would most want.**
