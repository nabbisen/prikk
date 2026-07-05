# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
milestone/status detail is in `rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- **Post-DC-18 crate-boundary design.** Next design work should define the replay/lifecycle extraction
  boundary below `prikk-store` before Prikk exposes a broader M2+ production merge/conflict surface.
  Patch algebra remains internal while rollback refs, authorization, branch switching, key lifecycle,
  and sync stay behind their dedicated plans.

## Last Released Increment

- **DC-18 - patch algebra commutation and confluence contract (released as 0.11.0).** Internal
  commutation now requires classifier independence plus replay-both-orders proof, and flat
  two-sequence confluence requires individual replay-validity, commuting cross-pairs, composed replay,
  and final lifecycle-state equality. Required sealed candidate evidence failures, including
  replacement blob evidence, remain outer evidence errors and are not hidden by algebraic `Unknown`.
  No CLI, schema, merge execution, persisted witness/proof, public confluence API, or production merge
  surface was added.

- **DC-17 - patch algebra evidence contract (released as 0.10.0).** Internal pair classification now
  uses a scoped evidence contract and store-backed resolver boundary so required sealed evidence
  failures are distinguishable from ordinary unsupported algebra, while optional unsealed candidate
  evidence still fails closed as `Unknown`. No CLI, schema, merge execution, persisted witness, or
  production confluence surface was added.

- **DC-16 - patch algebra foundation (released as 0.9.0).** Internal pair classification now covers
  `Independent`, `OrderedDependency`, `Conflict`, and `Unknown`, with baseline preimage validation,
  structured path effects, fail-closed insufficient-evidence handling, and oracle-backed vectors. No CLI,
  schema, merge execution, persisted witness, or production confluence surface was added.

- **DC-15 - active-session integrity and verification hardening (released as 0.8.0).** `verify` and
  `doctor` report active-WAL metadata integrity explicitly, rollback-draft append re-checks target tip
  freshness under the active lock, ref publication validates `heads/*` at the lower-level boundary, and
  signature key-id validation is shared across AUTHOR, MAINTAINER, and trust-policy paths.

- **DC-14 - arbitrary-span text direct inverse and rollback exposure (released as 0.7.0).** The existing
  inverse/rollback surfaces support arbitrary-span `EditText` by recomputing direct inverse identity
  against the post-forward text.

- **DC-13 - non-default ref genesis (released as 0.6.0).** First-commit genesis on explicit
  non-default branch refs is implemented with branch-ref validation, active-WAL ref ownership, and
  `seal --ref` publication rules.

- **DC-12 - arbitrary-span text edits (released as 0.5.0).** Worktree text edits are authored and
  replayed as deterministic arbitrary spans while keeping inverse/rollback, commutation, confluence,
  and conflict witnesses deferred.

- **DC-11 - publication signing and minimal trust store (released as 0.4.0).** Publication objects carry
  real role-bound Ed25519 MAINTAINER signatures verified against a minimal repository-local trust
  policy.

- **DC-10 - rollback-draft identity and AUTHOR signing (released as 0.3.0).** Rollback-draft identity is
  `PatchPurpose::RollbackDraft`, not a reserved AUTHOR key id, and rollback drafts carry real role-bound
  Ed25519 AUTHOR signatures. This closed the broad claim for AUTHOR-role Patch signatures produced by
  production commands.

## Next Increments

1. **M2+ patch algebra**: commutation, confluence, conflict witnesses, and merge evidence.
2. Branch copy/fork, branch switching, tags/remotes, rollback refs, conflict/inverse evidence,
   rollback authorization, audit/plugin, key lifecycle, and sync remain gated by
   their dedicated plans and FDDs.

Final feature scope remains governed by the FDDs and RFCs.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
