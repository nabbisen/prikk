# RFC 115 — Sync: what it can mean for prikk (investigation)

**Status.** **ACCEPTED by the project owner 2026-08-19** — **an investigation, not a design.**
Acceptance adopts §2's findings and rulings and §5.1's binding test discipline. **It settles neither the
transport (§3, four options open) nor §6's design decisions.** Started on the owner's direction the same
day, after badge criterion 2 closed. **Answers badge criterion 1's prerequisite question:** not
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

## 2.1 The owner's reframing, 2026-08-19 — and it is correct

> *"I feel the subject is not network connection (exchange) but design on data model interface on what
> to be shared."*

**Agreed, and §3's four transport options were the wrong centre of gravity.** Transport is
**downstream and largely interchangeable**: a file, a file plus a basis, SSH with prikk on both ends, or
a protocol all deliver the same thing once *what* travels is settled. **The data-model interface is
upstream and hard to reverse** — it fixes what a receiver can verify, what claims ride along, and what
it may do with them.

The evidence was already in this RFC and the architect did not act on it: **§3.1e's finding is a
data-model finding, not a transport one.** prikk's merge-per-sync cost comes from exchanging *blocks*
rather than *patches*, and no choice of transport changes that by a single line.

**§3 is retained below as the transport survey it is, and demoted.** It should be settled *after* §2.2,
and any of its four options remains available.

## 2.2 The ruling: the exchange unit is the Patch; block recognition travels as a claim

**Ruled by the project owner 2026-08-19:** *"the unit should be patch… The exchange should be done per
patch-level instead of block-level. In addition, of course, block-level recognition should be shared."*

**The data model already supports this, which is the strongest evidence it is right.** Verified:

- **`PatchPayload` carries `parent_patch_ids`** (`payload/patch.rs:57`) — the schema models a patch DAG
  independent of any block.
  **Corrected 2026-08-19 (RFC 115 Checkpoint 1 §0): this is true of the schema and false of the data.**
  **No production code has ever populated the field** — `worktree_patch/node_authoring.rs:567` and
  `patch_inverse.rs:142` both set `Vec::new()`, and every other construction site is a test. Verified
  independently. The original sentence read *"patches already form a DAG"*, which conflated capability
  with behaviour; **the ordering it claimed exists does not exist in any patch that has ever been
  written.** The owner's ruling stands — it rested on the exchange unit, and this was corroborating
  evidence, not its basis — but **a design must derive closure ordering rather than read it**, and the
  accepted answer is to derive it at export time from block lineage the sender already holds.
  **Consequence worth carrying forward:** a hostile sender would be the **first real-world producer** of
  a non-empty `parent_patch_ids` any receiver has ever seen.
- **`PatchPayload` carries `preconditions`** (`:61`) — a patch already states what it requires to apply,
  which is what makes out-of-order arrival checkable rather than hopeful.
- Patches are **content-addressed** and **AUTHOR-signed**, and since DC-53 Stage 2 their signing key
  material travels. **Set reconciliation — "which patch ids do you have that I do not" — is expressible
  today over objects that already exist.**

**So the Block is what it always was: a publication envelope.** Exchanging envelopes forced prikk into
Git's shape at the exchange layer (§3.1e); exchanging patches puts it back where Darcs and Pijul are.

### 2.3 What the shared interface must define

Five things, and none of them is a network question:

1. **The unit and its closure.** A patch plus the transitive `parent_patch_ids` the receiver lacks —
   plus its author key material, so authorship is checkable on arrival (DC-53 Stage 2).
2. **The "have" representation.** At patch level this is a **set of patch ids**, not a chain — simpler
   than the block-level basis §3.1b proposed, and the reason patch-level makes incremental exchange
   natural rather than negotiated.
3. **Block recognition as a claim, not as the carrier.** The receiver learns *which patches the sender
   sealed into which block, under which maintainer key*. **That is an assertion about patches, which is
   attestation-shaped**: a claim *about* history, separable from it, rather than the thing that carries
   it. **The reasoning stands on its own structural merits.** RFC 110 §4.1 and RFC 113 §4.1 reach for
   the same shape, but **both are unaccepted proposals and RFC 113's text calls itself a recommendation
   rather than a ruling** — corrected 2026-08-19 after Checkpoint 1 noted the original wording lent them
   an authority neither has.
4. **What the receiver may do with a received patch.** It has an AUTHOR signature but **no maintainer
   seal** — so DC-78's *"sealed by a Maintainer key you adopted"* does not cover it. **The coherent
   answer is that the receiver's own maintainer seals what it accepts**, which is exactly how Darcs and
   Pijul work and preserves DC-78's rule rather than weakening it: every block in your repository is
   still sealed by someone you trust, because you sealed it.
5. **Where an accepted-but-unsealed patch lives** before that happens. The WAL is the receiver's *own*
   active work; received patches are not that. This is genuinely undecided and is the one item with no
   existing home.

### 2.4 The consequence that must be accepted explicitly

**Two repositories can agree on every patch and disagree on every block.** If the receiver seals what it
accepts, its blocks have different ids, different lineage, and different state roots from the sender's —
**with identical content**.

**That is not a defect; it is what patch-level exchange means**, and it is Darcs's *"adjusted to fit the
new context"* stated in prikk's vocabulary. But it must be accepted deliberately, because it changes
what "the same history" means: **identity holds at patch level and not at block level.** Every
comparison, report, and user-facing statement about two repositories being in sync has to be written in
those terms.

**This is also why block recognition must travel (item 3).** Without it a receiver cannot tell whether
the sender considered a patch published, only that it exists.

### 2.5 Is anything actually lost? — and divergent blocks are **not** forced

**The owner's question, 2026-08-19:** *"Is it unnecessary to reconcile or arbitrate it? I care about
whether verification or guarantee is lost… If a person sees A and another person sees A', may somebody be
confused or untrusting?"*

**Taking the guarantees one at a time, against the code:**

| Guarantee | Under divergent blocks | Why |
|---|---|---|
| **Content integrity** | **Not lost** | Object ids are content hashes. Both parties verify the same patch bytes. |
| **Authorship** | **Not lost** | The AUTHOR signature is over the *patch's* own object id and travels with the patch (DC-53 Stage 2). Both verify identically, subject to criterion 5's TOFU limit. |
| **Publication trust** | **Not lost — and this is by design** | DC-78 already rules that a receiver's claim is *"sealed by a Maintainer key **you** adopted."* Bob's repository asserting Bob's maintainer vouched for it is the intended semantics, not a degradation. |
| **Lineage / state-root integrity** | **Not lost within a repository** | Each repository re-derives and verifies its own lineage. Both are valid. |
| **Cross-repository comparison by one identifier** | **Lost** | "We are both at block X" stops working. This is the real cost. |
| **Citable identifier** | **Not lost, but it moves** | The patch id is stable across repositories and is what should be cited. Darcs does exactly this — patches keep *"the same identifier as from the source repository"* while their representation is adjusted to context. |

**So no verification is lost. One convenience is** — and the confusion risk is real but is a naming
problem, not a trust problem: a person who sees a different block id may conclude history was altered.
**The mitigation is that the patch id is the global identifier and the block id is local publication
detail**, stated in the UI and the docs rather than left for users to infer.

**A cheap restoration of the lost comparison:** a **patch-set digest** — a canonical hash over the sorted
set of patch ids reachable from a ref. Two repositories with the same content produce the same digest
regardless of block structure, restoring the one-hash "are we the same?" check at the level where
identity actually holds.

### 2.6 The surprise: canonical sealing would make blocks converge

**`BlockPayload` contains no local nondeterminism at all** (`payload/block.rs:49-68`): parent block ids
(sorted), kind, patch ids (canonical order), state Merkle root, an optional snapshot ref, and two
merge-only fields. **No timestamp, no maintainer identity, no machine identity, no nonce.** And
**signatures are excluded from the object-id preimage** (`envelope.rs:29`).

**Therefore two repositories that seal the same patches, with the same parents and the same grouping,
produce the *same block id* — each carrying its own maintainer's signature.** Divergence is not imposed
by the format; it comes only from **how patches are grouped into blocks**, which today is a human and
temporal choice.

**So there is a third option nobody has named: canonical sealing.** Make grouping a deterministic
function of the patch DAG, and block identity becomes global again — with per-repository trust preserved,
because the signature is outside the id.

**The cost is real and it is why I do not recommend it yet.** Blocks exist partly to *bound expensive
patch reasoning* by batching; the most obviously canonical grouping — one patch per block — discards that
rationale entirely. **Canonical grouping trades the performance argument blocks were built for against
global block identity**, and that trade deserves its own evaluation rather than being made in passing.

### 2.7 Recommendation on reconciliation

**Do not arbitrate. Accept patch-level identity, and make it visible.**

1. **Patch id is the citable, global identifier.** Issue trackers, advisories, release notes and CLI
   output should lead with it.
2. **Block id is local publication detail**, and prikk should say so where a user would otherwise assume
   otherwise.
3. **Add a patch-set digest** so "are these two repositories the same?" has a one-hash answer again.
4. **Record canonical sealing (§2.6) as a genuine option that was declined for a stated reason**, not as
   something nobody considered — because the data model permits it and a future reader will notice.

**Arbitration — electing one repository's block structure as authoritative — is the one thing to avoid.**
It would need a coordinator, which is precisely what a distributed VCS must not require.

## 3. Transport survey — downstream of §2.2, retained for when it is decided

**Superseded as the primary question by §2.1.** The four options below remain open and any of them can
carry §2.3's interface.

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

### 3.1d Prior art: how Darcs and Pijul actually do it

**Checked against both projects' own documentation 2026-08-19, not recalled.** They are the closest prior
art — the only two patch-theory VCSs in real use — and the finding is not what the (a)-versus-(c) framing
predicted.

**Neither chose. Both ship (a) and (c) simultaneously.**

- **Darcs** — *"The send and apply commands use patch bundle files that people tend to exchange by
  email"* (darcs.net/Using/Model), **alongside** `push`/`pull`. The bundle workflow is first-class and
  still used, not a legacy path. `darcs pull` is **interactive**: it offers patches one at a time and
  refuses a selection whose dependencies are not also taken.
- **Pijul** — `clone`/`pull`/`push` against a local path, over **SSH**, or over **HTTP/HTTPS**, which is
  **pull-only**: *"Pijul is not (yet) able to push patches to an HTTP URL."* And `pijul apply [CHANGE]...`
  reads *"the change in text format on the standard input"* when given no file — an offline path
  equivalent to (c).

**So the binary framing in §3 was wrong about the industry, not only about prikk.** The mature answer in
this space is both, with the file path arriving first and never being removed.

### 3.1e The difference that matters more than the transport

**Darcs and Pijul do not have prikk's §3.1c problem 1, and the reason is theoretical, not ergonomic.**

Darcs describes pulling as patches being *"concatenated to the latter repository's sequence (or unioned
with its set)"*, with patches **adjusted to fit the new context** by commutation. Pijul's changes apply
in any order subject to dependencies. **In both, exchange is set reconciliation: which patches do you
have that I do not.** Divergence dissolves — there is nothing to fast-forward past and no merge object to
create.

**prikk is not in that position, and it is prikk's own design that puts it outside.** Patches are sealed
into immutable, maintainer-signed **Blocks** with a lineage chain and state roots. The exchange unit is a
block lineage, not a commutative patch set — so two machines that both sealed produce two chains, and
reconciliation needs a merge block. **prikk is patch-based underneath and Git-shaped at the exchange
layer.**

**That is worth stating plainly because it inverts an easy assumption:** prikk's merge-per-sync cost is
**not** inherited from patch theory. Darcs and Pijul demonstrate that patch theory *removes* this
problem. prikk reintroduced it by sealing — for good reasons (immutability, signed publication,
verifiable history), but the cost belongs to that choice and should be attributed to it.

**The question this raises, and this RFC does not answer:** prikk's real unit is the Patch; the Block is
a publication envelope. **Could exchange operate at patch level, with sealing remaining a local act?**
That would put prikk back in Darcs and Pijul's position — and it may be incompatible with DC-78's rule
that a receiver trusts a *maintainer's seal*, since unsealed patches carry no maintainer signature at
all. **Worth investigating before any design commits to block-level exchange.**

### 3.1f A fourth option prior art suggests: **(d) SSH transport, prikk on both ends**

**Pijul's SSH support is not a custom wire protocol** — *"A working Pijul needs to be installed on the
remote machine"*. The transport is SSH; the "protocol" is two Pijul processes over stdin/stdout.

**Applied to prikk, that answers most of §3.1a's objections to (a) without becoming (a):**

- **No TLS stack, no async runtime** — SSH is the transport and it is already on the machine.
- **No authentication model to invent** — SSH keys and the remote account are the answer, and they are
  the answer operators already administer.
- **No listening service and no new attack surface in prikk** — the remote side is the same binary
  invoked over an authenticated channel, not a server exposed to the internet.
- **`push`/`pull` ergonomics**, which is §3.1c's dominant cost and the adoption objection in one.

**What it does not remove:** an over-the-wire compatibility obligation between versions (§3.1a reason 1)
still applies, because the two processes must agree — though it is bounded to what the two binaries
exchange rather than to a published protocol other implementations depend on. **And HTTP would remain
pull-only** for the same reason it is in Pijul: serving is easy, accepting is not.

**(d) composes with (c) rather than replacing it** — the artifact and basis are what travels; SSH just
carries them and runs the command at the far end.

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

## 5.1 Test, plan and process discipline — binding on this work

**Directed by the project owner 2026-08-19:** *"we should take careful steps of tests, plan and process
of them. Networking and data sharing should be verified especially carefully on both function and
security."*

**Recorded as binding before any design**, because a test regime assembled during implementation tests
what was built rather than what was required.

### 5.1.1 What sync adds that prikk's existing regime does not already cover

prikk's quality regime is strong and **most of it transfers**: deterministic tests, gates observed
failing before the fix (RFC 111, RFC 114), negative controls on every guarantee, property tests
(`proptest_decode_bundle.rs`, `proptest_decoders.rs`, `proptest_round_trip.rs`), crash injection
(`failpoints.rs`'s `Point` and `TestBarrier`), and three-platform CI.

**Four things are genuinely new:**

1. **Adversarial input becomes the normal case, not an edge case.** Today one file path takes untrusted
   bytes and DC-86 bounds it. Under sync, everything received is attacker-controlled by default.
2. **Two-party state.** Every meaningful test needs two repositories and an assertion about the *pair*.
   `dc78_bundle_exchange.rs` and `dc85_merge_from_received_ref.rs` are the precedent to build on rather
   than start beside.
3. **Partial failure mid-transfer.** An interrupted exchange must leave the receiver sound. This is
   crash-injection territory and prikk already has the machinery.
4. **A lying peer, not merely malformed bytes.** Fuzzing finds parse defects; it does not find a peer
   that is syntactically perfect and semantically dishonest.

### 5.1.2 The security properties to test, stated as refusals

**Each is a specific expected refusal, not a general aspiration** — and each is derived from something
this project has already learned the hard way:

- **A hostile artifact must not leave the receiver in an unrecoverable state.** DC-53 Stage 2 already
  proved this is reachable: a refused bundle leaked one author-key entry into a container with no prune,
  no compaction and no repair. **Whole-import refusal, never partial.**
- **Trust must not expand on receipt.** A sender must not be able to cause the receiver to adopt a
  maintainer key. DC-78's rule is that the receiver decides; the test is that a crafted artifact cannot
  make it decide otherwise.
- **TOFU pinning must hold across transport.** A conflicting public key for a known `key_id` is refused,
  whatever route it arrives by (DC-53 Stage 2, criterion 5).
- **Missing prerequisites refuse the whole exchange.** A basis-scoped artifact whose dependencies the
  receiver lacks must not partially apply (§3.1c problem 3).
- **Resource bounds hold under adversarial input**, including declared counts, declared sizes, and
  deeply nested or pathological patch DAGs. DC-86's *"a declared count over the limit must not cost more
  than reading one u64 to reject"* is the standard.
- **Replay of an old artifact is inert**, not a downgrade.
- **A patch whose AUTHOR signature does not verify against transported material fails**, and one with no
  material reads `Unverifiable` — never `Sound` (DC-53's D3 rows, reached through the transport path).

### 5.1.3 Process

- **A written threat model precedes the design**, not the implementation review. The list above is its
  starting point, not its conclusion.
- **Adversarial fixtures are committed as bytes**, following RFC 114's approach — a hostile artifact
  reconstructed at test time from today's encoder proves the parser accepts a shape, not that the shape
  occurs.
- **Every refusal above gets a negative control**: disable the check, watch the test fail, restore. A
  refusal nobody has seen fire is not evidence.
- **Staged, with report-before-implement at each stage**, as every increment in this project now runs.
- **If a transport with a listening surface is ever chosen** (§3's option (a)), it does not ship on the
  same evidence as a file path. That is a different risk class and needs its own review, stated now so
  the bar cannot be lowered later by momentum.

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
