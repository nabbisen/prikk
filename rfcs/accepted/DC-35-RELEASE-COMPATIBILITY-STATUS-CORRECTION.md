# RFC (accepted) - DC-35 Release Compatibility and Status Correction

**Status.** Accepted after architect design re-review v2 on 2026-07-15; documentation/policy
implementation pending.
**Target milestone.** M1 - required before the 0.18.0 release candidate.
**Tracks.** TASK-13 and architect review N3.
**Touches.** Release/compatibility reference, implementation-status correction, release-signer
fingerprint authority, README/ROADMAP links, and release-state bookkeeping. This increment is policy
and documentation only; required manifest changes occur in the later release-candidate increment.

## Problem

Release rules and compatibility limits are distributed across README, ROADMAP, release reviews, Cargo
metadata, and current-state references. The implementation status also contradicts the released public
merge surface by omitting `merge-plan` and describing public merge evidence as absent.

Past release reviews repeatedly found a second failure mode: a candidate, finalization commit, tag,
and published release can each describe a different state. Post-release housekeeping then leaves a
window where the published tag or archive permanently contains stale README, changelog, roadmap, RFC,
or version claims. The release process needs one state matrix and one finalization transaction rather
than informal cleanup after publication.

## Design

Add `docs/src/reference/release-compatibility.md` as the public current-state policy. It must document:

- pre-1.0 semantic-version expectations and the absence of stable Rust API, CLI-output, object-schema,
  or repository-format guarantees unless a later RFC says otherwise;
- unprefixed Git tags such as `0.18.0`, while release archives use `prikk-v0.18.0.tar.gz`;
- the design, implementation review, release-candidate review, final release-state flip, tag, package,
  and publish sequence;
- the requirement that README, CHANGELOG, ROADMAP, RFC status/location, implementation status,
  workspace version, lockfile, and relevant mdBook pages already describe the release at publication;
- immutable release assets: a bad published artifact is superseded, never replaced under the same name;
- current manual gates and the distinction between observed evidence and policy.

Correct the merge-surface statements in `rfcs/IMPLEMENTATION-STATUS.md` against released 0.17.7
behavior. Link the new reference from relevant public and maintainer documentation without turning the
README into an internal development log.

### Compatibility contract

The reference must distinguish these surfaces explicitly:

- Cargo crate source APIs and feature sets;
- command names, arguments, exit behavior, and human-readable output;
- canonical object schemas, signature preimages, and ObjectIds;
- repository format, on-disk layout, and migration behavior;
- release archives and published documentation.

Before 1.0, an explicitly documented minor release may break Cargo APIs/features or CLI commands,
arguments, exits, and human output. Repository-format changes additionally require an accepted
governing RFC that assigns a new format/schema as applicable and defines directional read, write,
migration, and refusal behavior. Identity-bearing changes are not authorized by SemVer or release
notes: a signature preimage, ObjectId, canonical schema, or domain change requires an accepted RFC, a
new explicit version/domain, refusal or migration behavior, and literal compatibility vectors. Existing
identity versions are never silently reinterpreted.

A patch release must not intentionally break a documented surface. An unavoidable correctness or
security break uses a minor release unless a committed emergency exception is accepted by the
maintainer and architect before tagging. The exception must identify the affected surface, governing
RFC/schema authority, rejection or migration behavior, evidence, and user action; it cannot waive the
new-version/domain rule for identity-bearing bytes.

All workspace crates use one selected version and every internal registry dependency uses exact
`=X.Y.Z` resolution for that release. Lockstep means exact resolution, not merely inheriting one package
version in the workspace. No support window, LTS line, or compatibility promise is inferred from
continued readability of a historical format.

### Release-state matrix

The workspace version is a source compatibility line, not sufficient release identity. Outside the
exact release tag, a build is a development build even when its Cargo version equals the latest
release. An untagged build must not be distributed or described as a release, and any shared
development artifact must expose its exact commit and non-release status in basic build/source
metadata. Release identity is the authorized signed annotated tag object, its peeled commit, and the
digest of each distributed payload artifact.

The page must define these states:

| State | Workspace source line | Latest released | Current candidate | Changelog | RFC location | Git identity |
|---|---|---|---|---|---|---|
| Development | last release | last release | none | no unreviewed release claim | proposed/accepted | HEAD commit provenance; no release tag at HEAD |
| Release candidate | target version | last release | target version | target entry marked release candidate | accepted | reviewed RC commit; no target tag |
| Released | target version | target version | none | final target entry | shipped RFCs in done | signed annotated target tag peels to finalization commit |

An accepted RFC remains under `accepted/` during implementation and release-candidate review. It moves
to `done/` only in the finalization commit that becomes the release tag target. A rejected or abandoned
candidate returns all candidate fields to development state in a reviewed commit; it is not described
as released and creates no target tag or release asset.

The implementation must maintain an explicit field-to-asset inventory. At minimum, workspace source
line maps to root Cargo metadata, `Cargo.lock`, normalized package manifests, and `prikk --version`;
latest release maps to README and implementation status; candidate maps to ROADMAP and implementation
status; change state maps to CHANGELOG; RFC state maps to RFC status/location, inbound links, and
`rfcs/README.md`; release identity maps to the Git tag and release evidence snapshots. An unregistered
duplicate release claim is an audit failure.

The table-driven audit must test every valid row and reject mixed rows, including: candidate and latest
release both naming the target; released/latest/done claims without the exact tag; a tag that peels to
a different commit; a tagged tree retaining candidate wording or shipped RFCs in `accepted/`; broad or
mismatched internal Cargo requirements; distribution-complete claims with missing outputs; and an
abandoned candidate retaining any target claim. The finalization row without a public tag is allowed
only as a private local transaction and must never be pushed separately.

External distribution is a separate dimension from Git release state: `pending`, `partial`, `complete`,
or `superseded`. Atomic publication of the finalization commit and tag is the release event. Absence of
a release evidence snapshot means distribution is `pending`, never complete.

### Required workflow

The policy must specify this order:

1. Obtain design and implementation acceptance and record their isolated commits.
2. Prepare one release-candidate commit: select the version; update workspace metadata, every internal
   registry requirement to exact `=X.Y.Z`, and the lockfile;
   add the candidate changelog entry; update README, ROADMAP, MILESTONES, RFC indexes/status, relevant
   mdBook pages, and implementation status; keep latest-released and RFC lifecycle claims honest.
3. Run and record the full applicable gates against that exact commit, including package inspection,
   then obtain adversarial release-candidate review.
4. After RC acceptance, prepare the finalization commit: remove candidate wording, set latest released,
   clear the active candidate, move shipped RFCs from `accepted/` to `done/`, and make every tracked
   release asset describe the final state.
5. On the clean finalization commit, rerun and record the full deterministic release gate suite,
   including package/local-registry, mdBook/link, RFC lifecycle, version/lockfile, and release-state
   checks. RC evidence is not substituted for this run.
6. Create the unprefixed signed annotated tag at that exact commit and verify its signature and peeled
   commit against the committed release-signer authority. Generate and inspect the archive/checksum
   once in a new no-clobber staging directory. Verify the tag/archive release-state row before any
   remote publication.
7. Require a successful atomic-push capability check, then publish branch and tag together with
   `git push --atomic <remote> <branch> <tag>`. If the remote does not support atomic push, abort; there
   is no branch-first, tag-first, or multi-command fallback. Verify the remote tag object and peeled
   commit. This atomic publication is the release event.
8. Publish the staged archive/checksum and workspace crates from the exact clean tagged tree. Publish
   crates in manifest-derived dependency order, waiting for registry visibility before each dependent.
   Record every external result in append-only release evidence and verify the release page and Pages
   deployment without treating either as Git release identity.

Steps 4-8 are a controlled release transaction, not post-release housekeeping. If local finalization
fails before atomic push, correct or abandon the private transaction without publishing false state.
After atomic push, the release exists even when distribution is pending or partial. Preserve every
published identity, append evidence for each attempt, and retry only missing immutable outputs or
supersede with a new version. Never retag a published version, replace an archive or evidence snapshot
under the same name, or overwrite a published crate version.

### Package and artifact authority

The internal package graph is derived from normalized manifests, not maintained only as prose. For the
current graph its publish levels are: `prikk-error` and `prikk-hash`; then `prikk-crypto` and
`prikk-object`; then `prikk-replay`; then `prikk-store`; then `prikk`. RC and finalization evidence must
inspect every normalized manifest for exact selected-version requirements. Before tagging, an isolated
local registry must be populated from the staged package archives in graph order and each packaged
crate must build against that registry without workspace path overrides. During external publication,
the same graph is used and each predecessor must be observed in the registry before a dependent is
attempted.

The source archive is exactly `prikk-vX.Y.Z.tar.gz`; its checksum asset is exactly
`prikk-vX.Y.Z.tar.gz.sha256`. The digest covers the published compressed bytes. The checksum file is one
lowercase 64-hex SHA-256 digest, two ASCII spaces, the archive basename, and LF. Archive generation uses
tracked bytes from the peeled tag commit at archive root and deterministic gzip metadata (`gzip -n` or
an equivalently reviewed command). Generation occurs once in a new staging directory that refuses
existing output names. An unpublished abandoned staging transaction may be discarded; a published
name is immutable.

### Distribution evidence

The release page carries immutable, monotonically numbered JSON evidence snapshots named
`prikk-X.Y.Z-release-evidence-NNN.json`. The highest valid sequence is authoritative; snapshots are
appended, never replaced. Absence means `pending`. Every snapshot is keyed by version and records:

- the tag name, tag-object id, tag-signature verification result, and peeled commit id;
- archive and checksum names, compressed-byte SHA-256, checksum grammar result, and archive-root result;
- normalized crate names/versions/exact internal requirements, graph order, staged/registry/fetched
  checksums, equality result, and per-crate publication and registry-visibility status;
- release-page status and, separately, Pages workflow/deployed-revision status;
- overall `pending`, `partial`, `complete`, or `superseded` status, attempt time, and prior snapshot.

The snapshot grammar must reject missing required fields, unknown fields, invalid state transitions, a
`complete` state with any missing/failed output, and identity changes from an earlier snapshot. A failed
attachment leaves the normative state `pending` or at the last snapshot; recovery appends the next
snapshot. Partial publication never changes the Git release identity and never permits replacing an
existing output.

### Release-signer authority

DC-35 implementation must add a tracked root `release-signers.toml` containing a schema version and the
full uppercase OpenPGP fingerprints authorized to sign releases. The project owner must explicitly
confirm each fingerprint before it is committed; local Git configuration or an imported key is not
authority. No private or secret key material belongs in the repository, review request, or evidence.

Tag creation must explicitly select an authorized full fingerprint. Verification must extract the full
primary signer fingerprint from verifier output, compare it exactly with the allowlist in the peeled
commit, and fail before atomic push on missing authority, ambiguous output, or mismatch. The evidence
snapshot records the fingerprint, authorization path, authority-file blob id, and observed verification
result. Tags created under this policy retain their own authority file through their peeled commits.
Tags through 0.17.7 predate this authority and must not be reported as passing its signer audit. Key
rotation/revocation policy, attestations, SBOMs, and broader release-key lifecycle remain DC-43 scope.

### Crate-byte identity and completion

For every crate, release evidence records the staged `.crate` SHA-256, the registry-index checksum, the
SHA-256 of bytes fetched after registry visibility, and equality of all three values. A dependent is not
published until each predecessor is visible and its fetched bytes match the registry checksum and
reviewed staged package. A mismatch is `partial`, preserves the published crate identity, blocks all
dependents and `complete`, and requires incident evidence plus a superseding version rather than an
overwrite.

`complete` requires all of these outputs: the fixed source archive and checksum names are attached and
their bytes match recorded digests; every expected crate is registry-visible with equal staged,
registry, and fetched checksums; the release page is published rather than draft; Pages is deployed at
the peeled commit, or a reasoned pre-publication review explicitly records Pages as inapplicable. A
failed or delayed configured deployment remains `pending` or `partial` and cannot be waived afterward
to obtain `complete`. Source archives and `.crate` files are payload artifacts. Checksum files and
release evidence snapshots are integrity/status metadata and are not included in the self-referential
payload-digest rule, although their own names and observed bytes remain immutable once attached.

Evidence JSON uses an explicit schema version, rejects missing required fields and unknown fields, and
requires sequences to start at `001` and remain contiguous with no duplicates or gaps. Each snapshot
after `001` names the prior snapshot and its observed SHA-256; malformed or mismatched predecessor links
are invalid. Allowed overall transitions are absent/pending to `pending`, `partial`, `complete`, or
`superseded`; `partial` to `partial`, `complete`, or `superseded`; `complete` to `superseded`; and
`superseded` is terminal.

Snapshots carry a cumulative append-only attempt list. A later snapshot must retain every prior attempt
in order and add attachment, upload, registry, release-page, and Pages attempts since its predecessor,
including failed evidence-attachment attempts. If no snapshot can be attached, distribution remains
`pending` or at the last valid snapshot. A later `complete` snapshot may not omit an intervening failure.

### Gate and evidence boundary

The design must require, where applicable:

- clean worktree and exact commit/tag identity checks;
- `cargo fmt --all -- --check`;
- `cargo check --workspace --all-targets`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo test --workspace` and `cargo build --workspace`;
- `mdbook build docs` and `git diff --check`;
- package contents/metadata checks and publish dry runs supported by the installed Cargo;
- normalized exact internal dependency checks and isolated-registry package builds;
- authorized signer fingerprint and authority-file identity checks;
- staged/registry/fetched crate checksum equality;
- version, lockfile, changelog, roadmap, RFC lifecycle, tag, archive-root, checksum, evidence-snapshot
  sequence/attempt history, completion output set, and valid/forbidden release-state consistency.

The complete deterministic suite runs on the clean finalization commit before tagging. After local tag
creation, identity/archive checks prove that the signed tag peels to that tested commit. After atomic
push, remote identity checks prove that the published tag object and peeled commit are unchanged.

A policy list is not passing evidence. Each review or release record must say which commands were
actually observed, which were unavailable or inapplicable, and the host/filesystem limits. `cargo
audit`, `cargo deny`, crash-reboot testing, registry publication, GitHub release publication, and Pages
deployment must not be reported as passed unless they were executed and observed for that release.

### Documentation placement

The implementation must add `docs/src/reference/release-compatibility.md`, link it from the mdBook
summary and relevant format/recovery references, and add one concise README link. Detailed workflow,
gate, and lifecycle narration belongs in the reference page, not the README. `CHANGELOG.md` remains
released change history, `ROADMAP.md` remains the current schedule, and `rfcs/IMPLEMENTATION-STATUS.md`
remains a current-state snapshot rather than a second changelog.

## Non-goals

- No version bump, tag, package, publish, CI, executable-code, repository-object-schema, or CLI change.
- No compatibility promise, support window, LTS policy, migration tool, or 1.0 commitment.
- No private release-key storage, key-rotation/revocation policy, SBOM, or provenance attestation; those
  remain DC-43 scope.
- No claim that a listed gate passed unless observed for the release under review.

## Dependencies and gates

DC-35 may be implemented after design review independently of storage fixes, but it is held for the
single 0.18.0 corrective release. The final page must reflect DC-34's format and compatibility rulings,
DC-40's accepted format-1/format-2 transition, RFC-000's accepted-to-done boundary, and the project
owner's unprefixed signed-tag convention. `mdbook build docs`, link/status consistency, normalized
package/local-registry evidence, and a table-driven positive/forbidden release-state audit are required
implementation-review evidence.

## Acceptance criteria

The new reference is reviewed and navigable; the N3 contradictions are corrected; compatibility rules
cannot override identity authority; development build/source metadata, the three Git states, forbidden mixed rows,
and external distribution states are unambiguous; the full suite runs on the finalization commit; atomic
push has no non-atomic fallback; the tag signer is bound to committed fingerprint authority; exact Cargo
requirements, package order, staged/registry/fetched crate equality, immutable asset bytes, exact
completion output, and append-only partial-publication evidence are defined; no published asset requires
later bookkeeping to become truthful; and all compatibility and gate limitations remain explicit.
