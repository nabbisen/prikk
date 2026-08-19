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
