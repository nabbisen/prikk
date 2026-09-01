# Release Policy Data

This directory contains machine-readable contracts and review fixtures for the release policy in
[`docs/src/reference/release-compatibility.md`](../docs/src/reference/release-compatibility.md).
They do not claim that a hosted-service action or signer bootstrap has passed.

Run the tracked policy audit from the repository root:

```console
cargo run --locked -p prikk-release-policy -- check
```

The Rust workspace tool strictly parses the actual authority file, derives the outcome of every
release-evidence fixture, and fails when a computed outcome differs from its `expected` value. (RFC
119 track A parked the signer/challenge suites and track B removed the release-state suite outright
-- see `docs/src/reference/release-compatibility.md`'s Core Caveats.) Its focused Draft 2020-12
evaluator implements every schema
keyword used by the tracked release-evidence schema, fails closed on an unknown keyword, and uses strict
JSON value equality. It asserts canonical UTC `date-time` values rather than treating `format` as
annotation-only. Every policy JSON entry path rejects duplicate object names at any nesting depth before
schema or semantic validation. Raw parser boundary vectors cover valid unique names and duplicate names
at top level, through escaped-equivalent member spelling, at nested level, inside an array object, and in
schema-, evidence-, and fixture-shaped inputs. A malformed-JSON control keeps syntax errors distinct
from duplicate-name errors. Separate semantic checks cover relations JSON Schema cannot express,
including exact signer-proof coverage and append-only snapshot history. Running the command does not
create bytecode in the worktree.

RFC 119 track B removed the Python implementation this tool once migrated from, along with
`differential-check` (the command that compared the two) and `release/observe-policy.py`,
`release/check-policy.py`, and the `release/policy_check/` package it invoked. The Rust command
above is the sole authoritative policy check; `release/oracle/python-observations-v1.json` remains
as a frozen historical baseline it is still cross-checked against, not a live Python re-run.

## Signer Authority

The tracked authority file is [`release-signers.toml`](../release-signers.toml). Version 1 has exactly
these fields:

- `schema_version = 1`
- `authorized_primary_fingerprints = ["..."]`

Unknown fields and unsupported schema versions are invalid. Fingerprints are full uppercase OpenPGP
primary fingerprints containing either 40 or 64 hexadecimal characters. The array is strictly sorted,
contains no duplicates, and may contain multiple fingerprints.

The committed array is empty. It authorizes no release signer and therefore blocks every official
release. Creating this empty policy file is not signer bootstrap. The first fingerprint must be added by
the separately reviewed initial-bootstrap transaction defined in DC-35, including proof of possession,
two accountable reviewers, public incident/governance evidence, the 72-hour hold, and explicit lift.

Private keys, secret key material, and sensitive recovery details never belong in this file or its
review evidence.

## Signer Transactions

Transaction effect is derived from normalized old/new fingerprint sets before checking the declared
type:

| Effect | Set relation | Proof state |
|---|---|---|
| Bootstrap | old empty; new non-empty | `verified` |
| Addition | new adds fingerprints and removes none | `verified` |
| Replacement | new adds and removes fingerprints | `verified` |
| Removal-only | new is a strict subset | `not-applicable` with reason |
| Classification-only | old and new sets equal | `not-applicable` with reason |

Authority proof and later release-tag verification are separate evidence fields because they may refer
to different signers. A transaction introducing multiple fingerprints records a separate proof result
for every introduced primary fingerprint. The executed positive and forbidden transaction rows are in
[`fixtures/signer-governance-cases.json`](fixtures/signer-governance-cases.json).

A development-stage authority row references one canonical governance evidence document containing the
transaction type, old/new signer sets and authority blobs, proofs, approvals, record, and hold state. It
does not combine a signer transaction with a separate unrelated hold transaction.

### Proof-of-possession challenge v1

The signer creates an OpenPGP detached signature over these exact ASCII bytes, including the final LF:

```text
prikk-release-signer-proof-v1
repository=https://github.com/prikk-vcs/prikk
primary_fingerprint=<40-or-64-uppercase-hex>
role=official-release
authority_revision=<40-or-64-lowercase-git-object-id>
nonce=<64-lowercase-hex>
issued_at=<YYYY-MM-DDTHH:MM:SSZ>
expires_at=<YYYY-MM-DDTHH:MM:SSZ>
```

There are no extra fields, spaces, blank lines, alternate newline encodings, or missing final LF. The
nonce is 32 random bytes represented as lowercase hex and is not secret. `expires_at` must be later than
`issued_at` and no more than 24 hours later. Verification rejects an issue time more than five minutes
in the future and rejects at or after the exclusive expiry instant. `authority_revision` identifies the
immutable candidate authority-change commit under review; the resulting branch commit and review record
map back to it.

The verifier extracts the full primary fingerprint, checks it against `primary_fingerprint`, and records
the exact verifier result. An excluded local review-request file is never challenge authority.
Golden exact-byte, freshness, signer-binding, and malformed-input cases are in
[`fixtures/signer-challenge-cases.json`](fixtures/signer-challenge-cases.json).

### Protected-branch equivalent

If observed protected-branch review controls are unavailable, an architect-reviewed equivalent must
record the immutable authority revision, both accountable natural-person approvals, an observed no-
bypass review path or declared administrator-override incident, and the resulting branch commit. Missing
evidence blocks release.

## Release State

RFC 119 track B removed the release-state audit outright (NEVER, not parked): it was the
three-authority release-lane state machine, superseded 2026-08-24 by a proposal-authorize-execute
procedure that matches how releases are actually cut. No release-state check runs today.

## Distribution Evidence

[`schemas/release-evidence-v1.schema.json`](schemas/release-evidence-v1.schema.json) defines the strict
structure of append-only release evidence snapshots. The schema checks required/unknown fields and
basic value grammar. A predecessor digest covers the exact observed bytes of the immutable prior JSON
asset, including whitespace, key order, and final newline. The parsed snapshot must equal the JSON value
decoded from those same bytes; unrelated bytes cannot certify it. Parsed/re-serialized JSON is not
release identity.
Cross-snapshot sequence, predecessor digest, immutable identity, normalized signer-
set difference, cumulative-attempt, checksum equality, and completion rules are enforced separately by
the policy audit because JSON Schema alone cannot establish them. The positive and negative schema and
sequence corpus is [`fixtures/release-evidence-cases.json`](fixtures/release-evidence-cases.json).

The `release-evidence-*.json` files under `fixtures/` are synthetic base documents for pending, partial,
complete, superseded, and governance-hold corpus rows. Their zero Git ids and 0.18.0 values are
placeholders, not release evidence or a release claim. Active governance may be recorded with no end or
lift. Later snapshots may fill classification, hold end, and lift fields after the minimum interval;
once recorded, those fields are immutable, and an active hold always blocks `complete`.

Tag verification has status-independent coherence rules. `not-observed` carries no detail. `verified`
carries the signer fingerprint, authority path/blob, and verifier result. `failed` carries the authority
path/blob and verifier result, while its signer fingerprint may be absent when extraction was ambiguous.
Once observed, status and recorded details are immutable. Every successor snapshot strictly appends at
least one operation attempt while preserving the complete prior attempt prefix.

No release evidence file exists yet. Absence means distribution is `pending`, never `complete`.
