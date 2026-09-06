# RFC 141 — Publishing through CI, and the evidence nobody has ever produced

**Status.** **ACCEPTED by the project owner 2026-09-06**, the same day it was opened.

**Moved to `rfcs/accepted/` on acceptance** — the trigger is design complete, not handoff issued.

**What the acceptance covers, stated because a bare acceptance is scope-ambiguous.** The whole design
as written: §2's security position; §3's finding and the conclusion drawn from it; **§4's reading of
DC-35's authority composition**, which settles that registry publication sits in the
registry-administrator leg and that no past release was out of compliance; **§5.1's hard requirement**
that publication never become automatic on tag, with the publish job behind a required-reviewer
environment; §5.2's ordering, idempotence and fail-stop semantics; §5.3's recognition of the
publishing workflow as a governance surface; and §7's four increments in their stated order.

**It does not authorize anything §6 excludes.** In particular it does not configure Trusted Publishing
— that is the owner's act on their own account — and it does not touch `release-signers.toml` or the
signer bootstrap, which sit on a different leg and remain owner-blocked as criterion 4.

Originally opened as: **PROPOSED, 2026-09-06**, at the project owner's instruction, on their proposal:
*"We had better integrate CI with the publishment. I can configure Trusted Publishing on crates.io."*

**The owner's proposal is right and this RFC recommends adopting it.** It also records the thing that
makes the case stronger than convenience, and the three obligations that come with it — one of which
is a hard requirement without which this change would remove a control the owner currently exercises.

**Author-review independence.** The architect wrote this RFC and is also its only reviewer — the
standing gap on every architect-authored design here. Compensated at implementation review.

**Tracks.** Release authority and its evidence. **No change to any shipped surface.**

---

## 1. What publication is today

Eight crates, in topological order, published by the architect from a workstation after the owner
authorizes it as a separate act from the tag. The credential is a long-lived crates.io API token held
outside this repository. Verification is a check that each crate's version is visible in the sparse
index before the next one goes.

**That verification is thinner than this project's own specification, and §3 is about how much
thinner.**

## 2. The security argument, which is the reason to do this

**A stored `CARGO_REGISTRY_TOKEN` is a standing credential.** Whatever holds it — a repository secret,
a workstation keyring — it is long-lived, it authorizes publication of every crate in the namespace,
and its compromise is silent. If it lived in repository secrets it would additionally be reachable by
any workflow change, any compromised third-party action, and any person who can run a workflow.

**Trusted Publishing removes the standing credential entirely.** crates.io verifies a GitHub Actions
OIDC assertion naming the repository, the workflow file, and (optionally) the environment, and issues
a **short-lived** token scoped to that publication. There is nothing durable to steal.

**This is the strongest posture available short of not automating at all**, and "not automating at
all" is what we have now — which, as §3 shows, has its own cost.

## 3. The finding: DC-35 specifies a release-evidence regime, and no release has ever produced one

DC-35 ("Crate-byte identity and completion") requires, for every crate:

> the staged `.crate` SHA-256, the registry-index checksum, the SHA-256 of bytes fetched after
> registry visibility, and equality of all three values. A dependent is not published until each
> predecessor is visible and its fetched bytes match the registry checksum and reviewed staged
> package.

It further specifies a `pending`/`partial`/`complete`/`superseded` state machine, evidence JSON with
an explicit schema version rejecting unknown fields, snapshot sequences starting at `001` and
contiguous, each snapshot naming its predecessor and that predecessor's observed SHA-256, and a
**cumulative append-only attempt list** retaining every prior attempt including failed ones.

**`release/schemas/release-evidence-v1.schema.json` exists. The oracle carries 73 cases for it.**

**No release has ever emitted one.** Checked directly: `0.35.0`, `0.34.0` and `0.33.0` each carry
sixteen assets and **zero** matching `evidence`.

**Manual publication has never delivered this and realistically never will.** It is mechanical
bookkeeping — three checksums per crate, eight crates, an append-only snapshot chain — performed
under exactly the conditions where humans skip steps: at the end of a long sequence, when everything
has already gone well. **A machine is the only plausible author of this document.**

**So the honest framing of the owner's proposal is not "automate a chore".** It is: *the only way
DC-35's publication half stops being aspirational is if publication runs where evidence can be
produced as a by-product of doing the work.*

## 4. Where Trusted Publishing sits in DC-35's authority model — a question this RFC answers rather than raises

**The architect flagged a concern before opening this RFC**: `release-signers.toml` is still
`authorized_primary_fingerprints = []` and DC-35 was accepted *"with no signer admitted"*, while
DC-35's governance boundary covers *"official upstream Prikk tags, official release-page assets, and
**official package namespaces**"*. If crate publication required an admitted signer, every release to
date would have been out of compliance.

**It does not, and the reading is settled here.** DC-35 §"Release-signer authority" defines authority
as **a composition, not a single gate**:

> reviewed protected-branch governance authorizes policy and signer changes; `release-signers.toml`
> is the commit-local allowlist; an allowlisted private key **authenticates a tag**;
> **hosting/registry administrators control publication capabilities**; and release evidence binds
> those outputs.

**Four distinct legs.** `release-signers.toml` governs the leg that authenticates a **tag** — DC-35
is explicit that "Tag creation must explicitly select an authorized full fingerprint" and that
verification fails "before atomic push". **Registry publication capability is a different leg,
controlled by registry administrators.**

**Two consequences, and both matter:**

1. **Crate publication has not been out of compliance.** It sits in the registry-administrator leg,
   which has always been exercised by the owner's crates.io account.
2. **Trusted Publishing is a refinement of exactly that leg, not a new authority.** It replaces "a
   human account's long-lived token" with "a capability bound to a named workflow identity". That is
   the same leg, more tightly bound — which is why it fits DC-35 rather than amending it.

**The empty signer allowlist remains an open gate on the *tag* leg**, unchanged by this RFC, still
owner-blocked as criterion 4, and **not** made better or worse by anything proposed here. Naming it
is how it stops being conflated with this.

## 5. Three obligations, and the first is a hard requirement

### 5.1 Publication must not become automatic on tag

**Today "cut it" and "publish it" are two decisions the owner makes separately.** They were separate
at 0.34.0 and at 0.35.0, hours apart, and on both occasions the second was withheld until asked for.

**A tag-triggered publish collapses them into one**, and the one it collapses is the irreversible
half: a crates.io version number is burned permanently, yank is not delete, and there is no
supersede-in-place. **RFC 121's whole vocabulary exists because conflating two different things into
one signal loses information; this would conflate two different decisions into one trigger.**

**REQUIRED: the publish job runs in a GitHub Environment with required reviewers.** The workflow
prepares, verifies and stages; it then waits. The credential handling is automated; **the decision
stays a human act.** This is not a compromise with the proposal — it is what makes the proposal safe
to adopt, and it costs one click.

**DC-35 already supports this reading**: its step 7 names the atomic branch+tag push as *"the release
event"*. Registry publication is downstream of the release event, not identical to it.

### 5.2 Partial publication is unrecoverable and must be designed for, not discovered

Eight crates in dependency order. **If the fifth fails, the first four are permanently published at
that version and cannot be retried, withdrawn, or corrected in place.** DC-35 names this state
`partial` and requires it to preserve the published crate identity, block all dependents and
`complete`, and demand *"incident evidence plus a superseding version rather than an overwrite"*.

**The job must therefore be:**

- **Ordered**, and verify each predecessor is index-visible before starting its dependents — which is
  DC-35's requirement, not a nicety;
- **Idempotent**, skipping any crate whose exact version is already live, so a resumed run after a
  transient failure cannot fail on "already published";
- **Fail-stopped**, never continuing past a failure to "get the rest out".

**A run that ends `partial` must say so in evidence and stop.** The temptation this design must
foreclose is a job that retries its way to a green tick over a half-published release.

### 5.3 The workflow binding is itself a trust anchor

Trusted Publishing authorizes a **(repository, workflow file, environment)** triple. **Whoever can add
or modify that workflow can publish.** That moves a piece of publication authority into the
repository's own branch protection and workflow review — which is where DC-35's first leg already
lives (*"reviewed protected-branch governance authorizes policy and signer changes"*), and it should
be stated as such rather than left implicit.

**Consequence: the publishing workflow is policy, not plumbing.** Changes to it are governance
changes. This project already constrains workflow contents through `command_scan`; this RFC does not
propose new machinery, only that the file be recognized for what it becomes.

## 6. What this RFC does not decide

- **Whether to configure Trusted Publishing on crates.io.** That is the owner's act and their
  account; the RFC assumes it and specifies what should exist once it does.
- **The signer bootstrap.** Criterion 4, owner-blocked, on a different leg (§4). Untouched.
- **Whether `release.yml` hosts the publish job or a new workflow does.** An implementation choice for
  the increment, to be reported with its reasoning.
- **Retrofitting evidence for past releases.** Out of scope. DC-35's sequence starts at `001`; whether
  the first machine-produced evidence document claims `001` or acknowledges nine prior releases
  without evidence is an increment-level question, and the honest answer is likely the latter.

## 7. Increments

1. **The evidence producer.** A `release-policy` subcommand that stages a crate, records the three
   SHA-256 values DC-35 names, and emits a `release-evidence-v1` document the existing oracle already
   validates. **Testable entirely offline against fixtures** — no registry, no credentials, no
   workflow. This is where the substance is, and it is deliberately first.
2. **The ordered, idempotent, fail-stopped publish routine**, exercised against a dry-run path.
3. **The workflow**, in an environment with required reviewers, wired to Trusted Publishing.
4. **The first evidence-carrying release**, attached to the release page as an asset.

**Handoff issued for increment 1, 2026-09-06:**
`rfcs/handoffs/141-publication-through-ci/release-evidence-producer-handoff-v1.md`, after the move to
`rfcs/accepted/`.

**It carries one instruction this RFC did not think to give, and it is the increment's sharpest
hazard.** The three checksums are nullable in the schema and `checksum_equality` has a
`"not-observed"` value — so **a producer that defaults to `"match"` because nothing contradicted it
would emit a document asserting an equality nobody checked.** That is strictly worse than the nothing
we have today: an absent document is an honest gap, a document claiming unverified equality is a false
record wearing DC-35's authority. The handoff names it as the single most damaging thing the increment
could ship.

**Increment 1 is worth having even if 3 and 4 never land** — it makes manual publication produce the
evidence DC-35 requires, which is a strict improvement over today regardless of who runs it.

## 8. Scope

**In:** the evidence producer; the publication routine's ordering, idempotence and failure semantics;
the environment gate; the recognition of the workflow as governance surface; the §4 reading of
DC-35's authority composition.

**Out:** everything in §6; any change to what is published; any change to the tag procedure; any
change to `release-signers.toml`, which **must not be modified by this work**.

## 9. Risks

**The gate becomes a formality.** A required-reviewer environment where the reviewer is the same
person who triggered the tag, clicking through without reading, is theatre. **The mitigation is not
procedural but informational**: the job must present what it is about to publish — versions, staged
checksums, the crates already live — so approving is an act of reading rather than of clicking.

**Automation makes a partial publish more likely, not less, in one specific way**: it removes the
human pause between crates during which a person notices something wrong. §5.2's fail-stop is what
replaces that pause, and it must be tested by injecting a mid-sequence failure, not assumed.

**Evidence that is produced but never read is a cost with no benefit.** DC-35's oracle validates the
document's shape; nothing checks that anyone consults it. This RFC does not solve that, and says so.

**Related:** DC-35 (release compatibility, status correction, and the authority composition §4 reads),
DC-43 (release security controls — release-blocked, and the signer bootstrap it inherits), RFC 121
(the exit-code vocabulary whose reasoning §5.1 borrows), RFC 107 (release distribution surface).
