# RFC 115 — Sync: what it can mean for prikk (investigation)

**Status.** **Proposed — investigation, not design.** Started on the project owner's direction
2026-08-19, after badge criterion 2 closed. **Answers badge criterion 1's prerequisite question:** not
*how* to build sync, but *what sync is* for a VCS with no rewrite, no force-push, and immutable sealed
history. **No design exists and implementation must not start from this record.**

**Independence:** author-reviewed, the standing ceiling; every claim below cites the code it came from.

## 1. The board is wrong, and the correction narrows the problem

`MILESTONES.md` criterion 1 reads: *"Nothing built. Unowned, no increment. The largest single gap."*

**"Nothing built" is false, and it has been for some time.** Established from the code:

- **`bundle export` / `bundle import`** move a genesis-complete object closure between repositories
  (`bundle.rs`), now carrying AUTHOR key material as of DC-53 Stage 2.
- **Imported history lands in a received namespace** — `remotes/<origin ref name>` — recorded through
  `received::write_received_pointer`, deliberately **not** advancing any local ref
  (`dc78_bundle_exchange.rs::import_never_advances_a_local_ref_and_verify_reports_it_untrusted_until_adopted`).
- **Merge accepts a received ref as one side** (DC-85). `merge_evidence.rs:135` resolves
  `remotes/<name>` through `read_received_pointer`, and `dc85_merge_from_received_ref.rs` exercises
  export → import → merge end to end.
- **`verify` checks imported history's authorship** since DC-53 Stage 2, rather than reporting it
  permanently `Unverifiable`.

**So the local half of exchange exists and is tested.** What is missing is nameable and much smaller
than "everything":

1. **Transport.** There is no network code in the workspace — no TCP, HTTP, or SSH client anywhere.
2. **Repository-complete transfer.** `export_bundle(layout, ref_name)` takes **one ref**, and import
   lands it under `remotes/`. Trust policy, the whole author-key container, tags, and other branches do
   not travel (also RFC 114 §5.2).
3. **Whatever criterion 1's own words require beyond that** — see §5.

**The board row should be corrected regardless of what is built next.** A criterion that overstates its
own gap misdirects planning, which is the failure this project has now hit four times in a week.

## 2. What sync cannot mean here

**prikk has no amend, no rebase, no force-push, and no rewrite.** Sealed blocks are immutable and
maintainer-signed. Therefore:

- **There is no "your history diverged, fix it locally by rewriting"** story. Every reconciliation must
  be additive.
- **Receiving is an admission decision, not a fast-forward.** DC-78 already ruled that exchange claims
  only *"sealed by a Maintainer key you adopted"*, and import deliberately refuses to advance a local
  ref. **That refusal is a feature and any sync design must preserve it.**
- **A push that moves someone else's ref is not expressible.** The receiving side decides, always.

**So sync in prikk cannot be Git's sync with different nouns.** The closest existing shape is *receive
into a namespace the receiver controls, then let the receiver decide* — which is what the code already
does.

## 3. The question that decides everything: is sync a protocol, or a transfer?

**Two readings, and they differ by an order of magnitude in scope.**

**(a) Sync is a protocol prikk owns.** A client and a server, negotiation of what each side lacks,
incremental transfer, authentication, resumption. This is what "sync" means in most VCSs and what a user
migrating from Git will expect.

**(b) Sync is a repository-complete transfer, plus any transport the user already has.** prikk produces
and consumes a complete, verifiable artifact; moving it is `scp`, a shared drive, object storage, or a
courier. **The receiving side's decisions — admit, verify, merge, publish — are already built.**

**Reading (b) is smaller, composes with what exists, and does not commit prikk to owning a network
protocol, a wire version, or an authentication story** — three surfaces with their own compatibility
obligations under RFC 114. It also delivers the *carry-forward operation RFC 114 §5.2 needs anyway*, so
the work is not spent twice.

**Reading (a)'s honest advantage** is that (b) is not what most users mean by sync, and "use `scp`" may
read as an unfinished product regardless of whether it is sufficient.

### 3.1 The case against (b), examined properly

**Asked by the owner 2026-08-19 to steelman the opposite, and it is stronger than the architect's first
lean allowed.** Five reasons, two of which attack the argument for (b) directly.

1. **(b) may close criterion 1 on a technicality while leaving the claim it exists for unsupported.**
   The criterion's words — *"two machines can exchange sealed history"* — are literally satisfied by
   producing a file and moving it with `scp`. But the criterion exists to support *"prikk is a
   distributed VCS"*, and **this project has twice ruled that a criterion must not be closed by
   whichever reading happens to let the row be marked met** (RFC 111 §8, criterion 5). That ruling
   applies here against the architect's own preference.

2. **Incremental exchange is the normal case, and (b) does not do it.** A repository that syncs daily
   needs O(delta), not O(repository), per exchange. (b) can only scope an artifact to a delta if the
   sender knows what the receiver already has — **which is negotiation, which is a protocol.** So (b)
   either stays whole-repository, and becomes impractical at exactly the scale that matters, or it grows
   the thing it was chosen to avoid.

3. **(b) is not uniformly smaller — on consistency it is larger.** §6.2's snapshot problem is (b)'s
   alone: a repository-complete artifact needs **repository-wide** consistency across many refs, with no
   mechanism today. A protocol could serve refs individually with per-ref consistency and let the
   receiver reconcile, which is a weaker requirement. **The architect's "smaller" claim holds for
   surface area and fails for this axis.**

4. **"The work is not spent twice" is conditional, not certain.** It holds if a future protocol
   negotiates and then ships a scoped bundle. It fails if the protocol streams objects individually —
   the more usual design — in which case the monolithic artifact is not reused and (b)'s composition
   argument evaporates. **This was stated as a fact in the first draft and is properly a hypothesis.**

5. **Adoption.** A user migrating from Git expects `push`/`pull`. "Produce a file and move it yourself"
   may read as unfinished regardless of technical sufficiency, which matters to a project whose stated
   aim includes growth.

**What survives in (b)'s favour**, after that: prikk's trust is **object-level, not channel-level** —
signatures and adopted keys, not TLS and sessions — so an artifact moved by any means is verified
exactly as well as one delivered by a protocol. **That is a real and unusual advantage**, and it is why
(b) is defensible at all.

### 3.1a The case against (a), examined with the same care

**Asked by the owner 2026-08-19, excluding initial cost.** Six reasons; the first two are the ones that
would be hard to undo.

1. **A protocol is a second compatibility surface, and it is strictly harder than the one prikk just
   struggled with.** RFC 114 froze identity and gated the repository format — and prikk still severed
   every migration path with a format change two days before that gate existed. **Protocol compatibility
   is harder than format compatibility**: a repository can be migrated before it is opened, but two live
   peers on different versions must interoperate *as they are*. There is no "migrate then connect".
   **Every prikk release would owe interoperability with every prior release, permanently.**

2. **It creates a network attack surface prikk does not currently have.** Today there is **zero** network
   code in the workspace and **zero** async/TLS dependencies — the entire third-party runtime surface is
   five crates. A protocol means parsing adversary-controlled bytes off a socket, concurrently, with
   resource exhaustion and denial-of-service as first-class concerns. **DC-86 already had to add
   declared-count and byte bounds because *file* input is untrusted; network input is worse in every
   dimension** — unbounded, adversarial, and concurrent. And it arrives with an async runtime and a TLS
   stack, which is a step change in the audited surface of a product whose claim is verifiability.

3. **It drags in access control, which prikk has no concept of.** Object-level trust answers *"is this
   history authentic?"* It does not answer *"may this peer read this repository?"* A server needs the
   second, and prikk has no model for it — nor has criterion 4's signer bootstrap happened.

4. **It presumes a topology the product has not chosen.** Client/server, peer-to-peer, federated, hosted
   — a protocol commits to one before RFC 108's workspace concept and the owner's hosted-versus-local
   direction are settled.

5. **A protocol designed now would probably be Git's protocol with different nouns** — ref-advertisement,
   have/want negotiation, packfile transfer. **That is the same trap RFC 113 §3.1 named for the
   intermediate representation**: a design that converges on the familiar model imports the assumptions
   prikk exists to reject, and then the wire format makes them permanent.

6. **It does not fit this project's testing discipline.** prikk's quality regime is deterministic tests
   and gates that can be observed failing. Two live processes, timing, partial failure and resumption
   are a different discipline, and the weakest part of the regime would become the part guarding the
   largest new surface.

### 3.1b A third option the first draft missed: **(c) artifact plus a published basis**

**(b)'s fatal objection was reason 2 — no incremental transfer without negotiation.** That is only true
if the *sender* must discover what the receiver has **interactively**.

**It does not have to be interactive.** The receiver publishes a small, static **basis** — the ref-state
ids it already holds. The sender reads it and exports an artifact **bounded by that basis**, carrying
only what is missing. **O(delta) transfer, no live peer, no socket, no negotiation, no protocol.**

**This is not novel and that is the point: Git bundles already do it.** A Git bundle carries
*prerequisite* commits and is refused if the receiver lacks them. The design space is proven, and prikk's
`export_bundle` already walks a closure — bounding that walk by a set of known-present object ids is an
extension of what it does, not a new mechanism.

**It answers (b)'s strongest objection while incurring none of (a)'s six.** It does not answer
"discoverability" — someone still has to move two files instead of one — but that is a usability gap, not
an architectural one.

**This deserves to be evaluated as a first-class option rather than a footnote**, and the first draft's
binary framing is what hid it.

### 3.1c What (c) costs a user in production

**Asked by the owner 2026-08-19.** Ordered by how often a user would meet them. **Two are not (c)'s
fault — but (c) makes them the daily case, which is the same thing from the user's chair.**

1. **Every sync would seal a merge block, even for a purely linear update.** `merge_execute.rs:172`
   sets `kind: BlockKind::Merge` unconditionally; **there is no ancestor check and no fast-forward
   path** — the only refusal defined is non-confluence. So a user who has diverged in no way, pulling a
   straight series of someone else's commits, still gets a two-parent merge block per exchange. **Git
   fast-forwards and creates nothing.** Over a week of syncing this is visible history pollution, and it
   is the objection a Git migrant would raise first. **Pre-existing, but today it is only met by people
   who actually merge; under (c) it is met by everyone, every time.**

2. **Two transfers per exchange, in one direction only.** Basis out, artifact back. Bidirectional means
   four. `git pull` is one command. **This is (c)'s core ergonomic cost and it does not go away with
   polish** — it is inherent to not having a live peer.

3. **A stale basis is silently wasteful, and a wrong basis is not silent.** Stale (the receiver has
   advanced since publishing) merely over-transfers — harmless. **Wrong** — a basis from a different
   machine, or from before a restore — produces an artifact whose prerequisites the receiver lacks.
   **Import must detect missing prerequisites and refuse with a clear message**; if it trusts the basis
   blindly, the receiver gets a partial history and finds out later. Git bundles refuse exactly this
   way, and prikk must too.

4. **The first exchange is whole-repository and will exceed today's limits.** With an empty basis, the
   artifact is the entire closure. `DEFAULT_BUNDLE_MAX_OBJECT_COUNT` is 100,000 and
   `DEFAULT_BUNDLE_MAX_TOTAL_BYTES` is 256 MB — bounds sized by DC-86 for a single ref. **Onboarding a
   real repository is the case most likely to hit them**, and raising a security bound to make a feature
   work is exactly the trade that needs deciding deliberately, not discovered.

5. **Trust setup is O(n²) and manual.** A receiver must adopt each sender's MAINTAINER key or their
   history reads untrusted (DC-78, by design). For a team of five that is twenty adoptions. **Correct
   security, real friction** — and the friction is worst at exactly the moment a team is evaluating
   prikk.

6. **A user cannot answer "am I up to date?" without an exchange.** A protocol can be asked; a file
   cannot. Nor can divergence be discovered before transferring — you import, then find out.

7. **Author verification is weakest at first contact.** DC-53's pinning gives continuity, not
   authenticity, so the first artifact from a new colleague is trust-on-first-use. **A user who believes
   "prikk verified the author" is believing more than is true** — which is why criterion 5's row states
   the limit, and why the CLI wording matters here more than anywhere.

8. **File hygiene.** Bases and artifacts accumulate, are emailed, are mixed up between peers. Nothing
   corrupts — signatures and prerequisites see to that — but it is untidy in a way a protocol is not.

**The honest summary:** (c)'s architectural objections are answerable; **its costs are ergonomic and
they are concentrated at onboarding** — first sync is largest, trust setup is heaviest, and the merge-
block behaviour is most surprising, all in a new user's first hour. **That is the worst possible place
for friction in a product seeking adoption**, and it is a stronger argument for eventually adding (a)
than any of §3's architectural points were.

**Item 1 is separable and worth fixing regardless of which option wins.** A fast-forward path is a
missing capability in `merge_execute`, not a sync question.

### 3.2 The architect's refined position

**Not "start with (b) and add a protocol later" — that framing bundles two claims and one of them is
weak.** Separating them:

- **Build the repository-complete artifact, and do not call it sync.** RFC 114 §5.2's format-7
  carry-forward needs it, paikuli's prikk encoder needs a repository-complete landing, and criterion 1
  will need it whatever shape sync takes. **Three independent needs converge on it**, which is the
  strongest evidence available that it is a real unit — and if it is built for sync alone, the other two
  will each grow their own half-version.
- **Leave criterion 1 open until incremental exchange has an answer.** §3.1's reason 2 is the one the
  architect cannot argue away: whole-repository transfer per exchange is not what the criterion is for.

**This is a weaker claim than the first draft made, and deliberately so.** The owner should decide
whether the artifact ships as *migration and carry-forward* — where it is clearly right — or as *sync*,
where reasons 1 and 2 say it is not yet enough.

**Amended after §3.1a and §3.1b:** with the basis mechanism, **(c) removes the objection that kept the
artifact from being a credible sync answer.** The remaining gap between (c) and (a) is *discoverability
and convenience* — moving two files rather than invoking one command — which is a real product concern
and **not** an architectural one. **A protocol can then be added later as a transport over exactly the
same artifact and basis**, which is the composition claim §3.1 reason 4 correctly said was unproven for
(b) alone — and which (c) makes concrete rather than hoped-for.

## 4. Divergence — probably already answered, and worth confirming rather than assuming

A `RefState` carries `update_seq` and `previous_ref_state_id` (`payload/refs.rs:51,53`), so ref history
is a chain. **Two machines that both seal onto the same ref produce two chains from a common ancestor —
a fork.**

**prikk's existing answer appears to be: the fork lives in the receiver's `remotes/` namespace, and the
receiver merges it** — `merge` already takes a received ref as one side, seals a `BlockKind::Merge`
recording both parents, and `verify_merge_baseline` re-derives rather than trusts. **That is Git's shape,
minus the ability to pretend the divergence did not happen.**

**What must be confirmed rather than assumed** — and this is investigation work, not design:

- Does the merge path behave correctly when **both sides have advanced**, as opposed to the receiver
  being strictly behind? DC-85's test should be read for which case it actually covers.
- What happens when the same ref is received **twice**, with the second bundle superseding the first?
- Is there any case where reconciliation is **impossible** rather than merely refused — and if so, is it
  reported as such?

## 5. What "both verify it afterward" now means

Criterion 1's wording predates DC-53. **It is a stronger claim today than when it was written**, and the
design should be held to the stronger reading:

- The receiver verifies **structural integrity** (`verify`), **publication trust** against its own
  adopted maintainer keys (DC-78), and **authorship continuity** against transported author key material
  (DC-53 Stage 2) — the last of which did not exist when the criterion was drafted.
- **It remains trust-on-first-use.** Transported author material is sender-supplied; pinning gives
  continuity, not first-contact authenticity (`MILESTONES.md` criterion 5).

**A sync design must not quietly upgrade that claim.** "Both verified" means both ran the checks that
exist, not that either learned who the other really is.

## 6. What a design must decide

1. **§3's question — protocol or transfer.** Everything else follows.
2. **What a repository-complete transfer contains**: which refs, tags, received pointers, trust policy,
   the whole author-key container — and which of those a *receiver* should be allowed to accept
   silently. **Trust policy is the dangerous one**: carrying a sender's adopted maintainer keys into a
   receiver would let a sender expand what the receiver trusts.
3. **Where a complete transfer lands.** `remotes/` is right for foreign history and wrong for a
   repository moving itself (RFC 114 §5.2). One operation cannot have both meanings by default.
4. **Whether `verify`'s result travels.** A sender that has verified could say so — but that claim is
   sender-supplied and worth nothing without the receiver re-verifying, so it may be a footgun rather
   than an optimisation.
5. **What is refused.** Per prikk's standing posture, a refusal that is stated beats an approximation
   that is silent.

## 7. Non-goals

- **Not a hosted service**, not authentication, not authorization, not identity. Criterion 4's signer
  bootstrap is a separate and independent gap.
- **Not rewriting history** to reconcile divergence. There is no mechanism and there should not be one.
- **Not weakening DC-78's admission rule.** A receiver decides what it trusts; import must continue to
  refuse to advance a local ref on its own.
- **Not a second identity or format surface.** Anything sync adds is bound by RFC 114's contract like
  everything else.
