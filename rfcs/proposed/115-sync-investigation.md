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

**My assessment, offered as a lean and not a ruling: start from (b) and let (a) be a later transport
that reuses it.** If the artifact is right, a protocol is a delivery mechanism over it; if the artifact
is wrong, a protocol will encode the wrongness in a wire format that then needs its own migration path.
**The owner should settle this before any design**, because nearly every subsequent decision follows
from it.

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
